# Snabb kompilering

Kabootar har **AST-tolk**, **bytecode (.kbc)** och **self-host compile**. Self-host full compile av `emit.kab` kan ta **30–90 min** — undvik att köra det i varje edit-loop.

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

- Rust `compile::load_program_for_file` cachar parse/bytecode per fil under `.kabootar/cache/*.kbc`
- Ogiltigförklaras när källfilens mtime är **nyare** än cache-filen
- Self-host output: `_emit_full_out.kbc`, `_serialize_full_out.kbc` i repo root
- **Stale guard:** `self_host_emit_kbc_freshness_guard` / `assert_fresh_serialize_kbc` i `tests/self_host.rs`

**Kv8:** efter ändring i `lib/kv8/eval.kab`, radera gammal cache om while/import beter sig konstigt:

```bash
rm .kabootar/cache/eval.kab.kbc
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
