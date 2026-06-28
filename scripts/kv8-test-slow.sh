#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
echo "Kv8 slow tests: full React bundle eval — expect 2–10+ minutes (not a hang)."
echo "=== timing probe (prints eval_ops) ==="
cargo test -p kabootar --release --lib runtime::kv8::bundle::parse_probe::react_runtime_eval_timing -- --ignored --exact --nocapture
echo "=== remaining slow probes ==="
cargo test -p kabootar --release --lib 'runtime::kv8::bundle::parse_probe::' -- --ignored --skip react_runtime_eval_timing
