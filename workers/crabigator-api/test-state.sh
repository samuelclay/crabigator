#!/bin/bash
# Get current state of a crabigator session
# Usage: ./test-state.sh <session_id>

set -e

SESSION_ID="${1:-}"

if [ -z "$SESSION_ID" ]; then
    echo "Usage: ./test-state.sh <session_id>"
    exit 1
fi

# Generate auth headers using Python
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

secret_hash = hashlib.sha256(device_secret.encode()).hexdigest()
timestamp = str(int(time.time() * 1000))

method = "GET"
path = f"/api/sessions/{session_id}/state"
message_to_sign = f"{method}:{path}:{timestamp}"
signature = hmac.new(secret_hash.encode(), message_to_sign.encode(), hashlib.sha256).hexdigest()

print(f"DEVICE_ID={device_id}")
print(f"TIMESTAMP={timestamp}")
print(f"SIGNATURE={signature}")
print(f"API_PATH={path}")
PYEOF
)

eval "$AUTH_HEADERS"

curl -s "https://drinkcrabigator.com${API_PATH}" \
    -H "X-Device-Id: $DEVICE_ID" \
    -H "X-Timestamp: $TIMESTAMP" \
    -H "X-Signature: $SIGNATURE" | jq .
