#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
if [ -s "$root/crabigator.path" ]; then
  bin=$(cat "$root/crabigator.path")
else
  bin=$(command -v crabigator)
fi
if [ -z "${bin:-}" ] || [ ! -x "$bin" ]; then
  echo "crabigator not found. Install it and rerun: herdr plugin link $root" >&2
  exit 1
fi
exec "$bin" "$@"
