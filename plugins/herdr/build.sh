#!/bin/sh
# Record the absolute Crabigator binary so pane/action commands still work
# when Herdr's PATH is minimal.
set -eu

root=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
bin=$(command -v crabigator || true)
if [ -z "$bin" ]; then
  echo "crabigator is not on PATH. Install it, then reinstall this plugin." >&2
  echo "herdr plugin link $root" >&2
  : >"$root/crabigator.path"
  exit 0
fi
printf '%s\n' "$bin" >"$root/crabigator.path"
echo "recorded crabigator at $bin"
