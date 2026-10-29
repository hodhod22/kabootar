#!/usr/bin/env bash
# Kör en av 30 delar av integrationstestet sh_wave.
# Varje test startar en egen process så minnet lämnas tillbaka mellan testerna.
# Användning: bash scripts/sh_wave_part.sh 12
set -u
part="${1:-}"
if ! [[ "$part" =~ ^[0-9]+$ ]] || [ "$part" -lt 1 ] || [ "$part" -gt 30 ]; then
  echo "användning: bash scripts/sh_wave_part.sh <1-30>"
  exit 2
fi
export PATH="${USERPROFILE}/.cargo/bin:${PATH}"
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root" || exit 1
list="$(printf 'scripts/sh_wave_parts/part-%02d.txt' "$part")"
if [ ! -f "$list" ]; then
  echo "saknar $list"
  exit 1
fi
cargo test --test sh_wave --no-run || exit 1
exe="$(ls -1t target/debug/deps/sh_wave-*.exe | head -n 1)"
if [ -z "$exe" ]; then
  echo "hittade inte testbinären"
  exit 1
fi
while IFS= read -r name || [ -n "$name" ]; do
  name="${name//$'\r'/}"
  [ -z "$name" ] && continue
  echo "---- $name"
  log="$("$exe" --test-threads=1 --exact "$name" 2>&1)" || {
    printf '%s\n' "$log"
    echo "FAILED part $part: $name"
    exit 1
  }
  printf '%s\n' "$log"
  if printf '%s\n' "$log" | grep -q "running 0 tests"; then
    echo "FAILED part $part: $name matchade inget test"
    exit 1
  fi
done < "$list"
echo "part $part ok"
