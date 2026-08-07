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
| `parser.kab` | klar | `parseTokens(tokens)` → AST (+ generics: `<T>`, enum/class, typed calls/members) |
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

1. **Fn-lokaler** — bytecode speglar lokaler på *aktuell* aktiveringsram (`set` / `share_bindings` för pile); captures markeras (`local_captures`) och får `assign` till parent. Rekursiva anrop får egna ramar (L1/L4). **Session-state** (`lxPos`, `eOps`, `pPos`, …) förblir modul-global; rekursions-temps migreras till fn-lokaler (S1).
2. **`push` returnerar ny array** — skriv `arr = push(arr, item)`, inte bara `push(arr, item)`.
3. **Spara AST-fält före rekursion** — t.ex. `eSym = eNode["sym"]` innan `emitExpr(init)`; `pCallee = pLeft` innan call-args.
4. **Bracket-access för AST-nycklar** — undvik `.then`, `.sym`, `.value` där det krockar; använd `node["sym"]`.
5. **Radbrytning** — `"\n"` är literal i Kabootar; använd `CHAR_NL` från `lexer_defs` i serializer.
6. **Assign: peek före bump** — `let tok = peek(); bump();` (inte `bump()`-returvärde) för att få `sym`.
7. **Modulskala (L2)** — ≥40 top-level `fn` per modul OK (`share_bindings` vid register/clone). Äldre gräns ~7/~14 fn var en host-bug, inte ett språkkrav. Dela fortfarande stora filer av läsbarhetsskäl.
8. **Exporterade fn + privata syskon** — Rust `refresh_function_closures` + `prepare_exported_bytecode_fn` (dela post-refresh closure).
9. **Nested import** — använd `import "self_host/compile"` + `compile(src)` för hela kedjan; `parse.kab` för AST-only. Importera inte `parser.kab` i samma modul som `parse.kab` (namnkrock).
10. **Emitter: CALL-args i fn-kropp** — undvik var+literal i samma 2-arg `CALL`. Använd modul-global `ZERO = 0` + `char_code_at(ch, ZERO)`.
11. **Windows stack** — `build.rs` sätter 16 MiB stack för `kabootar`-bin.
12. **Compare-parse** — spara lhs i `pSave` före rhs; använd **inte** `parsePostfix()` för compare-rhs (skriver över `pLeft`). Undvik `parsePostfix()` i `.kbc`-cache (privata fn syns inte). Inline rhs som `+`-loopen + `null`/`undefined`/`true`/`false`.
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
24. **Parser bracket index** — fn-lokal `indexObj` (S1; tidigare `pIndexObj`): `parseCompare()` i `a[b]` skriver över `pLeft`.
25. **Parser compare rhs** — `pInAddSub`-flagga: compare-rhs via `parseCompare()` i add/sub-läge (inte inline literal); annars `len(stack) - 1` lämnar `-` kvar och `while` får `Expected {`.
26. **Parser `+`/`-` rhs** — `pAddLeftStack` + rekursiv `parseCompare()` under `pInAddSub=1` (inte ident-shortcut; tappar `.field` efter `+`, t.ex. `throw "msg" + eNode.kind`).
27. **Parser && expr** — `pExprLeft` (inte `pSave`/`pBinOp`): `parseCompare()` skriver över båda under rhs-parse.
28. **Emitter binary op** — fn-lokaler `binOp`/`binRight` före rekursiv `emitExpr` (S1; tidigare `eBinOpStack`).
29. **pub fn exports** — `isPub` i AST, `eExports` i emit, `exports=` i serialize.
30. **Emitter let/member** — `eStoreSym`/`eMemberFldStack` före rekursiv `emitExpr` (inte `eSym`/`eMemberFld`; clobbar sym/field). **`eAssignSym`** före `emitExpr(rhs)` på assign/let. **`eExprStmt`** på `AST_EXPR` (inte `eBxL`; clobras av call/member/index).
31. **Emitter module globals in fn** — `let lxPos` på modulnivå delas mellan fn vid interpret; i bytecode ska `emitLoadSym`/`emitStoreSym` leta i `eFnLocals` först, sedan `eGlobals` (inte `localIndex` på assign till modul-global).
32. **Emitter fn snapshot** — `snapArr(eFnOps)` (och params/locals/globals) vid push till `eFunctions`.
33. **Emitter block loop** — `eBlockIStack`/`eBlockNStack`; efter `emitStmt` läs `eBlockI = eBlockIStack[…] + 1` (inte `eBlockI + 1`).
34. **Emitter expr-loops** — object/array/call-arg med egna index-stackar (`eObjIStack`, `eArrIStack`, `eCallArgIStack`); samma pop/push-mönster som block. Extra top-level fn är OK efter L2 (tidigare OOM ~14 fn).
35. **Parser parseTokens EOF** — `while pDone == 0` (inte `while true`/`break` i bytecode); dubbelkolla `pPos >= len(pToks)` och `pTok.type == "EOF"` före `parseStmt()`. `parseStmt()` returnerar `null` vid EOF; `parseTokens` pushar bara när `pVal != null`.
36. **Emitter binary `+` i fn** — `emitExpr(AST_BINARY)` spara rhs i `eBxR` före `emitExpr(left)`; alltid rekursiv emit (ingen `eInFn`-genväg).
37. **Parser let sym** — `pLetSym = symCopy(pTok.value)` före `bump()` på let-ident; använd `pLetSym` i `pSymPool` (inte `pTok.value` efter bump; clobbras av postfix/index/call-parse).
38. **Parser undefined literal** — `TOKEN_UNDEFINED` i `parsePostfix` → `LIT_UNDEF` (emit.kab jämför `== undefined` / `!= undefined`).
39. **Parser postfix chains** — interleaved `()`, `.`, `[]` i en loop (inte tre separata while; annars tappas `obj["x"].field`).
40. **`null` vs `undefined`** — båda är förstklassiga i lexer/parser/bytecode. `null == undefined` är `false`. Saknad nyckel / oinitierad `let` → `undefined`; medveten tomhet → `null`. Self-host: `if node.kind == undefined`, `if obj["field"] != undefined` — **inte** `null` i dessa fall.
41. **Program body** — samma block-stack-loop som `AST_BLOCK` + `OP_HALT`.
42. **Parser index assign** — `arr[i] = rhs` → `AST_INDEX_ASSIGN` + `OP_INDEX_SET` (inte `parseExpr` + kvarlämnat `=`; emit.kab patchar `eFnOps[eJmp] = { … }`).
43. **Emitter index assign** — spara `eBxRhs = eNode["rhs"]` före `emitExpr(eBxL)`/`emitExpr(eBxR)`; `emitExpr` clobbrar `eNode` (annars `eNode["rhs"]` läser index-noden → member access-fel).
44. **Emitter popStack** — använd native `pop(stack)` (inte manuell while-kopia); self-host compile av emit.kab är annars extremt långsam.
45. **Self-compiled vs Rust emit** — `import "self_host/emit"` = Rust-bytecode (~långsam men klar). `compile(emit.kab)` → `.kbc` = self-hosted bytecode; om `emit(parse("let x = 1"))` hänger via `.kbc` men import fungerar → felsök serialize/compile-output, inte bara emit.kab-logik.
46. **Self-host nested builtins** — `push(stack, len(x))` kompileras fel (yttre anrop blir `len`). Använd `pushLen(stack, arr)` eller spara `eLenScratch = len(x)` före `push`.
47. **Serialize radbrytning** — använd `CHAR_NL` från `lexer_defs`, **inte** `"\n"` (literal i Kabootar); annars blir `.kbc` en enda rad som Rust `deserialize` avvisar.
48. **Emitter nested call** — `eCalleeStack` före rekursiv `emitExpr` på args (inte modul-global `eCallee`; nästlade `serialize_bc(emit(parse(x)))` laddar fel callee). Binary temps är fn-lokala (S1).
49. **Parser nested call** — fn-lokaler `savedCallee`/`savedTypeArgs` (S1; tidigare `pCalleeStack`).
50. **Parser generic call type args** — spara `savedTypeArgs` med call (S1); rekursiv `parseCompare` nollställer modul-global `pTypeArgs`.
51. **Emit generic fn** — spara template i `eGenericTemplates`; vid `AST_CALL` till generic callee: infer/mangle → specialisera → ersätt callee med `id$Number` (importera **inte** extra modul från `emit.kab` — kombinerad import overflowar compile).

## Nästa milstolpar

1. ~~`.kbc` roundtrip: `deserialize(serialize_bc(emit(ast)))` i Rust~~ ✅
2. ~~`fn`-anrop: `OP_CALL` mot self-hosted `functions[]`~~ ✅ (Rust `run_module`)
3. ~~`parse.kab`-facaden (nested `tokenize`)~~ ✅
4. ~~Full pipeline: `compile(source)` entrypoint~~ ✅
5. ~~Self-host bootstrap: `compile.kab` cache + `compile(sample)` -> Rust `run_module`~~ ✅
6. ~~Utöka self-hosted språksubset (obj, &&, compares, index)~~ ✅
7. ~~Lexer-like compile (`char_at`-loop, `!=`, `continue`/`break`/`undefined`)~~ ✅
8. ~~Self-host hela `lexer.kab` via `compile()`~~ ✅ — `self_host_lexer_full_compile_and_run` (~2.5 h)

9. ~~Self-host `parser.kab` / `emit.kab` (större moduler, fler opcodes).~~ ✅
   - **parser.kab** (~960 rader, 9 fn): generics (`<T>` på fn/class/enum, type args på call/member), `self_host_parser_suite` via Rust bytecode-preload (undviker Windows OOM). `self_host_parser_full_compile_and_run` (~2.5 h) verifierar `compile(parser.kab)` → `parseTokens(tokenize("let x = 1"))`.
   - **emit.kab** (~850 rader, 8 fn): redo för vidare opcode-stöd om parsern utökas (`||`, unary `!`, `*`, assign till index, etc.).
    - **Verifiering:** snabb: `self_host_emit_suite` (3 subprocess-chunks: core / generics / calls — undviker Windows OOM), `self_host_parser_full_compile_smoke`, `self_host_emit_full_compile_smoke`; långsam: `self_host_parser_full_compile_and_run` (ignored, ~2.5 h).

10. ~~Self-host hela `emit.kab` via `compile()` → kör `emit(parse("let x = 1"))` i bytecode~~ ✅
    - Snabb smoke: `self_host_emit_full_compile_smoke`.
    - Långsam CI: `self_host_emit_full_compile_and_run` (ignored, ~2–3 h).
    - Run-only: `self_host_emit_kbc_run_only` (kräver `_emit_full_out.kbc`).

11. ~~Self-host hela `serialize.kab` via `compile()` → kör `serialize_bc(emit(parse(...)))` + roundtrip~~ ✅
    - Snabb smoke: `self_host_serialize_full_compile_smoke`.
    - Långsam CI: `self_host_serialize_full_compile_and_run` (ignored, ~40 min).
    - Run-only: `self_host_serialize_kbc_run_only` (kräver `_serialize_full_out.kbc`).
    - Bygg KBC: `python scripts/profile_emit_compile.py compile serialize.kab`
    - **Kör tunga tester med `--test-threads=1`** (parallella serialize-tester kan OOM:a på Windows).

12. ~~True bootstrap — `compile(compile.kab)` körs som self-hosted bytecode och kan `compile(sample)`~~ ✅
    - Snabb smoke: `self_host_compile_full_compile_smoke` (subprocess — djup pipeline overflowar test-stack).
    - Långsam CI: `self_host_compile_full_compile_and_run` (ignored, ~3 min för compile.kab).
    - Run-only: `self_host_compile_kbc_run_only` (kräver `_compile_full_out.kbc`).
    - Bygg KBC: `python scripts/profile_emit_compile.py compile compile.kab`

13. ~~**Generics (språk):** Rust v1 + self-host G4~~ ✅ — `fn id<T>`, monomorphisering, `tests/generics.rs`, `test_parser.kab` / `test_emit.kab`. Design: [docs/GENERICS.md](../docs/GENERICS.md). **Struct planeras inte.** Semikolon förblir valfria (ev. framtida breaking change).

14. **Generics fas 2 (G6–G11):** ~~G6 inferens~~ ✅, ~~G7 klassmetoder~~ ✅, ~~G8 klasser~~ ✅, ~~G9 enum~~ ✅, ~~G10 self-host~~ ✅, ~~G11 LSP~~ ✅. Plan: [docs/GENERICS.md#fas-2--g6-planering](../docs/GENERICS.md#fas-2--g6-planering), roadmap **Våg F** i [docs/ROADMAP.md](../docs/ROADMAP.md).

## Profilering (compile-tid)

Efter grön `emit` full compile — hitta flaskhalsar innan M11/M12.

```bash
# Fas-tid: parse / emit / serialize (emit.kab, kan ta timmar)
python scripts/profile_emit_compile.py phases emit.kab

# P6b leaf (minsta skip-listade källan) — samma pipeline
python scripts/profile_emit_compile.py phases self_host/serialize_body.kab

# Wall-time compile() end-to-end
python scripts/profile_emit_compile.py compile emit.kab

# Prefix-skala: vilka radintervall dominerar
python scripts/profile_emit_compile.py bisect emit

# Jämför lexer / parser / emit
python scripts/profile_emit_compile.py compare

# Run-fas (kräver _emit_full_out.kbc)
CARGO_TARGET_DIR=target-alt3 cargo test --test self_host self_host_emit_profile_run_phases -- --ignored --nocapture
```

Output-rader `PROFILE ...` är maskinläsbara. `popStack()` och stack-trim-loopar använder nu native `pop()` (kräver ny `compile(emit.kab)` för `.kbc`).

Snabb smoke: `cargo test --test self_host self_host_profile_phases_smoke`.

### P6b (skip-list → tom lista)

Se [seed/README.md](seed/README.md) för policy, playbook, **fas-profil** och baslinjer.

- Produktpath = committed seeds; **töm inte** listan förrän alla fem löv
  `compile_source_self_host` < 10 s (`P6_SELF_HOST_LEAF_CI_FAST_MS`).
- Fas-profil (tiny if/+): parse ≈ emit ≫ serialize. Landade cuts: `symIndex` maps,
  iterative `+`/`-` i `parser_impl`, `eOpsN` jump patches i `emit_impl`,
  **IR membership i `serialize_defs`**, **`outTag`/`outSpNum`/`sLine` AccAdd** i `serialize_body`.
- Leaf densify + toolchain cuts: `serialize_body` **~689 s** debug (IR hoist + AccAdd; prior ~670 s) — still ≫ 10 s, **skip-list stays**.
- Efter `emit_impl` / `parser_impl` / `serialize_body`-ändring: regenerera motsvarande `self_host/seed/*.kbc`.
