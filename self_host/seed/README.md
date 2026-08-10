# Self-host leaf seeds (H6e / P6 seed-only)

Committed `.kbc` for skip-listed cores so `KABOOTAR_VM=kab-only` can load them
**without a live Rust compile** (fingerprint must match source).

## Policy

| Track | Meaning |
|-------|---------|
| **P6 ✅** | Seed-only is the **product path** for the five leaves. |
| **P6b 📋** | Empty `SELF_HOST_SKIP_LISTED_LEAVES` **only** when every leaf self-host-compiles under `P6_SELF_HOST_LEAF_CI_FAST_MS` (10 000 ms). |
| **Flag** | `P6B_EMPTY_SKIP_LIST_READY` must stay `false` until that budget passes (asserted in `p6_skip_list_stays_until_ci_fast_gate`). |

Self-host cost scales with **AST density of the source being compiled**, not with
import-time cost of dependents.

## Phase profiling (required before guessing)

```bash
python scripts/profile_emit_compile.py phases self_host/serialize_defs.kab --timeout 1200
python scripts/profile_emit_compile.py bisect serialize_body   # thin facade
python scripts/profile_emit_compile.py bisect serialize_defs
```

**2026-08-10:** AccAdd serialize body moved into `serialize_defs` (skip-listed). Thin
`serialize_body` facade self-hosts in **~2 s** (under budget). `serialize_defs` still
**~152 s** — first remaining serialize speed target.

## P6b playbook

1. **Densify skip-listed source** — fewer If/Binary trees; move pure helpers to
   non-listed modules only when that module stays CI-fast or is itself skip-listed.
2. **Speed toolchain** (parser/emit len caches, Rc Array/Object, …) — see prior notes.
3. **Measure** before flipping any flag:
   ```bash
   cargo test --test self_host p6b_serialize_body_compile_budget -- --nocapture
   cargo test --test self_host p6b_serialize_defs_compile_budget -- --ignored --nocapture
   cargo test --test self_host p6_leaf_self_host_compile_budget -- --ignored --nocapture
   ```
4. **Do not** empty the skip-list or set `P6B_EMPTY_SKIP_LIST_READY=true` until
   step 3 shows **all five** leaves `ok` and `ms ≤ 10000`.

### Measured baselines (debug `cargo test`, host VM)

| Leaf | Notes | Last recorded |
|------|-------|---------------|
| `serialize_body.kab` | thin facade → `serSerializeBc` | **~2 s** (under budget; not skip-listed) |
| `serialize_defs.kab` | AccAdd + sections (skip-listed) | **~152 s** debug (2026-08-10) — ≫ 10 s |
| others | Larger / denser | not under budget |

## Gates

| Test | Role |
|------|------|
| `p6_seed_only_all_leaves_have_seeds` | Files exist; list length stays 5 |
| `p6_seed_fingerprint_all_leaves_load` | Seed deserializes; fingerprint matches source |
| `p6_skip_list_stays_until_ci_fast_gate` | Oversize emit stays skipped; flag off |
| `p6b_serialize_body_still_skip_listed_progress` | defs skip-listed; body thin + attemptable |
| `p6b_serialize_body_compile_budget` | Thin body must be ≤10 s (CI) |
| `p6b_serialize_defs_compile_budget` (ignored) | Timing probe for serialize_defs |
| `p6_leaf_self_host_compile_budget` (ignored) | Timing probe for all five leaves |

## Seeds

| Seed | Source |
|------|--------|
| `emit_impl.kab.kbc` | `../emit_impl.kab` |
| `parser_impl.kab.kbc` | `../parser_impl.kab` |
| `lexer_impl.kab.kbc` | `../lexer_impl.kab` |
| `serialize_defs.kab.kbc` | `../serialize_defs.kab` |
| `vm_run_body.kab.kbc` | `../vm_run_body.kab` |

```bash
./scripts/regen_self_host_seeds.sh
# or single leaf via kabootar compile --rust + copy cache → seed/
```
