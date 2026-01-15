#!/bin/bash
# Cloudflare Workers usage analysis for crabigator-api
# Reads OAuth token from wrangler config, queries GraphQL API

set -e

ACCOUNT_ID="20af5d7e521c82550b1ffe8705e981c5"
SCRIPT_NAME="crabigator-api"

# Find wrangler OAuth token
TOKEN_PATHS=(
    "$HOME/Library/Preferences/.wrangler/config/default.toml"
    "$HOME/.config/.wrangler/config/default.toml"
    "$HOME/.wrangler/config/default.toml"
)

API_TOKEN=""
for path in "${TOKEN_PATHS[@]}"; do
    if [ -f "$path" ]; then
        API_TOKEN=$(grep "oauth_token" "$path" | cut -d'"' -f2)
        break
    fi
done

if [ -z "$API_TOKEN" ]; then
    echo "Error: Could not find wrangler OAuth token"
    echo "Run 'npx wrangler login' to authenticate"
    exit 1
fi

# Calculate date range (last 30 days)
END_DATE=$(date -u +%Y-%m-%dT00:00:00Z)
START_DATE=$(date -u -v-30d +%Y-%m-%dT00:00:00Z 2>/dev/null || date -u -d "30 days ago" +%Y-%m-%dT00:00:00Z)

# Query worker analytics with usage model
WORKER_DATA=$(curl -s "https://api.cloudflare.com/client/v4/graphql" \
  -H "Authorization: Bearer $API_TOKEN" \
  -H "Content-Type: application/json" \
  --data '{
    "query": "query { viewer { accounts(filter: {accountTag: \"'"$ACCOUNT_ID"'\"}) { workersInvocationsAdaptive(limit: 100, filter: {scriptName: \"'"$SCRIPT_NAME"'\", datetime_geq: \"'"$START_DATE"'\", datetime_leq: \"'"$END_DATE"'\"}) { sum { requests subrequests errors } dimensions { date usageModel } } durableObjectsInvocationsAdaptiveGroups(limit: 100, filter: {datetime_geq: \"'"$START_DATE"'\", datetime_leq: \"'"$END_DATE"'\"}) { sum { requests } dimensions { date } } } } }"
  }')

# Check for errors
if echo "$WORKER_DATA" | jq -e '.errors != null and .errors != []' > /dev/null 2>&1; then
    echo "API Error:"
    echo "$WORKER_DATA" | jq '.errors'
    exit 1
fi

# Detect plan type from usage model (standard = paid, bundled = free)
USAGE_MODEL=$(echo "$WORKER_DATA" | jq -r '.data.viewer.accounts[0].workersInvocationsAdaptive[0].dimensions.usageModel // "bundled"')
if [ "$USAGE_MODEL" = "standard" ]; then
    PLAN_TYPE="Workers Paid (\$5/month)"
    # Paid plan limits
    WORKER_MONTHLY_LIMIT=10000000  # 10M included
    DO_MONTHLY_LIMIT=1000000       # 1M included (then $0.15/M)
else
    PLAN_TYPE="Free Tier"
    # Free tier limits
    WORKER_MONTHLY_LIMIT=3000000   # 100K/day * 30
    DO_MONTHLY_LIMIT=1000000       # 1M/month
fi

# Parse metrics
WORKER_TOTAL=$(echo "$WORKER_DATA" | jq '[.data.viewer.accounts[0].workersInvocationsAdaptive[].sum.requests] | add // 0')
WORKER_ERRORS=$(echo "$WORKER_DATA" | jq '[.data.viewer.accounts[0].workersInvocationsAdaptive[].sum.errors] | add // 0')
WORKER_DAYS=$(echo "$WORKER_DATA" | jq '.data.viewer.accounts[0].workersInvocationsAdaptive | length')
DO_TOTAL=$(echo "$WORKER_DATA" | jq '[.data.viewer.accounts[0].durableObjectsInvocationsAdaptiveGroups[].sum.requests] | add // 0')
DO_DAYS=$(echo "$WORKER_DATA" | jq '.data.viewer.accounts[0].durableObjectsInvocationsAdaptiveGroups | length')

# Calculate averages
WORKER_AVG=$((WORKER_TOTAL / (WORKER_DAYS > 0 ? WORKER_DAYS : 1)))
DO_AVG=$((DO_TOTAL / (DO_DAYS > 0 ? DO_DAYS : 1)))

# Get D1 info (parse box-drawing table format)
D1_INFO=$(npx wrangler d1 info crabigator 2>/dev/null || true)
D1_SIZE=$(echo "$D1_INFO" | grep "database_size" | sed 's/.*│[^│]*│ *\([^│]*\) *│.*/\1/' | xargs || echo "unknown")
D1_READS=$(echo "$D1_INFO" | grep "read_queries_24h" | sed 's/.*│ *\([0-9,]*\) *│.*/\1/' | tr -d ',' || echo "0")
D1_WRITES=$(echo "$D1_INFO" | grep "write_queries_24h" | sed 's/.*│ *\([0-9,]*\) *│.*/\1/' | tr -d ',' || echo "0")

# Display results
echo "===== CRABIGATOR CLOUDFLARE USAGE ====="
echo ""
echo "Plan: $PLAN_TYPE"
echo "Period: Last $WORKER_DAYS days with activity"
echo ""
echo "📊 USAGE SUMMARY:"
echo "─────────────────────────────────────────"
printf "Worker Requests:     %'d total (%'d/day avg)\n" "$WORKER_TOTAL" "$WORKER_AVG"
printf "Worker Errors:       %'d (%.3f%%)\n" "$WORKER_ERRORS" "$(echo "scale=3; $WORKER_ERRORS * 100 / ($WORKER_TOTAL + 1)" | bc)"
printf "DO Requests:         %'d total (%'d/day avg)\n" "$DO_TOTAL" "$DO_AVG"
echo "D1 Database:         $D1_SIZE"
echo "D1 Queries (24h):    $D1_READS reads, $D1_WRITES writes"
echo ""

# Monthly projection
MONTHLY_WORKER=$((WORKER_AVG * 30))
MONTHLY_DO=$((DO_AVG * 30))

echo "📈 PLAN USAGE:"
echo "─────────────────────────────────────────"
WORKER_PCT=$(echo "scale=1; $MONTHLY_WORKER * 100 / $WORKER_MONTHLY_LIMIT" | bc)
DO_PCT=$(echo "scale=1; $MONTHLY_DO * 100 / $DO_MONTHLY_LIMIT" | bc)
if [ "$USAGE_MODEL" = "standard" ]; then
    printf "Worker Requests:     %'d / 10M/month = %s%%\n" "$MONTHLY_WORKER" "$WORKER_PCT"
    printf "DO Requests:         %'d / 1M/month = %s%%\n" "$MONTHLY_DO" "$DO_PCT"
else
    printf "Worker Requests:     %'d / 3M/month = %s%%\n" "$MONTHLY_WORKER" "$WORKER_PCT"
    printf "DO Requests:         %'d / 1M/month = %s%%\n" "$MONTHLY_DO" "$DO_PCT"
fi
echo ""

# Scaling estimate based on plan
if [ "$USAGE_MODEL" = "standard" ]; then
    # Paid plan - much more headroom
    WORKER_HEADROOM=$(echo "scale=1; $WORKER_MONTHLY_LIMIT / ($MONTHLY_WORKER + 1)" | bc)
    DO_HEADROOM=$(echo "scale=1; $DO_MONTHLY_LIMIT / ($MONTHLY_DO + 1)" | bc)
    # Use the smaller headroom as bottleneck
    if [ "$(echo "$WORKER_HEADROOM < $DO_HEADROOM" | bc)" -eq 1 ]; then
        HEADROOM=$WORKER_HEADROOM
        BOTTLENECK="Worker Requests"
    else
        HEADROOM=$DO_HEADROOM
        BOTTLENECK="Durable Objects"
    fi
    echo "🚀 SCALING CAPACITY:"
    echo "─────────────────────────────────────────"
    echo "Bottleneck:          $BOTTLENECK"
    echo "Included headroom:   ${HEADROOM}x current usage"
    EST_USERS=$((${HEADROOM%.*} > 1 ? ${HEADROOM%.*} : 1))
    echo "Est. max users:      ~$EST_USERS concurrent (within \$5/mo)"
    echo ""

    # Overage costs if applicable
    echo "💵 COST ANALYSIS:"
    echo "─────────────────────────────────────────"
    echo "Base cost:           \$5/month"
    if [ "$MONTHLY_WORKER" -gt "$WORKER_MONTHLY_LIMIT" ]; then
        OVERAGE_WORKER=$(( (MONTHLY_WORKER - WORKER_MONTHLY_LIMIT) / 1000000 ))
        WORKER_COST=$(echo "scale=2; $OVERAGE_WORKER * 0.30" | bc)
        echo "Worker overage:      +\$$WORKER_COST ($OVERAGE_WORKER M extra @ \$0.30/M)"
    fi
    if [ "$MONTHLY_DO" -gt "$DO_MONTHLY_LIMIT" ]; then
        OVERAGE_DO=$(( (MONTHLY_DO - DO_MONTHLY_LIMIT) / 1000000 ))
        DO_COST=$(echo "scale=2; $OVERAGE_DO * 0.15" | bc)
        echo "DO overage:          +\$$DO_COST ($OVERAGE_DO M extra @ \$0.15/M)"
    fi
    if [ "$MONTHLY_WORKER" -le "$WORKER_MONTHLY_LIMIT" ] && [ "$MONTHLY_DO" -le "$DO_MONTHLY_LIMIT" ]; then
        echo "Current usage:       Within included limits"
        echo "Total cost:          \$5/month"
    fi
else
    # Free tier
    HEADROOM=$(echo "scale=1; $DO_MONTHLY_LIMIT / ($MONTHLY_DO + 1)" | bc)
    echo "🚀 SCALING CAPACITY:"
    echo "─────────────────────────────────────────"
    echo "Bottleneck:          Durable Objects"
    echo "Free tier headroom:  ${HEADROOM}x current usage"
    echo "Est. max users:      ~$((${HEADROOM%.*} > 1 ? ${HEADROOM%.*} : 1)) concurrent"
    echo "With \$5/mo paid:     50-100+ users"
fi
