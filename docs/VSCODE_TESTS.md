# Tester i VS Code / Cursor

Kabootar-projektet använder **Rust `cargo test`** för motor-tester och **`.kab`-suiter** för self-host. Extension: [IDE.md](IDE.md).

---

## Förutsättningar

1. Rust toolchain (`cargo`, `rustc`)
2. VS Code/Cursor med Kabootar-extension (syntax + LSP)
3. Terminal i workspace root: `nova-interpreter/`

---

## Snabb smoke (under 2 min)

Öppna terminal (**Terminal → New Terminal**) och kör:

```bash
cargo test --lib hover_ -- --nocapture
```

LSP/generics-hover — ska visa ~22 passed.

```bash
cargo test --test generics -- --nocapture
```

Generics Rust-suite — 24 tester.

```bash
cargo test --test stdlib_wave -- --nocapture
```

Nya stdlib-tester (Math hyperbolic + string match).

```bash
cargo test --test kdom_lib -- --nocapture --test-threads=1
```

kDOM/KSS Kabootar-moduler (`lib/kdom`, `lib/kstyle`) — 8 tester.

VS Code / terminal (från repo root, `kabootar` finns inte i PATH förrän du byggt/installerat):

```bash
cargo run --bin kabootar -- examples/kdom_smoke.kab
cargo run --bin kabootar -- examples/kstyle_parse_smoke.kab
cargo run --bin kabootar -- examples/kv8_lexer_smoke.kab
cargo test --test kv8_lib -- --test-threads=1
cargo test --test kv8_lib_slow -- --test-threads=1
cargo run --bin kabootar -- examples/kv8_parser_smoke.kab
cargo run --bin kabootar -- examples/kv8_eval_smoke.kab
cargo run --bin kabootar -- examples/kv8_dom_smoke.kab
cargo test --test os_lib -- --test-threads=1
cargo run --bin kabootar -- examples/os_smoke.kab
cargo run --bin kabootar -- examples/os_async_smoke.kab
```

Valfritt: lägg `target/debug` (eller `target/release`) i PATH, eller `cargo install --path .` en gång.

---

## JS-paritet (5–15 min)

```bash
cargo test --test kabootar_js_parity for_of -- --nocapture
```

Verifierar `for x of […]` och iterator-varianter.

```bash
cargo test --test kabootar_js_parity -- --nocapture --test-threads=4
```

Full JS-paritetssvit (kan ta längre tid).

---

## Self-host (långsam — kör selektivt)

Snabb emit-regression (~1s + ~3.5 min):

```bash
cargo test --test self_host self_host_emit_compiles self_host_emit_rust_run_module_smoke -- --test-threads=1
cargo test --test self_host self_host_emit_rust_compile_run_smoke -- --test-threads=1
```

Första emit-sektionen (tokenize → parse → emit, ~6 min):

```bash
cargo test --test self_host self_host_emit_first_section_smoke -- --test-threads=1
```

Full emit-suite (~2h på Windows, en subprocess per sektion):

```bash
cargo test --test self_host self_host_emit_suite -- --ignored --nocapture --test-threads=1
```

Kräver att `emit.kab` har <=7 top-level fn (annars OOM vid import). Varje sektion körs i egen `kabootar`-process.

```bash
cargo test --test self_host self_host_serialize_suite -- --test-threads=1
```

Serialize + `new_instance` / `classes[]`.

```bash
cargo test --test self_host self_host_parser_suite -- --test-threads=1
```

Parser-suite (~8 min).

### Ignorerade (full compile — timmar)

```bash
cargo test --test self_host -- --ignored --test-threads=1
```

Kräver `compile(emit.kab)` / `compile(serialize.kab)` — se [COMPILE.md](COMPILE.md).

---

## Kör en enskild `.kab`-fil

```bash
cargo run -- run self_host/test_emit.kab
```

Eller via CLI om installerad:

```bash
kabootar run self_host/test_emit.kab
```

---

## VS Code tasks (valfritt)

Skapa `.vscode/tasks.json`:

```json
{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "Kabootar: generics",
      "type": "shell",
      "command": "cargo test --test generics -- --nocapture",
      "group": "test",
      "problemMatcher": []
    },
    {
      "label": "Kabootar: LSP hover",
      "type": "shell",
      "command": "cargo test --lib hover_ -- --nocapture",
      "group": "test"
    },
    {
      "label": "Kabootar: self-host emit",
      "type": "shell",
      "command": "cargo test --test self_host self_host_emit_suite -- --test-threads=1",
      "group": "test"
    }
  ]
}
```

Kör via **Terminal → Run Task…**.

---

## Felsökning

| Problem | Åtgärd |
|---------|--------|
| LSP hover fungerar inte | `cargo build --features lsp --bin kabootar-lsp`; starta om språkserver |
| Self-host emit fail | Kör `cargo run -- run self_host/test_emit.kab` för stack trace |
| Compile timeout i IDE | Kör tunga tester i terminal, inte via Test Explorer |
| `.kbc` stale | Se [COMPILE.md](COMPILE.md) — regenerera med `compile(serialize.kab)` |
| **`kv8_lib` verkar hänga i VS Code** | Normalt **2–3 min per test utan output** (första `import "kv8/eval"` kompilerar modulkedjan). Kör med `--test-threads=1` (finns i `.vscode/settings.json`). Om det hänger **>10 min**: radera `.kabootar/cache/eval.kab.kbc` och bygg om — gammal cache kan innehålla trasig `evalSource`→`evalSourceWith`-kedja. Terminal: `rm .kabootar/cache/eval.kab.kbc` |
| **`LNK1104` / cannot open `kv8_lib_slow*.exe`** | En **hängande testprocess** låser exe (vanligt efter avbruten slow-körning). Stäng Test Explorer-körningen, döda processer, bygg om: `taskkill //F //IM kv8_lib_slow-0f5b08c27dd34a7d.exe` (Windows), `rm target/debug/deps/kv8_lib_slow*.exe`, `cargo test --test kv8_lib_slow --no-run`. Kör **inte** fast + slow parallellt i IDE. |

---

## CI-liknande full körning

```bash
cargo test --test generics --test kabootar_js_parity --lib language:: -- --test-threads=4
```

Self-host lämnas till `--ignored` i CI eller nattlig pipeline.
