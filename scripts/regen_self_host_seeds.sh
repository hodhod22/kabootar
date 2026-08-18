#!/usr/bin/env bash
# Regenerate committed self_host/seed/*.kbc for H6e skip-listed leaves.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
BIN="${KABOOTAR_BIN:-}"
if [[ -z "$BIN" ]]; then
  for c in target-h6e5/release/kabootar.exe target/release/kabootar.exe target/release/kabootar; do
    if [[ -x "$c" ]]; then BIN="$c"; break; fi
  done
fi
if [[ -z "${BIN}" || ! -x "$BIN" ]]; then
  echo "No kabootar binary; set KABOOTAR_BIN or build release" >&2
  exit 1
fi
mkdir -p self_host/seed
LEAVES=(emit_impl.kab parser_impl.kab lexer_impl.kab)
for name in "${LEAVES[@]}"; do
  echo "compile $name --rust"
  KABOOTAR_COMPILE=rust "$BIN" compile "self_host/$name" --rust
  src=".kabootar/cache/self_host__${name}.kbc"
  dst="self_host/seed/${name}.kbc"
  python - "$src" "$dst" "self_host/$name" <<'PY'
import sys
from pathlib import Path
src, dst, rel = sys.argv[1], sys.argv[2], sys.argv[3]
text = Path(src).read_text(encoding="utf-8")
out = []
for line in text.splitlines(True):
    if line.startswith("source="):
        out.append(f"source={rel}\n")
    else:
        out.append(line)
Path(dst).write_text("".join(out), encoding="utf-8", newline="\n")
print(f"wrote {dst}")
PY
done
echo "done — commit self_host/seed/*.kbc"
