# Snabb kompilering

Kabootar har **AST-tolk**, **bytecode (.kbc)** och **self-host compile**.

## H6e — boot-policy i Kab

Produktpolicy för bootstrap: `import "kab/boot"`. **Nolltolerans:** ingen Rust-emit, ingen Rust-JIT, ingen Rust-GC — se ROADMAP-rutan.

**`kabootar run` / `run_file`:** self-host först. `KABOOTAR_COMPILE=rust` är **skuld** (ska bort), inte produktväg. Full Rust-fri plan: [ROADMAP.md — Kabootar på egna fötter](ROADMAP.md#kabootar-på-egna-fötter--noll-rust). Fart (JIT/IC/GC/AOT i Kab): [Våg FT](ROADMAP.md#våg-ft--fart-alla-tekniker-i-kab).

## `kabootar compile` (S2)

Default: **`self_host/compile.kab`**. Rust-fallback är **tillfällig** (stängs i SH16). `--rust` / `KABOOTAR_COMPILE=rust` ska försvinna för produktkällor. Efter `compile()`: `lastCompileMs(0)` = total ms (1 parse, 2 emit, 3 serialize).

```bash
kabootar compile main.kab              # self-host → rust fallback
kabootar compile main.kab --self-host  # endast self-host (fel om det misslyckas)
kabootar compile main.kab --rust       # tvinga Rust-host
KABOOTAR_COMPILE=rust kabootar compile main.kab
```

Cache: `.kabootar/cache/<path-with-__>.kbc` (cwd-relative, mtime + fingerprint + `source=`).
Sibling `.kbcb` is written on compile. Warm the tree: `warm_self_host_disk_cache` (Rust API) so the next process rust-loads shards from disk.
Basenames alone kolliderar (`self_host/lexer` vs `lib/kv8/lexer`) — undvik gamla platta `lexer.kab.kbc`.
`.kbc`-strängkonstanter escapar `\n`/`\r`/`\t`/`\s` så whitespace inte bryter radformatet.

**H6e seeds:** committed bytecode under `self_host/seed/` + packed `compiler.kbcb` (fingerprint). **P6b:** skip-list tom. Dirty toolchain: `compile_dirty_dag_seeds`. Produktträd: `compile_dirty_product_tree`. Gates: `tests/sh_wave.rs`.

Self-host full compile av heavy leaves kan ta **minuter–timmar** — undvik i varje edit-loop; facader + seeds är CI-vägen.

## P10 — self-host pipeline (inte mer parser-isolering)

Parser-shards (~4–4.5 s, 123/123 tester) är **tillräckliga**. Nästa milstolpe är hela kedjan och VM-hotpath (se ROADMAP **P10**).

1. **Mät** lexer / parse / emit / serialize / deserialize / VM / total — `cargo test --test perf_p10_pipeline`
2. **LoadMember shape IC** (ptr + key-hash + cached value) + **CALL_0 / direct bytecode IC**
3. **Text `.kbc`** med `with_capacity`; **`.kbcb`** cache-envelope (`KBCB` + v1 payload) bredvid `.kbc`

Mål: self-host compile **10 → 7 → 5 → 3 s**, inte postfix 4.5 → 3.5 s. **P10 stängd** (a–h + j; i skippad). Unbox/JIT är **P11+**. **Självständighet/stabilitet i compiler-DAG:en är Våg SH** ([ROADMAP.md](ROADMAP.md) § SH).

---

## Rekommenderat arbetsflöde

| Vad du ändrar | Snabb verifiering |
|---------------|-------------------|
| Rust (`src/`) | `cargo test --test generics` eller riktad `cargo test fn_name` |
| `self_host/*.kab` | `cargo run -- run self_host/test_emit.kab` (interpreter) |
| LSP | `cargo test --lib language::hover` |
| Serialize format | `cargo test --test self_host self_host_serialize_suite` |

**Regel:** interpreter (`run_file`) för utveckling; full `compile(.kab)` endast vid milestone.

---

## Incremental Rust build

```bash
cargo test --test generics --no-run
```

Bygger bara det som behövs. Använd `--lib` för snabbare lib-only builds.

```bash
cargo build --bin kabootar
```

Undvik `cargo clean` om du inte måste — cold build tar flera minuter på Windows.

---

## Self-host compile-profiler

```bash
python scripts/profile_emit_compile.py compile emit.kab
```

Profilerar compile-faser. Använd för att hitta flaskhalsar — **inte** i varje PR.

---

## `.kbc`-cache

- Rust `compile::load_program_for_file` cachar parse/bytecode under `.kabootar/cache/<cwd-rel-path>.kbc` (`/` → `__`)
- Ogiltigförklaras när källfilens mtime är **nyare**, fingerprint ändras, eller `source=` inte matchar sökvägen
- Self-host output: `_emit_full_out.kbc`, `_serialize_full_out.kbc` i repo root
- **Stale guard:** `self_host_emit_kbc_freshness_guard` / `assert_fresh_serialize_kbc` i `tests/self_host.rs`

**Kv8:** efter ändring i `lib/kv8/eval.kab`, radera gammal cache om while/import beter sig konstigt:

```bash
rm .kabootar/cache/lib__kv8__eval.kab.kbc
# (äldre platta namn:) rm .kabootar/cache/eval.kab.kbc
```

## Modul-export-cache (process)

`import "kv8/eval"` kompilerar och kör modulen en gång per process; upprepade importer kopierar exporterade bindings från minnescache (`src/modules/mod.rs`, nyckel = filsökväg + mtime). Detta gör `kv8_lib_slow` snabbare när flera tester importerar samma kedja.

Ogiltigförklaras automatiskt när `.kab`-källan ändras (mtime).

Regenerera efter opcode-ändringar:

```bash
cargo test --test self_host self_host_emit_full_compile_and_run -- --ignored --test-threads=1
```

---

## Optimeringar (pågående / planerat)

| Optimering | Status |
|------------|--------|
| Parse-cache per fil | ✅ Rust |
| Bytecode VM vs AST eval | ✅ default bytecode |
| Modul-export-cache (import) | ✅ `modules/mod.rs` |
| Self-host: undvik megabundles i tester | ✅ regel i `.cursor/rules` |
| Kv8: parse-only CI för React bundle | ✅ `#[ignore]` full eval |
| Parallel `cargo test --test-threads=4` | ✅ JS parity |
| Self-host compile i Kabootar (M12) | ✅ subset |
| Incremental self-host emit | ✅ subset — `.kbc` `fingerprint=` (source hash + import mtimes) |

---

## Kv8 / React — minimal CI

```bash
cargo test --test kv8_lib -- --test-threads=1
```

Snabb suite: lexer + parser (~1–2 min).

```bash
cargo test --test kv8_lib_slow -- --test-threads=1
```

Tung suite: eval + dom (~5–10 min). Kör **inte** slow i varje IDE-klick.

Alternativ scripts:

```bash
bash scripts/kv8-test-fast.sh
bash scripts/kv8-test-slow.sh
```

---

## Se även

- [VSCODE_TESTS.md](VSCODE_TESTS.md)
- [self_host/README.md](../self_host/README.md)
- [ROADMAP.md](ROADMAP.md) — Våg G compile-opt
