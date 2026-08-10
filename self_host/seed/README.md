# Self-host leaf seeds (H6e / P6 seed-only)

Committed `.kbc` for skip-listed cores so `KABOOTAR_VM=kab-only` can load them
**without a live Rust compile** (fingerprint must match source).

## Policy

| Track | Meaning |
|-------|---------|
| **P6 ✅** | Seed-only is the **product path** for skip-listed leaves. |
| **P6b 📋** | Empty `SELF_HOST_SKIP_LISTED_LEAVES` **only** when every leaf self-host-compiles under `P6_SELF_HOST_LEAF_CI_FAST_MS` (10 000 ms). |

## Status (2026-08-10)

**Serialize path is clear of the skip-list** — densified pure-threaded shards all
self-host-compile under 10 s (measured). Remaining skip-listed leaves:

| Leaf | Role |
|------|------|
| `emit_impl.kab` | emit body |
| `parser_impl.kab` | parser body |
| `lexer_impl.kab` | lexer body |
| `vm_run_body.kab` | VM run body |

```bash
./scripts/regen_self_host_seeds.sh
```
