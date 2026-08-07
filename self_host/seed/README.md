# Self-host leaf seeds (H6e / P6 seed-only)

Committed `.kbc` for skip-listed cores so `KABOOTAR_VM=kab-only` can load them
**without a live Rust compile** (fingerprint must match source).

## Policy (P6 product path — do not empty yet)

**P6 ✅** = seed-only is the product path for the five leaves.
**P6b 📋** = empty `SELF_HOST_SKIP_LISTED_LEAVES` only when every leaf
self-host-compiles under `P6_SELF_HOST_LEAF_CI_FAST_MS` (10s). Today even
~13KB leaves take minutes (AST-cost); see `self_host/emit.kab` comments.

**P6b progress:** first speed target = `serialize_body.kab` (smallest leaf).
`irOpLine` uses `IR_WITH_ARG`/`IR_ZERO_ARG` membership (+ `joinComma`) instead of
~58 If arms; source ~8.8KB. Measured self-host compile (debug `cargo test`):
~889s — still ≫ `P6_SELF_HOST_LEAF_CI_FAST_MS` (10s); skip-list stays 5.
Run `cargo test --test self_host p6b_serialize_body_compile_budget -- --ignored --nocapture`
(or full `p6_leaf_self_host_compile_budget`) to re-record ms; do not flip
`P6B_EMPTY_SKIP_LIST_READY` until all five are under budget.

Gates:

- `p6_seed_only_all_leaves_have_seeds` — files exist, list length stays 5
- `p6_seed_fingerprint_all_leaves_load` — each seed deserializes and fingerprint matches source
- `p6_skip_list_stays_until_ci_fast_gate` — oversize emit stays skipped
- `p6_leaf_self_host_compile_budget` (ignored) — timing probe for P6b
- `P6B_EMPTY_SKIP_LIST_READY` — must stay `false` until budget passes (asserted in `p6_skip_list_stays_until_ci_fast_gate`)

## Seeds

| Seed | Source |
|------|--------|
| `emit_impl.kab.kbc` | `../emit_impl.kab` |
| `parser_impl.kab.kbc` | `../parser_impl.kab` |
| `lexer_impl.kab.kbc` | `../lexer_impl.kab` |
| `serialize_body.kab.kbc` | `../serialize_body.kab` |
| `vm_run_body.kab.kbc` | `../vm_run_body.kab` |

Regenerate after editing a leaf:

```bash
./scripts/regen_self_host_seeds.sh
```

Requires a built `kabootar` binary (`CARGO_TARGET_DIR` / `KABOOTAR_BIN` optional).
