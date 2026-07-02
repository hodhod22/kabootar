# Self-hosted Kabootar compiler

Kabootar kompilerar sig själv steg för steg. Varje fas speglar motsvarande Rust-modul.

## Kedja

```
source text
    → lexer.kab        (src/lexer.rs)     token[]
    → parse.kab        (facade)           AST
    → emit.kab         (src/bytecode/compiler.rs)  opcode IR
    → serialize.kab    (src/bytecode/types.rs)     .kbc text
    → compile.kab      (facade)           source → .kbc
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
| `emit.kab` | klar | `emit(ast)` → opcode IR (+ `functions[]`) |
| `serialize_defs.kab` | klar | `.kbc`-header |
| `serialize.kab` | klar | `serialize_bc(ir)` → text |
| `parse.kab` | klar | `parse(source)` facade (lexer + parser) |
| `compile.kab` | klar | `compile(source)` → `.kbc` text (full pipeline) |
| `test_lexer.kab` | klar | 234+ lexer-tester |
| `test_parser.kab` | klar | Parser-tester |
| `test_emit.kab` | klar | Emitter-tester |
| `test_serialize.kab` | klar | Serializer-tester |
| `roundtrip_probe.kab` | klar | Kabootar roundtrip smoke |
| `roundtrip_main_probe.kab` | klar | Main program → Rust `deserialize` |
| `roundtrip_fn_probe.kab` | klar | Fn-body roundtrip (Rust CI) |
| `roundtrip_call_probe.kab` | klar | Fn call `add(1,2)` → Rust `run_module` |
| `test_parse_facade.kab` | klar | `parse(source)` facade-tester |
| `test_subset.kab` | klar | Utökad parser-subset |
| `mini_module.kab` | klar | Lexer-liknande mini-modul (Rust) |
| `larger_probe.kab` | klar | `compile(mini_module)` -> Rust CI |
| `test_larger.kab` | klar | Larger compile smoke |
| `sample.kab` | klar | Bootstrap-exempel (`return 42`) |
| `bootstrap_probe.kab` | klar | `compile(sample)` -> Rust `run_module` CI |
| `test_bootstrap.kab` | klar | Bootstrap smoke |
| `test_tiny.kab` | klar | Snabb smoke |

## Kör tester

```bash
kabootar self_host/test_lexer.kab
kabootar self_host/test_parser.kab
kabootar self_host/test_parse_facade.kab
kabootar self_host/test_subset.kab
kabootar self_host/test_larger.kab
kabootar self_host/test_compile.kab
kabootar self_host/test_bootstrap.kab
kabootar compile self_host/compile.kab
kabootar self_host/test_emit.kab
kabootar self_host/test_serialize.kab
kabootar self_host/test_tiny.kab
cargo test --test self_host
```

## Designregler (lärt från lexern)

1. **Modul-global state** — `let lxPos`, `let pPos`, `let pLeft`, `let eNode` på modulnivå. Kabootar har **inga re-entranta fn-lokaler**; rekursiva parser/emitter-anrop skriver över `let` i samma funktion.
2. **`push` returnerar ny array** — skriv `arr = push(arr, item)`, inte bara `push(arr, item)`.
3. **Spara AST-fält före rekursion** — t.ex. `eSym = eNode["sym"]` innan `emitExpr(init)`; `pCallee = pLeft` innan call-args.
4. **Bracket-access för AST-nycklar** — undvik `.then`, `.sym`, `.value` där det krockar; använd `node["sym"]`.
5. **Radbrytning** — `"\n"` är literal i Kabootar; använd `CHAR_NL` från `lexer_defs` i serializer.
6. **Assign: peek före bump** — `let tok = peek(); bump();` (inte `bump()`-returvärde) för att få `sym`.
7. **≤~7 fn per modul** — fler privata fn kan ge stack overflow vid modul-init.
8. **Exporterade fn + privata syskon** — Rust `refresh_function_closures` + `prepare_exported_bytecode_fn` (dela post-refresh closure).
9. **Nested import** — använd `import "self_host/compile"` + `compile(src)` för hela kedjan; `parse.kab` för AST-only. Importera inte `parser.kab` i samma modul som `parse.kab` (namnkrock).
10. **Emitter: CALL-args i fn-kropp** — undvik blandade var+literal i samma 2-arg `CALL` i fn-body (emit-hang). Använd t.ex. strängjämförelse i stället för `char_code_at(ch, 0)` tills fix finns.
11. **Windows stack** — `build.rs` sätter 16 MiB stack för `kabootar`-bin (djup self-host-kedja).

## Nästa milstolpar

1. ~~`.kbc` roundtrip: `deserialize(serialize_bc(emit(ast)))` i Rust~~ ✅
2. ~~`fn`-anrop: `OP_CALL` mot self-hosted `functions[]`~~ ✅ (Rust `run_module`)
3. ~~`parse.kab`-facaden (nested `tokenize`)~~ ✅
4. ~~Full pipeline: `compile(source)` entrypoint~~ ✅
5. ~~Self-host bootstrap: `compile.kab` cache + `compile(sample)` -> Rust `run_module`~~ ✅
6. ~~Utöka self-hosted språksubset (obj, &&, compares, index)~~ ✅
7. Self-host `lexer.kab` (fortsatt utökning: `continue`, `!=` chains, …)
