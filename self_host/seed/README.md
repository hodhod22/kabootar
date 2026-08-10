# Self-host leaf seeds (H6e / P6 seed-only)

Committed `.kbc` for skip-listed cores so `KABOOTAR_VM=kab-only` can load them
**without a live Rust compile** (fingerprint must match source).

## Policy

| Track | Meaning |
|-------|---------|
| **P6 ✅** | Seed-only is the **product path** for skip-listed leaves. |
| **P6b 📋** | Empty `SELF_HOST_SKIP_LISTED_LEAVES` **only** when every leaf self-host-compiles under `P6_SELF_HOST_LEAF_CI_FAST_MS` (10 000 ms). |

## Measured (debug, 2026-08-10 after out/sections/op shard)

| Leaf | ms | Status |
|------|-----|--------|
| `serialize_out_base` | ~7.8 s | under budget (not skip-listed) |
| `serialize_out_tagged` | ~5.8 s | under budget (not skip-listed) |
| `serialize_ops` | ~7.3 s | under budget (not skip-listed) |
| `serialize_lists` | ~5.9 s | under budget (not skip-listed) |
| `serialize_esc` | ~8.8 s | under budget (not skip-listed) |
| `serialize_out_try` | ~11 s | skip-listed |
| `serialize_arrows` / `serialize_classes` / `serialize_const` | ~10.5–11 s | skip-listed |
| `serialize_fns` / `serialize_class_methods` | ~14–15 s | skip-listed |
| `serialize_ir_line` | ~18 s | skip-listed |
| `serialize_acc` | ~24 s | skip-listed |

## Seeds (12 leaves)

`emit_impl`, `parser_impl`, `lexer_impl`, `serialize_out_try`, `serialize_fns`,
`serialize_arrows`, `serialize_class_methods`, `serialize_classes`, `serialize_acc`,
`serialize_const`, `serialize_ir_line`, `vm_run_body`.

```bash
./scripts/regen_self_host_seeds.sh
```
