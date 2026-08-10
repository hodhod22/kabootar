# Self-host leaf seeds (H6e / P6 seed-only)

Committed `.kbc` for skip-listed cores so `KABOOTAR_VM=kab-only` can load them
**without a live Rust compile** (fingerprint must match source).

## Policy

| Track | Meaning |
|-------|---------|
| **P6 ✅** | Seed-only is the **product path** for skip-listed leaves. |
| **P6b 📋** | Empty `SELF_HOST_SKIP_LISTED_LEAVES` **only** when every leaf self-host-compiles under `P6_SELF_HOST_LEAF_CI_FAST_MS` (10 000 ms). |

## Measured (debug, 2026-08-10 after acc/ir_line densify)

### Promoted (≤10 s, not skip-listed)
`serialize_acc` ~6 s, `serialize_acc_pool` ~7 s, `serialize_out_try5/4`, `serialize_fn_tries`,
`serialize_class_method_ops`, `serialize_classes`, `serialize_const`, `serialize_join`,
plus earlier `out_base`/`out_tagged`/`ops`/`lists`/`esc`.

### Still skip-listed
| Leaf | ~ms |
|------|-----|
| `serialize_fns` | ~12 s |
| `serialize_arrows` | ~11 s |
| `serialize_class_methods` | ~11 s |
| `serialize_acc_tail` | ~10.0 s (borderline) |
| `serialize_ir_op` | ~13 s |

## Seeds (9 leaves)

`emit_impl`, `parser_impl`, `lexer_impl`, `serialize_fns`, `serialize_arrows`,
`serialize_class_methods`, `serialize_acc_tail`, `serialize_ir_op`, `vm_run_body`.

```bash
./scripts/regen_self_host_seeds.sh
```
