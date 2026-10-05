#!/usr/bin/env bash
# SH28: regenerate self_host/seed/dag/*.kbc with the Kab compiler itself —
# `kabootar compile <f> --self-host` on every DAG file, then copy the
# produced .kabootar/cache entry into self_host/seed/dag (the fingerprint
# + source= lines are identical to write_seed_dag_file's format).
# Same evidence as tests `sh28_self_host_seed_write_smoke`, without the
# 85k-line test binary. Usage:
#   ./scripts/regen_dag_seeds_selfhost.sh
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
BIN="${KABOOTAR_BIN:-}"
if [[ -z "$BIN" ]]; then
  for c in target/release/kabootar.exe target/release/kabootar; do
    if [[ -x "$c" ]]; then BIN="$c"; break; fi
  done
fi
if [[ -z "${BIN}" || ! -x "$BIN" ]]; then
  echo "No kabootar binary; set KABOOTAR_BIN or cargo build --release --bin kabootar" >&2
  exit 1
fi
mkdir -p self_host/seed/dag
# DAG list = the REAL compile DAG: BFS of plain `import "x"` edges from
# self_host/compile — the same walk walk_compile_dag() does in Rust
# (pub import is a re-export edge, not a compile-DAG dependency). Never
# iterate the seed dir itself: stale seeds for deleted sources live there.
declare -A seen=()
queue=("self_host/compile")
order=()
while ((${#queue[@]})); do
  spec="${queue[0]}"
  queue=("${queue[@]:1}")
  spec="${spec%.kab}"
  [[ -n "${seen[$spec]:-}" ]] && continue
  seen[$spec]=1
  rel="$spec.kab"
  [[ -f "$rel" ]] || continue
  order+=("$rel")
  while IFS= read -r dep; do
    queue+=("$dep")
  done < <(grep -oE '^import[[:space:]]+"[^"]+"' "$rel" | sed -E 's/^import[[:space:]]+"([^"]+)"/\1/')
done
echo "compile DAG: ${#order[@]} files"
count=0
fail=0
for rel in "${order[@]}"; do
  name="$(basename "$rel")"               # e.g. emit_stmt_body.kab
  echo "=== $rel (self-host)"
  if ! "$BIN" compile "$rel" --self-host; then
    echo "FAILED: $rel" >&2
    fail=$((fail + 1))
    continue
  fi
  src=".kabootar/cache/self_host__${name}.kbc"
  dst="self_host/seed/dag/${name}.kbc"
  if [[ ! -f "$src" ]]; then
    echo "NO CACHE WRITTEN for $rel" >&2
    fail=$((fail + 1))
    continue
  fi
  cp "$src" "$dst"
  count=$((count + 1))
  echo "wrote $dst"
done
echo "regenerated $count seeds by Kab self-host (failures: $fail)"
[[ $fail -eq 0 ]]
