#!/bin/sh
# Launch Crabigator in the focused Herdr pane.
set -eu

herdr="${HERDR_BIN_PATH:-herdr}"
pane="${HERDR_PANE_ID:-}"
if [ -z "$pane" ]; then
  echo "HERDR_PANE_ID is not set; this action must run inside Herdr." >&2
  exit 1
fi

name="cg$(date +%s | tail -c 6 | tr -d '\n')"
if [ "$#" -gt 0 ]; then
  exec "$herdr" agent start "$name" --kind crabigator --pane "$pane" -- "$@"
fi
exec "$herdr" agent start "$name" --kind crabigator --pane "$pane"
