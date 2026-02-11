#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Run Codex end-to-end tmux verification without a browser.

Usage:
  ./scripts/e2e-codex-tmux.sh [options]

Options:
  --session NAME      tmux session name (default: crab-e2e-codex)
  --project DIR       project directory to run crabigator from (default: current dir)
  --timeout SEC       max seconds to wait for state transitions (default: 180)
  --full-auto         launch nested Codex with --full-auto
  --strict-question   require a real question prompt (no unavailable fallback)
  --keep              keep tmux session running after test
  --no-approve        do not auto-approve permission prompt
  --no-answer         do not auto-answer question prompt
  --help              show this help

Exit code is non-zero if required transitions/signals are not observed.
EOF
}

require_cmd() {
  local cmd="$1"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "Missing required command: $cmd" >&2
    exit 1
  fi
}

SESSION_NAME="crab-e2e-codex"
PROJECT_DIR="$(pwd)"
TIMEOUT_SECS=180
KEEP_SESSION=0
AUTO_APPROVE=1
AUTO_ANSWER=1
CODEX_FULL_AUTO=0
STRICT_QUESTION=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --session)
      SESSION_NAME="${2:-}"
      shift 2
      ;;
    --project)
      PROJECT_DIR="${2:-}"
      shift 2
      ;;
    --timeout)
      TIMEOUT_SECS="${2:-}"
      shift 2
      ;;
    --full-auto)
      CODEX_FULL_AUTO=1
      shift
      ;;
    --strict-question)
      STRICT_QUESTION=1
      shift
      ;;
    --keep)
      KEEP_SESSION=1
      shift
      ;;
    --no-approve)
      AUTO_APPROVE=0
      shift
      ;;
    --no-answer)
      AUTO_ANSWER=0
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if [[ ! -d "$PROJECT_DIR" ]]; then
  echo "Project directory does not exist: $PROJECT_DIR" >&2
  exit 1
fi

if ! [[ "$TIMEOUT_SECS" =~ ^[0-9]+$ ]]; then
  echo "--timeout must be an integer number of seconds" >&2
  exit 1
fi

require_cmd tmux
require_cmd jq
require_cmd rg
require_cmd find

if tmux has-session -t "$SESSION_NAME" 2>/dev/null; then
  echo "tmux session already exists: $SESSION_NAME" >&2
  echo "Use --session with a different name, or kill existing session first." >&2
  exit 1
fi

start_marker="$(mktemp)"
touch "$start_marker"

cleanup() {
  rm -f "$start_marker"
  if [[ "${KEEP_SESSION}" -eq 0 ]]; then
    tmux kill-session -t "$SESSION_NAME" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

if [[ -x "$PROJECT_DIR/target/debug/crabigator" ]]; then
  launch_cmd_parts=("target/debug/crabigator" "codex")
else
  launch_cmd_parts=("cargo" "run" "--" "codex")
fi

if [[ "$CODEX_FULL_AUTO" -eq 1 ]]; then
  launch_cmd_parts+=("--full-auto")
fi

printf -v q_project '%q' "$PROJECT_DIR"
printf -v launch_base_cmd '%q ' "${launch_cmd_parts[@]}"
launch_cmd="env -u CODEX_THREAD_ID -u CODEX_ROLLOUT_PATH ${launch_base_cmd% }"
echo "Starting tmux session: $SESSION_NAME"
tmux new-session -d -s "$SESSION_NAME" "cd $q_project && $launch_cmd"

session_dir=""
for _ in $(seq 1 40); do
  pane="$(tmux capture-pane -pt "$SESSION_NAME:0" -S -220 || true)"
  session_dir="$(printf '%s\n' "$pane" | sed -n 's/.*Session[[:space:]]*\/tmp\/\(crabigator-[^\/[:space:]]*\)\/.*/\/tmp\/\1/p' | tail -n 1)"
  if [[ -n "$session_dir" ]]; then
    break
  fi
  sleep 1
done

if [[ -z "$session_dir" ]]; then
  echo "Failed to detect crabigator session directory from tmux output" >&2
  exit 1
fi

inspect_json="$session_dir/inspect.json"
screen_txt="$session_dir/screen.txt"
scrollback_log="$session_dir/scrollback.log"
echo "Detected session directory: $session_dir"

for _ in $(seq 1 20); do
  if [[ -f "$inspect_json" ]]; then
    break
  fi
  sleep 1
done

if [[ ! -f "$inspect_json" ]]; then
  echo "inspect.json not found: $inspect_json" >&2
  exit 1
fi

codex_ready=0
echo "Waiting for nested Codex prompt..."
for _ in $(seq 1 40); do
  pane="$(tmux capture-pane -pt "$SESSION_NAME:0" -S -220 || true)"
  if printf '%s\n' "$pane" | rg -q 'context left'; then
    codex_ready=1
    break
  fi
  sleep 1
done

if [[ "$codex_ready" -ne 1 ]]; then
  echo "Warning: did not detect nested Codex prompt before sending test input." >&2
fi

marker="CRAB_E2E_${RANDOM}_$(date +%s)"
prompt="E2E marker: ${marker}. Run this exact test: 1) Use exec_command with cmd='open -a TextEdit', sandbox_permissions='require_escalated', and justification='Codex tmux E2E permission check'. 2) After that permission prompt is answered, call request_user_input with one question and two options. 3) Stop."
echo "Sending E2E prompt to nested Codex session..."
tmux send-keys -t "$SESSION_NAME:0" "$prompt"
sleep 1
tmux send-keys -t "$SESSION_NAME:0" C-m

seen_permission=0
seen_question=0
approved_permission=0
answered_question=0
last_state=""

echo "Waiting for state transitions (timeout: ${TIMEOUT_SECS}s)..."
for _ in $(seq 1 "$TIMEOUT_SECS"); do
  state="$(jq -r '.widgets.stats.data.state // "unknown"' "$inspect_json" 2>/dev/null || echo "unknown")"
  if [[ "$state" != "$last_state" ]]; then
    echo "state => $state"
    last_state="$state"
  fi

  if [[ "$state" == "permission" ]]; then
    seen_permission=1
    if [[ "$AUTO_APPROVE" -eq 1 && "$approved_permission" -eq 0 ]]; then
      echo "Permission prompt detected; approving option 1 (Enter)."
      tmux send-keys -t "$SESSION_NAME:0" C-m
      approved_permission=1
    fi
  fi

  if [[ "$state" == "question" ]]; then
    seen_question=1
    if [[ "$AUTO_ANSWER" -eq 1 && "$answered_question" -eq 0 ]]; then
      echo "Question prompt detected; selecting option 1."
      tmux send-keys -t "$SESSION_NAME:0" 1 C-m
      answered_question=1
    fi
  fi

  if [[ "$approved_permission" -eq 1 && "$state" == "complete" ]]; then
    if [[ "$AUTO_ANSWER" -eq 0 || "$seen_question" -eq 0 || "$answered_question" -eq 1 ]]; then
      break
    fi
  fi

  if [[ "$approved_permission" -eq 1 && "$answered_question" -eq 1 && "$state" != "permission" && "$state" != "question" ]]; then
    break
  fi

  sleep 1
done

candidate_logs=()
while IFS= read -r line; do
  candidate_logs+=("$line")
done < <(find "$HOME/.codex/sessions" -type f -name '*.jsonl' -newer "$start_marker" 2>/dev/null | sort)

if [[ "${#candidate_logs[@]}" -eq 0 ]]; then
  echo "No Codex JSONL logs modified after start marker." >&2
else
  echo "Candidate Codex logs:"
  for log in "${candidate_logs[@]}"; do
    echo "  - $log"
  done
fi

target_logs=("${candidate_logs[@]}")
marker_logs=()
if [[ "${#candidate_logs[@]}" -gt 0 ]]; then
  for log in "${candidate_logs[@]}"; do
    if rg -q "$marker" "$log"; then
      marker_logs+=("$log")
    fi
  done
fi

if [[ "${#marker_logs[@]}" -gt 0 ]]; then
  target_logs=("${marker_logs[@]}")
  echo "Marker-matched Codex logs:"
  for log in "${target_logs[@]}"; do
    echo "  - $log"
  done
fi

req_call_seen=0
question_call_seen=0
req_unavailable_seen=0
escalated_call_seen=0

if [[ "${#target_logs[@]}" -gt 0 ]]; then
  if rg -q '"type":"function_call","name":"request_user_input"' "${target_logs[@]}"; then
    req_call_seen=1
  fi
  if rg -q '"type":"function_call","name":"(request_user_input|AskUserQuestion|ask_user)"' "${target_logs[@]}"; then
    question_call_seen=1
  fi
  if rg -q 'request_user_input is unavailable in Default mode' "${target_logs[@]}"; then
    req_unavailable_seen=1
  fi
  if rg -q '"sandbox_permissions":"require_escalated"' "${target_logs[@]}"; then
    escalated_call_seen=1
  fi
fi

echo
echo "=== E2E Summary ==="
echo "tmux session:            $SESSION_NAME"
echo "launch mode:             $([[ "$CODEX_FULL_AUTO" -eq 1 ]] && echo full-auto || echo default)"
echo "session directory:       $session_dir"
echo "inspect.json:            $inspect_json"
echo "screen.txt:              $screen_txt"
echo "scrollback.log:          $scrollback_log"
echo "saw state=permission:    $seen_permission"
echo "saw state=question:      $seen_question"
echo "auto-approved permission:$approved_permission"
echo "auto-answered question:  $answered_question"
echo "saw request_user_input:  $req_call_seen"
echo "saw any question tool:   $question_call_seen"
echo "request unavailable msg: $req_unavailable_seen"
echo "saw escalated call:      $escalated_call_seen"

ok=1
if [[ "$seen_permission" -ne 1 ]]; then
  echo "FAIL: did not observe state=permission" >&2
  ok=0
fi
if [[ "$escalated_call_seen" -ne 1 ]]; then
  echo "FAIL: did not observe an escalated tool call in Codex logs" >&2
  ok=0
fi
if [[ "$question_call_seen" -ne 1 ]]; then
  echo "FAIL: did not observe a question tool call (AskUserQuestion/request_user_input)" >&2
  ok=0
fi
if [[ "$STRICT_QUESTION" -eq 1 ]]; then
  if [[ "$seen_question" -ne 1 ]]; then
    echo "FAIL: strict question mode requires state=question" >&2
    ok=0
  fi
  if [[ "$AUTO_ANSWER" -eq 1 && "$answered_question" -ne 1 ]]; then
    echo "FAIL: strict question mode expected auto-answer to be sent" >&2
    ok=0
  fi
else
  if [[ "$seen_question" -ne 1 && "$req_unavailable_seen" -ne 1 ]]; then
    echo "FAIL: did not observe question state and no explicit unavailable message" >&2
    ok=0
  fi
fi

if [[ "$ok" -ne 1 ]]; then
  exit 1
fi

echo "PASS: Codex tmux E2E flow validated from local files."
