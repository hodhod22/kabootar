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
# Tiny / mid sources: use scripts/profile_emit_compile.py (Windows: c:/… mounts)
python scripts/profile_emit_compile.py phases self_host/serialize_body.kab --timeout 1200

# Or hand probe (same counters):
# PROFILE phase parse_ms / emit_ms / serialize_ms / total_ms
```

**Recorded ratio (AccAdd-dense mid smoke, debug host VM, after toolchain maps):**
emit ≈ 47% | parse ≈ 36% | serialize ≈ 17% (40× `s = s + …`).
Tiny if/+ smoke previously: parse ≈ emit ≫ serialize.

**Full leaf:** `p6b_serialize_body_compile_budget` **~643 s** (was ~689 s) with `eLocalMap` +
map-only `emitSym` + early `IDENT=`. Still ≫ 10 s — skip-list stays.

**Toolchain focus (after leaf densify plateau):** finish `emitSym`/`localSymIndex`
maps + early `IDENT=` in `parseStmt`. Profile script uses `KABOOTAR_COMPILE=rust` +
`kabootar run` so `PROFILE phase *_ms` lines are reliable on Windows.

## P6b playbook

1. **Densify leaf source** — fewer If/Binary trees:
   - `IR_WITH_ARG` / `IR_ZERO_ARG` in **`serialize_defs`** (not leaf Const AST)
   - **shallow AccAdd** (`outTag` / `outSpNum` / `sLine`) — avoid depth-16+ `+` trees
2. **Speed toolchain:**
   - `symIndex` const/global **maps** + **`eLocalMap` / map-only `emitSym`**
   - AccAdd recurse; early `emitIfStmt`; **`eOpsN`/`eFnOpsN`** in jump patches
   - **iterative left-assoc `+`/`-`** + **early `IDENT=`** in `parser_impl`
3. **Measure** before flipping any flag:
   ```bash
   cargo test --test self_host p6b_serialize_body_compile_budget -- --ignored --nocapture
   cargo test --test self_host p6_leaf_self_host_compile_budget -- --ignored --nocapture
   ```
4. **Do not** empty the skip-list or set `P6B_EMPTY_SKIP_LIST_READY=true` until
   step 3 shows **all five** leaves `ok` and `ms ≤ 10000`.

### Measured baselines (debug `cargo test`, host VM)

| Leaf | Notes | Last recorded |
|------|-------|---------------|
| `serialize_body.kab` | IR in defs + toolchain `eLocalMap`/`emitSym` maps + early `IDENT=` | **~643 s** debug (2026-08-07; prior ~689 s / ~670 s / ~885–964 s) — still ≫ 10 s |
| others | Larger / denser | not under budget |

## Gates

| Test | Role |
|------|------|
| `p6_seed_only_all_leaves_have_seeds` | Files exist; list length stays 5 |
| `p6_seed_fingerprint_all_leaves_load` | Seed deserializes; fingerprint matches source |
| `p6_skip_list_stays_until_ci_fast_gate` | Oversize emit stays skipped; flag off |
| `p6b_serialize_body_still_skip_listed_progress` | First speed target still listed; IR tables in defs + `outTag`/`outSpNum` |
| `p6b_emit_accadd_hotpath_progress` | AccAdd recurse hotpath |
| `p6b_emit_if_hotpath_progress` | Early `emitIfStmt` + `patchRelJump` |
| `p6b_emit_symindex_map_progress` | `eConstMap` + `eLocalMap` / map-only `emitSym` |
| `p6b_parser_iterative_add_progress` | Iterative `+`/`-` + early `IDENT=` assign |
| `p6b_serialize_body_compile_budget` (ignored) | Timing probe for serialize_body |
| `p6_leaf_self_host_compile_budget` (ignored) | Timing probe for all five leaves |

Windows: `self_host_parser_suite` and `self_host_serialize_suite` use a 32 MiB thread stack.

## Seeds

| Seed | Source |
|------|--------|
| `emit_impl.kab.kbc` | `../emit_impl.kab` |
| `parser_impl.kab.kbc` | `../parser_impl.kab` |
| `lexer_impl.kab.kbc` | `../lexer_impl.kab` |
| `serialize_body.kab.kbc` | `../serialize_body.kab` |
| `vm_run_body.kab.kbc` | `../vm_run_body.kab` |

```bash
./scripts/regen_self_host_seeds.sh
# or single leaf via kabootar compile --rust + copy cache → seed/
```
