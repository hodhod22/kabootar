# Self-hosted Kabootar compiler

Kabootar kompilerar sig själv steg för steg. Varje fas speglar motsvarande Rust-modul.

## Kedja

```
source text
    → lexer.kab        (src/lexer.rs)     token[]
    → parser.kab       (src/parser.rs)    AST
    → [opt.kab]        (src/runtime/kv8/opt.rs)
    → [emit.kab]       (src/bytecode/compiler.rs)
    → .kbc bytecode
    → kabootar compile self_host/…   (full self-host)
```

## Filer

| Fil | Status | Beskrivning |
|-----|--------|-------------|
| `lexer_defs.kab` | klar | Token-konstanter och `KEYWORDS` |
| `lexer.kab` | klar | `tokenize(source)` → token-array |
| `ast_defs.kab` | klar | AST-nodtyper |
| `parser.kab` | pågår | `parseTokens(tokens)` → AST (subset) |
| `parse.kab` | klar | `parse(source)` facade (lexer + parser) |
| `test_lexer.kab` | klar | 223+ lexer-tester |
| `test_parser.kab` | klar | Parser-tester |
| `test_tiny.kab` | klar | Snabb smoke |

## Kör tester

```bash
kabootar self_host/test_lexer.kab
kabootar self_host/test_parser.kab
kabootar self_host/test_tiny.kab
cargo test --test self_host
```

## Designregler (lärt från lexern)

1. **Modul-global state** — `let lxPos`, `let pPos` på modulnivå; mutera inte via parametrar.
2. **`push` returnerar ny array** — skriv `arr = push(arr, item)`, inte bara `push(arr, item)`.
3. **Exporterade fn + privata syskon** — Rust `prepare_exported_bytecode_fn` + delad closure krävs.
4. **Nested import + private fn** — `tokenize` → `lxScan` via `import "lexer"` in parser fails; use `parse.kab` facade or `parseTokens(tokenize(src))` in tests.

## Nästa milstolpar

1. Utöka parsern: `if`/`while`/`fn`/assign
2. AST → bytecode emitter i Kabootar
3. `kabootar compile self_host/parser.kab` med self-hostad pipeline
