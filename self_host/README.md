# Self-hosted Kabootar compiler

Kabootar kompilerar sig själv steg för steg. Varje fas speglar motsvarande Rust-modul.

## Kedja

```
source text
    → lexer.kab        (src/lexer.rs)     token[]
    → parser.kab       (src/parser.rs)    AST
    → emit.kab         (src/bytecode/compiler.rs)  opcode IR
    → [opt.kab]        (src/runtime/kv8/opt.rs)
    → .kbc bytecode
    → kabootar compile self_host/…   (full self-host)
```

## Filer

| Fil | Status | Beskrivning |
|-----|--------|-------------|
| `lexer_defs.kab` | klar | Token-konstanter och `KEYWORDS` |
| `lexer.kab` | klar | `tokenize(source)` → token-array |
| `ast_defs.kab` | klar | AST-nodtyper |
| `parser.kab` | klar | `parseTokens(tokens)` → AST (let/assign/if/while/fn/+/==/calls) |
| `emit_defs.kab` | klar | Opcode-namn för IR |
| `emit.kab` | klar | `emit(ast)` → opcode IR |
| `parse.kab` | klar | `parse(source)` facade (lexer + parser) |
| `test_lexer.kab` | klar | 234+ lexer-tester |
| `test_parser.kab` | klar | Parser-tester |
| `test_emit.kab` | klar | Emitter-tester |
| `test_tiny.kab` | klar | Snabb smoke |

## Kör tester

```bash
kabootar self_host/test_lexer.kab
kabootar self_host/test_parser.kab
kabootar self_host/test_emit.kab
kabootar self_host/test_tiny.kab
cargo test --test self_host
```

## Designregler (lärt från lexern)

1. **Modul-global state** — `let lxPos`, `let pPos`, `let pLeft`, `let eNode` på modulnivå. Kabootar har **inga re-entranta fn-lokaler**; rekursiva parser/emitter-anrop skriver över `let` i samma funktion.
2. **`push` returnerar ny array** — skriv `arr = push(arr, item)`, inte bara `push(arr, item)`.
3. **Spara AST-fält före rekursion** — t.ex. `eSym = eNode["sym"]` innan `emitExpr(init)`; `pCallee = pLeft` innan call-args.
4. **Bracket-access för AST-nycklar** — undvik `.then`, `.sym`, `.value` där det krockar; använd `node["sym"]`.
5. **Assign: peek före bump** — `let tok = peek(); bump();` (inte `bump()`-returvärde) för att få `sym`.
6. **≤~7 fn per modul** — fler privata fn kan ge stack overflow vid modul-init.
7. **Exporterade fn + privata syskon** — Rust `refresh_function_closures` + `prepare_exported_bytecode_fn` (dela post-refresh closure).
8. **Nested import** — `parse(source)` via `parse.kab`; tester: `parseTokens(tokenize(src))`.

## Nästa milstolpar

1. Utöka parsern: fler operatorer, `fn`-body i emitter
2. Serialisera opcode IR till `.kbc`-textformat
3. `kabootar compile self_host/emit.kab` med self-hostad pipeline
