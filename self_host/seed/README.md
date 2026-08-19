# Self-host leaf seeds (H6e / P6b attempt-all)

Committed `.kbc` for historical cores so `KABOOTAR_VM=kab-only` can load them
as a cache. **Skip-list is empty** — emit/parser/lexer impls are thin drivers
that self-host-compile under `P6_SELF_HOST_LEAF_CI_FAST_MS` (10 000 ms).

**SH1:** `seed/dag/*.kbc` — rust-compiled bytecode for `compile`/`parse`/`emit`/`serialize`/`ownership`
facades (fingerprint-validerad). Regenerera:

```bash
# facader (CI via tests/sh_wave.rs sh1_compiler_facade_seeds)
# hela compile-DAG:en:
KABOOTAR_SH1_WARM=1 cargo test --test sh_wave sh1 -- --ignored
```

`write_compiler_dag_seeds()` skriver ett `.kbc` per DAG-fil under `seed/dag/`.

## Policy

| Track | Meaning |
|-------|---------|
| **P6 ✅** | Seed files remain a kab-only cache. |
| **P6b ✅** | `SELF_HOST_SKIP_LISTED_LEAVES` is empty. |

## Status (2026-08-18)

| Leaf | Role | Compile (host VM, rust compile) |
|------|------|----------------------------------|
| `emit_impl.kab` | thin driver | ~1.3 s |
| `parser_impl.kab` | thin driver | ~2.0 s |
| `lexer_impl.kab` | thin driver | ~4.6 s |

Parser densify: session trampoline + stmt/postfix/compare shards; `test_parser.kab` 123/123.
Lexer densify: session-style scan shards (`lexer_scan_*`, `lexer_tokenize`); `test_lexer.kab` 230/230.

```bash
./scripts/regen_self_host_seeds.sh
```
