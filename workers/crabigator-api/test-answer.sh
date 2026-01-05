#!/bin/bash
# Test script to send an answer to a crabigator session via the cloud API
# Usage: ./test-answer.sh <session_id> "Your message here"

set -e

SESSION_ID="${1:-}"
MESSAGE="${2:-Tell me a joke, but use the AskUserQuestionTool first}"

if [ -z "$SESSION_ID" ]; then
    echo "Usage: ./test-answer.sh <session_id> [message]"
    echo "Get session_id from the 'Cloud  ✓ <session_id>' line in crabigator output"
    exit 1
fi

# Read device credentials and generate auth headers using Python
AUTH_HEADERS=$(SESSION_ID="$SESSION_ID" python3 << 'PYEOF'
import json
import hashlib
import hmac
import time
import os

session_id = os.environ.get('SESSION_ID', '')

device_path = os.path.expanduser('~/.crabigator/device.json')
if not os.path.exists(device_path):
    print("ERROR: Device not registered")
    exit(1)

with open(device_path) as f:
    device = json.load(f)

device_id = device['device_id']
device_secret = device['device_secret']

# Compute secret_hash (SHA-256 of device_secret)
secret_hash = hashlib.sha256(device_secret.encode()).hexdigest()

# Generate timestamp
timestamp = str(int(time.time() * 1000))

# Sign the request
method = "POST"
path = f"/api/sessions/{session_id}/answer"
message_to_sign = f"{method}:{path}:{timestamp}"
signature = hmac.new(secret_hash.encode(), message_to_sign.encode(), hashlib.sha256).hexdigest()

# Output as shell variables
print(f"DEVICE_ID={device_id}")
print(f"TIMESTAMP={timestamp}")
print(f"SIGNATURE={signature}")
print(f"API_PATH={path}")
PYEOF
)

if [ $? -ne 0 ]; then
    echo "Failed to generate auth headers"
    exit 1
fi

eval "$AUTH_HEADERS"

echo "Session ID: $SESSION_ID"
echo "Device ID: $DEVICE_ID"
echo "Timestamp: $TIMESTAMP"
echo "Message: $MESSAGE"
echo ""

# Send the answer
curl -s -X POST "https://drinkcrabigator.com${API_PATH}" \
    -H "Content-Type: application/json" \
    -H "X-Device-Id: $DEVICE_ID" \
    -H "X-Timestamp: $TIMESTAMP" \
    -H "X-Signature: $SIGNATURE" \
    -d "{\"text\": \"$MESSAGE\"}" | jq .

echo ""
echo "Done! Check your local crabigator session."
