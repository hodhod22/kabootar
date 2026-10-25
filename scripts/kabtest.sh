#!/usr/bin/env bash
# Run product tests without cargo test. Uses the already built kabootar binary.
set -u
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root" || exit 1
exe="${KABOOTAR_BIN:-}"
if [ -z "$exe" ]; then
  exe="$(ls -1t \
    "$LOCALAPPDATA/Temp/cursor-sandbox-cache"/*/cargo-target/debug/kabootar.exe \
    target/debug/kabootar.exe \
    2>/dev/null | head -n 1)"
fi
if [ -z "$exe" ] || [ ! -f "$exe" ]; then
  echo "hittade inte kabootar.exe. Bygg den en gång med: cargo build --bin kabootar"
  exit 1
fi
target="${1:-tests}"
exec "$exe" test "$target"
