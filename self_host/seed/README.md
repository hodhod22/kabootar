# Self-host leaf seeds (H6e / P6 seed-only)

Committed `.kbc` for skip-listed cores so `KABOOTAR_VM=kab-only` can load them
**without a live Rust compile** (fingerprint must match source).

## Policy

| Track | Meaning |
|-------|---------|
| **P6 ✅** | Seed-only is the **product path** for skip-listed leaves. |
| **P6b 📋** | Empty `SELF_HOST_SKIP_LISTED_LEAVES` **only** when every leaf self-host-compiles under `P6_SELF_HOST_LEAF_CI_FAST_MS` (10 000 ms). |

## Status (2026-08-11)

**Serialize path is clear of the skip-list** — densified pure-threaded shards all
self-host-compile under 10 s (measured via `scripts/_emit_shard_times.py`).

| Leaf | Role | Notes |
|------|------|-------|
| `emit_impl.kab` | emit driver | thin driver; all `emit_*` shards ≤10 s — seed regen’d |
| `parser_impl.kab` | parser body | densified into `parser_*` shards + session trampoline; split `parser_stmt`/`parser_postfix` until all ≤10 s |
| `lexer_impl.kab` | lexer body | still monolithic (~82 s); do **not** multi-module-shard |

**Emit densify ✅ (shards):** session trampoline + kind handlers + shared helpers.
**Parser densify (in progress):** `parser_session` + `parser_hooks`/`parser_tramp` + expr/stmt shards.
Phase 2: `parser_postfix_*`, `parser_compare_*`, `parser_add_shift_*`, `parser_stmt_*` (122/123 `test_parser.kab`).
Phase 3: further densify — session field groups, class/fn/iface/enum/try/if/postfix_lit/tail/type_args/main helpers.
Regenerators: `scripts/_densify_parser_impl.py`, `scripts/_split_parser_shards.py`, `scripts/_densify_parser_phase3.py`.
Measure: `scripts/_parser_shard_times.py`, `scripts/_parser_all_shard_times.py`, `scripts/_leaf_compile_times.py`.
Many shards still >10 s (`parser_stmt_class_method`, `parser_postfix_paren`, …) — continue densify before seed regen / skip-list clear.
Do **not** empty skip-list until parser + lexer leaves also ≤10 s.

**VM path is clear of the skip-list** — `vm_run_exec_core` densified (~6.7 s) via session trampoline + `vm_run_hook_*` / `vm_run_tramp_*` shards.

```bash
./scripts/regen_self_host_seeds.sh
```
