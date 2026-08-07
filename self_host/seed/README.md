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
import-time cost of dependents. Even a ~9 KB leaf can take **minutes** in debug
`cargo test` until emit/parse hot paths catch up.

## P6b playbook

1. **Densify the leaf source** (fewer If / Binary trees) — e.g. `serialize_body`
   `irOpLine` → `IR_WITH_ARG` / `IR_ZERO_ARG` membership (~8.8 KB).
2. **Speed the toolchain emit** — `symIndex` const/global **maps** (avoid O(C²)
   LoadGlobal clones); AccAdd recurse; early `emitIfStmt` + `patchRelJump`;
   regenerate **emit_impl** seed after edits.
3. **Measure** before flipping any flag:
   ```bash
   # Single leaf (serialize_body)
   cargo test --test self_host p6b_serialize_body_compile_budget -- --ignored --nocapture

   # All five skip-listed leaves
   cargo test --test self_host p6_leaf_self_host_compile_budget -- --ignored --nocapture

   # Phase split: parse / emit / serialize (needs kabootar bin)
   python scripts/profile_emit_compile.py phases self_host/serialize_body.kab
   ```
4. **Do not** empty the skip-list or set `P6B_EMPTY_SKIP_LIST_READY=true` until
   step 3 shows **all five** leaves `ok` and `ms ≤ 10000`.

### Measured baselines (debug `cargo test`, host VM)

| Leaf | Notes | Last recorded |
|------|-------|---------------|
| `serialize_body.kab` | densify + AccAdd/If + `symIndex` maps (in-place IndexSet) | **~964 s** (still ≫ 10 s; ~885 s before maps — leaf not map-bound yet) |
| others | Larger / denser | not under budget |

Maps target O(C²) `symIndex` clones on **large** const/global tables (bigger leaves /
self-hosting `emit_impl` itself). `serialize_body` remains ≫ budget; skip-list stays 5.
Fas-profil: `python scripts/profile_emit_compile.py phases self_host/serialize_body.kab`

## Gates

| Test | Role |
|------|------|
| `p6_seed_only_all_leaves_have_seeds` | Files exist; list length stays 5 |
| `p6_seed_fingerprint_all_leaves_load` | Seed deserializes; fingerprint matches source |
| `p6_skip_list_stays_until_ci_fast_gate` | Oversize emit stays skipped; flag off |
| `p6b_serialize_body_still_skip_listed_progress` | First speed target still listed + densified |
| `p6b_emit_accadd_hotpath_progress` | AccAdd recurse hotpath present in emit_impl |
| `p6b_emit_if_hotpath_progress` | Early `emitIfStmt` + `patchRelJump` present |
| `p6b_emit_symindex_map_progress` | `eConstMap` / `constKey` — no `len(eConsts)` scan |
| `p6b_serialize_body_compile_budget` (ignored) | Timing probe for serialize_body |
| `p6_leaf_self_host_compile_budget` (ignored) | Timing probe for all five leaves |

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
# or a single leaf:
KABOOTAR_COMPILE=rust "$BIN" compile self_host/emit_impl.kab --rust
# then copy .kabootar/cache/… → self_host/seed/… with source= rewritten
```

Requires a built `kabootar` binary (`CARGO_TARGET_DIR` / `KABOOTAR_BIN` optional).
