# Self-host leaf seeds (H6e / P6 seed-only)

Committed `.kbc` for skip-listed cores so `KABOOTAR_VM=kab-only` can load them
**without a live Rust compile** (fingerprint must match source).

## Policy (do not empty the skip-list yet)

Leaves stay skip-listed under **seed-only** policy. Emptying
`SELF_HOST_SKIP_LISTED_LEAVES` waits until self-host compile of these shards is
CI-fast. See `self_host_skip_policy()` in `src/compile/mod.rs`.

Gates:

- `p6_seed_only_all_leaves_have_seeds` — files exist, list length stays 5
- `p6_seed_fingerprint_all_leaves_load` — each seed deserializes and fingerprint matches source

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
