#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo test --test kv8_smoke --release
cargo test -p kabootar --release --lib 'runtime::kv8::bundle::parse_probe::' -- --skip react_runtime_eval_probe --skip react_create_element_probe --skip react_runtime_create_root --skip react_runtime_counter_smoke --skip react_has_client_internals --skip inner_t_shadow_react_internals --skip dom_runtime_eval_probe --skip umd_eval_probe --skip react_umd_create_element_probe --skip react_umd_create_root
