# Self-host leaf seeds (H6e / P6 seed-only)

Committed `.kbc` for skip-listed cores so `KABOOTAR_VM=kab-only` can load them
**without a live Rust compile** (fingerprint must match source).

## Policy

| Track | Meaning |
|-------|---------|
| **P6 ✅** | Seed-only is the **product path** for skip-listed leaves. |
| **P6b 📋** | Empty `SELF_HOST_SKIP_LISTED_LEAVES` **only** when every leaf self-host-compiles under `P6_SELF_HOST_LEAF_CI_FAST_MS` (10 000 ms). |
| **Flag** | `P6B_EMPTY_SKIP_LIST_READY` must stay `false` until that budget passes (asserted in `p6_skip_list_stays_until_ci_fast_gate`). |

Self-host cost scales with **AST density of the source being compiled**, not with
import-time cost of dependents.

## Phase profiling (required before guessing)

```bash
python scripts/profile_emit_compile.py phases self_host/serialize_out.kab --timeout 600
python scripts/profile_emit_compile.py phases self_host/serialize_sections.kab --timeout 600
python scripts/profile_emit_compile.py bisect serialize_out
python scripts/profile_emit_compile.py bisect serialize_sections
```

**2026-08-10:** Serialize shards use **pure string threading** (`out` returned) —
module-local AccAdd session is unreliable across seed-loaded imports. Pure split:
`serialize_esc` (CI-fast, not skip-listed) + `serialize_op`. Thin aggregators:
`serialize_pure`, `serialize_defs`, `serialize_body`.

## P6b playbook

1. **Densify skip-listed source** — prefer pure `out`-threading across shards;
   keep Binary AccAdd only inside a single module when shared session is required.
2. **Speed toolchain** (parser/emit len caches, Rc Array/Object, …) — see prior notes.
3. **Measure** before flipping any flag:
   ```bash
   cargo test --test self_host p6b_serialize_body_compile_budget -- --nocapture
   cargo test --test self_host p6b_serialize_shards_compile_budget -- --ignored --nocapture
   cargo test --test self_host p6_leaf_self_host_compile_budget -- --ignored --nocapture
   ```
4. **Do not** empty the skip-list or set `P6B_EMPTY_SKIP_LIST_READY=true` until
   step 3 shows **all** leaves `ok` and `ms ≤ 10000`.

### Measured baselines (debug, host VM, 2026-08-10)

| Leaf | Notes | Last recorded |
|------|-------|---------------|
| `serialize_body.kab` | thin facade | **~2 s** (under budget) |
| `serialize_ir.kab` | IR tables | **~1.5 s** |
| `serialize_esc.kab` | escape helper | **~8.8 s** (under budget; not skip-listed) |
| `serialize_out.kab` | out helpers (skip-listed) | **~27 s** |
| `serialize_acc.kab` | serSerializeBc (skip-listed) | **~20 s** |
| `serialize_op.kab` | const/op/join (skip-listed) | **~29 s** |
| `serialize_sections.kab` | appenders (skip-listed) | **~61 s** |
| others | Larger / denser | not under budget |

## Gates

| Test | Role |
|------|------|
| `p6_seed_only_all_leaves_have_seeds` | Files exist; list length stays 8 |
| `p6_seed_fingerprint_all_leaves_load` | Seed deserializes; fingerprint matches source |
| `p6_skip_list_stays_until_ci_fast_gate` | Oversize emit stays skipped; flag off |
| `p6b_serialize_body_still_skip_listed_progress` | out/sections/acc/op skip-listed; esc attemptable |
| `p6b_serialize_body_compile_budget` | Thin body must be ≤10 s (CI) |
| `p6b_serialize_shards_compile_budget` (ignored) | Timing probe for serialize shards |
| `p6_leaf_self_host_compile_budget` (ignored) | Timing probe for all leaves |

## Seeds

| Seed | Source |
|------|--------|
| `emit_impl.kab.kbc` | `../emit_impl.kab` |
| `parser_impl.kab.kbc` | `../parser_impl.kab` |
| `lexer_impl.kab.kbc` | `../lexer_impl.kab` |
| `serialize_out.kab.kbc` | `../serialize_out.kab` |
| `serialize_sections.kab.kbc` | `../serialize_sections.kab` |
| `serialize_acc.kab.kbc` | `../serialize_acc.kab` |
| `serialize_op.kab.kbc` | `../serialize_op.kab` |
| `vm_run_body.kab.kbc` | `../vm_run_body.kab` |

```bash
./scripts/regen_self_host_seeds.sh
```
