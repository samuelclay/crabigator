#!/bin/bash
# Estimate Crabigator's Cloudflare usage for the current billing cycle.
# GraphQL analytics can differ from the final invoice, so this report is a
# cost warning and capacity guide rather than an invoice.

set -euo pipefail

WRANGLER_CONFIG="${WRANGLER_CONFIG:-workers/crabigator-api/wrangler.jsonc}"
WRANGLER_PROFILE="${WRANGLER_PROFILE:-}"
ACCOUNT_ID="${CLOUDFLARE_ACCOUNT_ID:-$(sed -n 's/.*"account_id"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$WRANGLER_CONFIG" | head -1)}"
SCRIPT_NAME="${CLOUDFLARE_WORKER_NAME:-$(sed -n 's/.*"name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$WRANGLER_CONFIG" | head -1)}"
D1_NAME="${D1_DATABASE:-$(sed -n 's/.*"database_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$WRANGLER_CONFIG" | head -1)}"
CONFIG_BILLING_DAY=$(sed -n 's/.*"billing_cycle_day"[[:space:]]*:[[:space:]]*\([0-9][0-9]*\).*/\1/p' "$WRANGLER_CONFIG" | head -1)
BILLING_DAY="${CLOUDFLARE_BILLING_DAY:-${CONFIG_BILLING_DAY:-1}}"

if [ -z "$ACCOUNT_ID" ]; then
    echo "Error: Set CLOUDFLARE_ACCOUNT_ID or add account_id to $WRANGLER_CONFIG"
    exit 1
fi

for command in curl jq awk wrangler; do
    if ! command -v "$command" >/dev/null 2>&1; then
        echo "Error: $command is required"
        exit 1
    fi
done

API_TOKEN="${CLOUDFLARE_API_TOKEN:-}"
if [ -z "$API_TOKEN" ]; then
    PROFILE_NAME="${WRANGLER_PROFILE:-default}"
    TOKEN_PATHS=(
        "$HOME/.wrangler/config/$PROFILE_NAME.toml"
        "$HOME/Library/Preferences/.wrangler/config/$PROFILE_NAME.toml"
        "$HOME/.config/.wrangler/config/$PROFILE_NAME.toml"
    )
    for path in "${TOKEN_PATHS[@]}"; do
        if [ -f "$path" ]; then
            API_TOKEN=$(sed -n 's/^oauth_token = "\([^"]*\)"/\1/p' "$path" | head -1)
            [ -n "$API_TOKEN" ] && break
        fi
    done
fi

if [ -z "$API_TOKEN" ]; then
    echo "Error: Set CLOUDFLARE_API_TOKEN or log in with Wrangler"
    exit 1
fi

date_to_epoch() {
    if date -u -j -f '%Y-%m-%d' "$1" '+%s' >/dev/null 2>&1; then
        date -u -j -f '%Y-%m-%d' "$1" '+%s'
    else
        date -u -d "$1" '+%s'
    fi
}

TODAY_UTC="${CF_USAGE_TODAY:-$(date -u +%Y-%m-%d)}"
if [[ ! "$TODAY_UTC" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]]; then
    echo "Error: CF_USAGE_TODAY must use YYYY-MM-DD"
    exit 1
fi

YEAR=${TODAY_UTC%%-*}
MONTH_DAY=${TODAY_UTC#*-}
MONTH=${MONTH_DAY%%-*}
DAY=${TODAY_UTC##*-}
YEAR=$((10#$YEAR))
MONTH=$((10#$MONTH))
DAY=$((10#$DAY))

if [ "$DAY" -lt "$BILLING_DAY" ]; then
    MONTH=$((MONTH - 1))
    if [ "$MONTH" -eq 0 ]; then
        MONTH=12
        YEAR=$((YEAR - 1))
    fi
fi

BILLING_START=$(printf '%04d-%02d-%02d' "$YEAR" "$MONTH" "$BILLING_DAY")
NEXT_YEAR=$YEAR
NEXT_MONTH=$((MONTH + 1))
if [ "$NEXT_MONTH" -eq 13 ]; then
    NEXT_MONTH=1
    NEXT_YEAR=$((YEAR + 1))
fi
NEXT_BILLING_START=$(printf '%04d-%02d-%02d' "$NEXT_YEAR" "$NEXT_MONTH" "$BILLING_DAY")

START_EPOCH=$(date_to_epoch "$BILLING_START")
TODAY_EPOCH=$(date_to_epoch "$TODAY_UTC")
NEXT_START_EPOCH=$(date_to_epoch "$NEXT_BILLING_START")
ELAPSED_DAYS=$(((TODAY_EPOCH - START_EPOCH) / 86400 + 1))
CYCLE_DAYS=$(((NEXT_START_EPOCH - START_EPOCH) / 86400))
BILLING_END_EPOCH=$((NEXT_START_EPOCH - 86400))
if date -u -r "$BILLING_END_EPOCH" '+%Y-%m-%d' >/dev/null 2>&1; then
    BILLING_END=$(date -u -r "$BILLING_END_EPOCH" '+%Y-%m-%d')
else
    BILLING_END=$(date -u -d "@$BILLING_END_EPOCH" '+%Y-%m-%d')
fi

GRAPHQL_QUERY=$(printf '%s' '
query CrabigatorUsage($accountTag: string!, $start: Date!, $end: Date!, $scriptName: string!, $databaseId: string!) {
  viewer {
    accounts(filter: {accountTag: $accountTag}) {
      workersInvocationsAdaptive(limit: 1000, filter: {scriptName: $scriptName, date_geq: $start, date_leq: $end}) {
        sum { requests errors cpuTimeUs }
        dimensions { date usageModel }
      }
      durableObjectsInvocationsAdaptiveGroups(limit: 10000, filter: {date_geq: $start, date_leq: $end}) {
        sum { requests }
        dimensions { date namespaceId type }
      }
      durableObjectsPeriodicGroups(limit: 10000, filter: {date_geq: $start, date_leq: $end}) {
        sum { duration inboundWebsocketMsgCount }
        dimensions { date namespaceId }
      }
      d1AnalyticsAdaptiveGroups(limit: 1000, filter: {date_geq: $start, date_leq: $end, databaseId: $databaseId}) {
        sum { rowsRead rowsWritten }
        dimensions { date databaseId }
      }
    }
  }
}')

D1_DATABASE_ID="${CLOUDFLARE_D1_DATABASE_ID:-$(sed -n 's/.*"database_id"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$WRANGLER_CONFIG" | head -1)}"
if [ -z "$D1_DATABASE_ID" ]; then
    echo "Error: Set CLOUDFLARE_D1_DATABASE_ID or add database_id to $WRANGLER_CONFIG"
    exit 1
fi

GRAPHQL_PAYLOAD=$(jq -n \
    --arg query "$GRAPHQL_QUERY" \
    --arg accountTag "$ACCOUNT_ID" \
    --arg start "$BILLING_START" \
    --arg end "$TODAY_UTC" \
    --arg scriptName "$SCRIPT_NAME" \
    --arg databaseId "$D1_DATABASE_ID" \
    '{query: $query, variables: {
        accountTag: $accountTag,
        start: $start,
        end: $end,
        scriptName: $scriptName,
        databaseId: $databaseId
    }}')

GRAPHQL_RESPONSE=$(curl --silent --show-error --fail \
    'https://api.cloudflare.com/client/v4/graphql' \
    -H "Authorization: Bearer $API_TOKEN" \
    -H 'Content-Type: application/json' \
    --data "$GRAPHQL_PAYLOAD")

if echo "$GRAPHQL_RESPONSE" | jq -e '.errors != null and .errors != []' >/dev/null; then
    echo "Cloudflare GraphQL error:"
    echo "$GRAPHQL_RESPONSE" | jq '.errors'
    exit 1
fi

NAMESPACE_RESPONSE=$(curl --silent --show-error --fail \
    "https://api.cloudflare.com/client/v4/accounts/$ACCOUNT_ID/workers/durable_objects/namespaces" \
    -H "Authorization: Bearer $API_TOKEN")

if ! echo "$NAMESPACE_RESPONSE" | jq -e '.success == true' >/dev/null; then
    echo "Could not list Durable Object namespaces:"
    echo "$NAMESPACE_RESPONSE" | jq '.errors'
    exit 1
fi

NAMESPACES=$(echo "$NAMESPACE_RESPONSE" | jq --arg script "$SCRIPT_NAME" \
    '[.result[] | select(.script == $script) | {id, class}]')
NAMESPACE_IDS=$(echo "$NAMESPACES" | jq '[.[].id]')
if [ "$(echo "$NAMESPACE_IDS" | jq 'length')" -eq 0 ]; then
    echo "Error: No Durable Object namespaces found for $SCRIPT_NAME"
    exit 1
fi

ACCOUNT_DATA=$(echo "$GRAPHQL_RESPONSE" | jq '.data.viewer.accounts[0]')
WORKER_REQUESTS=$(echo "$ACCOUNT_DATA" | jq '[.workersInvocationsAdaptive[].sum.requests] | add // 0')
WORKER_ERRORS=$(echo "$ACCOUNT_DATA" | jq '[.workersInvocationsAdaptive[].sum.errors] | add // 0')
WORKER_CPU_MS=$(echo "$ACCOUNT_DATA" | jq '[.workersInvocationsAdaptive[].sum.cpuTimeUs] | add // 0 | . / 1000')
USAGE_MODEL=$(echo "$ACCOUNT_DATA" | jq -r \
    '.workersInvocationsAdaptive[0].dimensions.usageModel // "standard"')

DO_FULL_REQUESTS=$(echo "$ACCOUNT_DATA" | jq --argjson ids "$NAMESPACE_IDS" '
    [.durableObjectsInvocationsAdaptiveGroups[]
      | select(.dimensions.namespaceId as $id | $ids | index($id))
      | select(.dimensions.type != "hibernation")
      | .sum.requests] | add // 0')
DO_HIBERNATION_MESSAGES=$(echo "$ACCOUNT_DATA" | jq --argjson ids "$NAMESPACE_IDS" '
    [.durableObjectsInvocationsAdaptiveGroups[]
      | select(.dimensions.namespaceId as $id | $ids | index($id))
      | select(.dimensions.type == "hibernation")
      | .sum.requests] | add // 0')
DO_STANDARD_INCOMING_MESSAGES=$(echo "$ACCOUNT_DATA" | jq --argjson ids "$NAMESPACE_IDS" '
    [.durableObjectsPeriodicGroups[]
      | select(.dimensions.namespaceId as $id | $ids | index($id))
      | .sum.inboundWebsocketMsgCount] | add // 0')
DO_BILLING_REQUESTS=$(awk -v full="$DO_FULL_REQUESTS" \
    -v messages="$((DO_HIBERNATION_MESSAGES + DO_STANDARD_INCOMING_MESSAGES))" \
    'BEGIN { printf "%.0f", full + int((messages + 19) / 20) }')
DO_DURATION=$(echo "$ACCOUNT_DATA" | jq --argjson ids "$NAMESPACE_IDS" '
    [.durableObjectsPeriodicGroups[]
      | select(.dimensions.namespaceId as $id | $ids | index($id))
      | .sum.duration] | add // 0')
DO_BY_CLASS=$(echo "$ACCOUNT_DATA" | jq --argjson namespaces "$NAMESPACES" '
    .durableObjectsPeriodicGroups as $groups
    | $namespaces
    | map(. as $namespace | {
        class: $namespace.class,
        duration: ([$groups[]
          | select(.dimensions.namespaceId == $namespace.id)
          | .sum.duration] | add // 0)
      })
    | sort_by(-.duration)')

D1_ROWS_READ=$(echo "$ACCOUNT_DATA" | jq '[.d1AnalyticsAdaptiveGroups[].sum.rowsRead] | add // 0')
D1_ROWS_WRITTEN=$(echo "$ACCOUNT_DATA" | jq '[.d1AnalyticsAdaptiveGroups[].sum.rowsWritten] | add // 0')

WRANGLER_ARGS=(--config "$WRANGLER_CONFIG")
if [ -n "$WRANGLER_PROFILE" ]; then
    WRANGLER_ARGS+=(--profile "$WRANGLER_PROFILE")
fi
D1_INFO=$(wrangler d1 info "$D1_NAME" "${WRANGLER_ARGS[@]}" 2>/dev/null || true)
D1_SIZE=$(echo "$D1_INFO" | awk -F '│' '$2 ~ /database_size/ {gsub(/^[[:space:]]+|[[:space:]]+$/, "", $3); print $3; exit}')
D1_SIZE=${D1_SIZE:-unknown}

project() {
    awk -v value="$1" -v elapsed="$ELAPSED_DAYS" -v days="$CYCLE_DAYS" \
        'BEGIN { printf "%.0f", value * days / elapsed }'
}

percent() {
    awk -v value="$1" -v included="$2" 'BEGIN { printf "%.1f", value * 100 / included }'
}

charge() {
    awk -v value="$1" -v included="$2" -v rate="$3" '
        BEGIN {
            over = value - included
            if (over <= 0) { printf "0.00"; exit }
            units = int(over / 1000000)
            if (over > units * 1000000) units++
            printf "%.2f", units * rate
        }'
}

PROJECTED_WORKER_REQUESTS=$(project "$WORKER_REQUESTS")
PROJECTED_WORKER_CPU_MS=$(project "$WORKER_CPU_MS")
PROJECTED_DO_REQUESTS=$(project "$DO_BILLING_REQUESTS")
PROJECTED_DO_DURATION=$(project "$DO_DURATION")
PROJECTED_D1_READS=$(project "$D1_ROWS_READ")
PROJECTED_D1_WRITES=$(project "$D1_ROWS_WRITTEN")

WORKER_REQUEST_COST=$(charge "$PROJECTED_WORKER_REQUESTS" 10000000 0.30)
WORKER_CPU_COST=$(charge "$PROJECTED_WORKER_CPU_MS" 30000000 0.02)
DO_REQUEST_COST=$(charge "$PROJECTED_DO_REQUESTS" 1000000 0.15)
DO_DURATION_COST=$(charge "$PROJECTED_DO_DURATION" 400000 12.50)
D1_READ_COST=$(charge "$PROJECTED_D1_READS" 25000000000 0.001)
D1_WRITE_COST=$(charge "$PROJECTED_D1_WRITES" 50000000 1.00)
PROJECTED_USAGE_COST=$(awk \
    -v a="$WORKER_REQUEST_COST" -v b="$WORKER_CPU_COST" \
    -v c="$DO_REQUEST_COST" -v d="$DO_DURATION_COST" \
    -v e="$D1_READ_COST" -v f="$D1_WRITE_COST" \
    'BEGIN { printf "%.2f", a + b + c + d + e + f }')
PROJECTED_TOTAL=$(awk -v usage="$PROJECTED_USAGE_COST" 'BEGIN { printf "%.2f", 5 + usage }')
ERROR_RATE=$(awk -v errors="$WORKER_ERRORS" -v requests="$WORKER_REQUESTS" \
    'BEGIN { printf "%.3f", (requests > 0 ? errors * 100 / requests : 0) }')

echo "Crabigator Cloudflare usage"
echo "Billing cycle: $BILLING_START through $BILLING_END ($ELAPSED_DAYS of $CYCLE_DAYS days reported)"
echo "Plan model: $USAGE_MODEL"
echo
echo "Current analytics"
printf "  Worker requests:          %'d (%s%% of 10M included)\n" \
    "$WORKER_REQUESTS" "$(percent "$WORKER_REQUESTS" 10000000)"
printf "  Worker CPU:               %'d ms (%s%% of 30M included)\n" \
    "$(printf '%.0f' "$WORKER_CPU_MS")" "$(percent "$WORKER_CPU_MS" 30000000)"
printf "  Worker errors:            %'d (%s%%)\n" "$WORKER_ERRORS" "$ERROR_RATE"
printf "  DO billing requests:      %'d estimated (%s%% of 1M included)\n" \
    "$DO_BILLING_REQUESTS" "$(percent "$DO_BILLING_REQUESTS" 1000000)"
printf "  DO compute duration:      %'.0f GB-s (%s%% of 400K included)\n" \
    "$DO_DURATION" "$(percent "$DO_DURATION" 400000)"
echo "$DO_BY_CLASS" | jq -r '.[] | "    \(.class): \(.duration | round) GB-s"'
printf "  D1 rows read:             %'d (%s%% of 25B included)\n" \
    "$D1_ROWS_READ" "$(percent "$D1_ROWS_READ" 25000000000)"
printf "  D1 rows written:          %'d (%s%% of 50M included)\n" \
    "$D1_ROWS_WRITTEN" "$(percent "$D1_ROWS_WRITTEN" 50000000)"
echo "  D1 database size:         $D1_SIZE"
echo
echo "Full-cycle projection at the average so far"
printf "  Worker requests:          %'d\n" "$PROJECTED_WORKER_REQUESTS"
printf "  Worker CPU:               %'d ms\n" "$PROJECTED_WORKER_CPU_MS"
printf "  DO billing requests:      %'d estimated\n" "$PROJECTED_DO_REQUESTS"
printf "  DO compute duration:      %'d GB-s\n" "$PROJECTED_DO_DURATION"
printf "  D1 rows read:             %'d\n" "$PROJECTED_D1_READS"
printf "  D1 rows written:          %'d\n" "$PROJECTED_D1_WRITES"
echo
echo "Projected cost after Cloudflare's million-unit rounding"
echo "  Workers base plan:        \$5.00"
echo "  Worker request overage:   \$$WORKER_REQUEST_COST"
echo "  Worker CPU overage:       \$$WORKER_CPU_COST"
echo "  DO request overage:       \$$DO_REQUEST_COST"
echo "  DO duration overage:      \$$DO_DURATION_COST"
echo "  D1 read overage:          \$$D1_READ_COST"
echo "  D1 write overage:         \$$D1_WRITE_COST"
echo "  Estimated usage charges:  \$$PROJECTED_USAGE_COST"
echo "  Estimated total:          \$$PROJECTED_TOTAL"
echo
echo "Notes"
echo "  - Durable Object WebSocket messages use Cloudflare's 20:1 billing ratio."
echo "  - GraphQL analytics are not invoice data and can differ from Billable Usage."
echo "  - The projection assumes the average so far continues; a new cost fix takes time to lower it."
