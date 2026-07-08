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
| `parser.kab` | klar | `parseTokens(tokens)` → AST (let/assign/if/while/fn/obj/array/member/index/+/==/&&/calls) |
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
| `test_m7.kab` | klar | `!=`-chains (parse smoke) |
| `test_lexer_compile.kab` | klar | Lexer-like loop compile smoke |
| `lexer_compile_probe.kab` | klar | Lexer loop -> Rust `run_module` CI |
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
kabootar self_host/test_m7.kab
kabootar self_host/test_lexer_compile.kab
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
7. **≤~7 fn per modul** — fler privata fn kan ge stack overflow vid modul-init. **`lexer.kab`: max ~4 fn** (endast `lxScan` + exports); dela inte upp i `lxChar`/`lxPlus`/… som egna fn.
8. **Exporterade fn + privata syskon** — Rust `refresh_function_closures` + `prepare_exported_bytecode_fn` (dela post-refresh closure).
9. **Nested import** — använd `import "self_host/compile"` + `compile(src)` för hela kedjan; `parse.kab` för AST-only. Importera inte `parser.kab` i samma modul som `parse.kab` (namnkrock).
10. **Emitter: CALL-args i fn-kropp** — undvik var+literal i samma 2-arg `CALL`. Använd modul-global `ZERO = 0` + `char_code_at(ch, ZERO)`.
11. **Windows stack** — `build.rs` sätter 16 MiB stack för `kabootar`-bin.
12. **Compare-parse** — spara lhs i `pSave` före rhs; använd **inte** `parsePostfix()` för compare-rhs (skriver över `pLeft`). Undvik `parsePostfix()` i `.kbc`-cache (privata fn syns inte). Inline rhs som `+`-loopen + `null`/`true`/`false`.
13. **Emitter while** — spara loop-head i `eWhileHead` (inte `eIdx`). Jump-args är **relativa** i VM: `target - jmpIndex - 1`.
14. **Bytecode-cache** — `.kabootar/cache/*.kbc` ogiltigförklaras när källan är nyare (`read_bytecode_cache` mtime-check).
15. **Serialize från `.kbc`** — undvik privata fn-anrop från exporterade fn (`serialize_bc`); använd modul-global `sOut` + `CHAR_NL` inline istället för `appendLine()`.
16. **Array literal** — `[]` / `[a, b]` kräver `AST_ARRAY` + `make_array` i parser/emit/serialize (lexer.kab använder `let parts = []`).
17. **Emitter scratch** — `eBxL`/`eBxR` för CALL/INDEX/MEMBER/BINARY; `eList` för OBJECT/ARRAY; `eBodyStmts` för BLOCK (inte `eLeft`).
18. **Throw** — `throw expr` som `AST_THROW` + `throw` opcode i parser/emit/serialize.
19. **Emitter nested if/while** — `eIfJmpStack`/`eIfSkipStack` för jump-patch (inte modul-global `eJmp`; nästlade `if` skrev över den). För `break`: trimma `eBreakIdxs` tillbaka till `eBf` när en loop är färdigpatchad, annars läcker inner-loop breaks till outer-loop.
20. **Parser sym snapshot** — `symCopy()` + `pFnSym`/`pFnPub`; spara före rekursiv `parseStmt` (token/sträng-alias + modul-global `pSaveSym`).
21. **Parser while/if cond** — `pCondStack`: spara `pCond` efter `parseExpr()` före body/then/else (annars skriver sista inner `if` över `while`-villkoret).
22. **Parser let/assign sym** — `pBindSym` (inte `pSaveSym`): objektnycklar i rhs skriver över `pSaveSym` innan `return` (t.ex. `tokens = push(tokens, { column: lxCol })`).
23. **Parser assign lookahead** — säker `ident =`-lookahead via `pNextTok = pToks[pPos+1]` med explicit EOF-fallback; undviker både OOB och att clobbra `pTok` före expr-stmts.
24. **Parser bracket index** — `pIndexObj` (inte `pLeft`): `parseCompare()` i `a[b]` skriver över `pLeft` (t.ex. `KEYWORDS[id]` blev `id[id]`).
25. **Parser compare rhs** — `pInAddSub`-flagga: compare-rhs via `parseCompare()` i add/sub-läge (inte inline literal); annars `len(stack) - 1` lämnar `-` kvar och `while` får `Expected {`.
26. **Parser && expr** — `pExprLeft` (inte `pSave`/`pBinOp`): `parseCompare()` skriver över båda under rhs-parse.
27. **Emitter binary op** — `eBinOpStack` + `eBinRStack` före rekursiv `emitExpr` (inte `eOp`/`eBxR`; clobbar `&&` och rhs).
28. **pub fn exports** — `isPub` i AST, `eExports` i emit, `exports=` i serialize.
29. **Emitter let/member** — `eStoreSym`/`eMemberFldStack` före rekursiv `emitExpr` (inte `eSym`/`eMemberFld`; clobbar sym/field). **`eAssignSym`** före `emitExpr(rhs)` på assign/let. **`eExprStmt`** på `AST_EXPR` (inte `eBxL`; clobras av call/member/index).
30. **Emitter module globals in fn** — `let lxPos` på modulnivå delas mellan fn vid interpret; i bytecode ska `emitLoadSym`/`emitStoreSym` leta i `eFnLocals` först, sedan `eGlobals` (inte `localIndex` på assign till modul-global).
31. **Emitter fn snapshot** — `snapArr(eFnOps)` (och params/locals/globals) vid push till `eFunctions`.
32. **Emitter block loop** — `eBlockIStack`/`eBlockNStack`; efter `emitStmt` läs `eBlockI = eBlockIStack[…] + 1` (inte `eBlockI + 1`).
33. **Emitter expr-loops** — object/array/call-arg med egna index-stackar (`eObjIStack`, `eArrIStack`, `eCallArgIStack`); samma pop/push-mönster som block. **Inga extra top-level fn** (Kabootar OOM vid ~14 fn/modul).
34. **Program body** — samma block-stack-loop som `AST_BLOCK` + `OP_HALT`.

## Nästa milstolpar

1. ~~`.kbc` roundtrip: `deserialize(serialize_bc(emit(ast)))` i Rust~~ ✅
2. ~~`fn`-anrop: `OP_CALL` mot self-hosted `functions[]`~~ ✅ (Rust `run_module`)
3. ~~`parse.kab`-facaden (nested `tokenize`)~~ ✅
4. ~~Full pipeline: `compile(source)` entrypoint~~ ✅
5. ~~Self-host bootstrap: `compile.kab` cache + `compile(sample)` -> Rust `run_module`~~ ✅
6. ~~Utöka self-hosted språksubset (obj, &&, compares, index)~~ ✅
7. ~~Lexer-like compile (`char_at`-loop, `!=`, `continue`/`break`/`undefined`)~~ ✅
8. ~~Self-host hela `lexer.kab` via `compile()`~~ ✅ — `self_host_lexer_full_compile_and_run` (~2.5 h)

9. **Pågår:** Self-host `parser.kab` / `emit.kab` (större moduler, fler opcodes).
   - **parser.kab** (~650 rader, 8 fn): interpreter-varianten och self-hostade parser-tester är gröna; full self-hostad `compile(parser.kab)` kompilerar, men runtime-smoket `parseTokens(tokenize("let x = 1"))` via bytecode har kvar en känd EOF/`+`-bugg (`Cannot add Object EOF and Object EOF`).
   - **emit.kab** (~880 rader, 8 fn): redo för vidare opcode-stöd om parsern utökas (`||`, unary `!`, `*`, assign till index, etc.).
   - **Verifiering:** snabb: `self_host_parser_suite`, `self_host_parser_full_compile_smoke`, `self_host_emit_full_compile_smoke`; långsam: `self_host_parser_full_compile_and_run` (ignored, ~2 h) för full self-hostad parser + runtime.
