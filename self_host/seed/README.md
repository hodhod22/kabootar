# Self-host leaf seeds (H6e / P6 seed-only)

Committed `.kbc` for skip-listed cores so `KABOOTAR_VM=kab-only` can load them
**without a live Rust compile** (fingerprint must match source).

**Policy:** leaves stay skip-listed; emptying the list waits until self-host
compile of these shards is CI-fast. See `SELF_HOST_SKIP_LISTED_LEAVES` /
`self_host_skip_policy()` in `src/compile/mod.rs`.

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
