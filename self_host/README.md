# Self-hosted Kabootar compiler

**Slutmål (nolltolerans):** hela stacken är `.kab`. Rust är skuld tills [SH28](../docs/ROADMAP.md#kabootar-på-egna-fötter--noll-rust). Plan: [docs/ROADMAP.md — Kabootar på egna fötter](../docs/ROADMAP.md#kabootar-på-egna-fötter--noll-rust).

Produktkompilatorn är `self_host/compile.kab`. Plan: **[docs/ROADMAP.md — Våg SH](../docs/ROADMAP.md)**. Nästa kod: reverse-densify i `.kab`, inte `src/`.

## Kedja

```
source text
    → tokenizeExec / parseTokensExec   AST
    → emitMainExec                     opcode IR
    → serSerializeBc                   .kbc text
    → compile.kab                      source → .kbc
    → seed/compiler.kbcb               SH1 packed image
```

Default CLI: `kabootar compile` → self-host först. App-`.kab` har **ingen** Rust-fallback (SH16); `KABOOTAR_COMPILE=rust` / `--rust` **felar** för appar. `self_host/` DAG får rust-seeds. `bootPolicy("prefer")` = `self-host-only`.

## Nuläge (inte den gamla shard-listan)

| Yta | Status |
|-----|--------|
| Skip-list | **tom** (`attempt-all`, P6b) |
| Compile-DAG | **&lt; 80** `.kab` (SH5); `vm_*` **&lt; 40** (SH6) |
| Image | `self_host/seed/compiler.kbcb` + `seed/dag/*.kbc` (SH1) |
| Facader | `pub let` alias, inte wrapping `pub fn` (SH3b) |
| Lexer | per-call `sess` i `tokenizeExec` |
| Parser/emit | **SH2/SH13:** återanvänd `gSess`/`gE` + `pResetSession`/`eResetSession`; tramp 0-arg. `pCondStack` på sess |
| Dirty seeds | `compile_dirty_dag_seeds()` loggar `dirty=N` (SH7) |
| Produktträd | `compile_dirty_product_tree(entry)` (SH7b) |
| Tiny parse | `sh8_tiny_parse_via_compiler_image` i CI; full `compile("return 1")` ignored i debug |
| Cache | SH15 content-addressed `cache/ca/v{image}_{fp}.kbcb` + mmap |

Tunga `_*probe*` / `_bisect*` är **inte** produkt. Regenerera image: `KABOOTAR_SH1_WARM=1 cargo test --test sh_wave sh1_warm -- --ignored`.

## Filer (ingångar)

| Fil | Roll |
|-----|------|
| `compile.kab` | `compile(source)` / `compileIr` |
| `parse.kab` | `parse` = tokenizeExec + parseTokensExec |
| `lexer.kab` / `parser.kab` / `emit.kab` / `serialize.kab` | tunna `pub let`-facader |
| `parser_exec.kab` / `emit_exec.kab` | per-call session + tramp |
| `ownership.kab` | O5 `@manual` |
| `vm.kab` | kab-only VM (alias till `vm_run_exec_core`) |
| `seed/compiler.kbcb` | packed compile-DAG |

## Tester

```bash
cargo test --test sh_wave -- --test-threads=1
cargo test --test self_host -- --test-threads=1
kabootar self_host/test_tiny.kab
kabootar compile self_host/sample.kab
```

## Designregler

**SH2:** parser/emit-cursors (`pPos`, `eOps`, …) ligger på **session-objektet**, inte nya modul-globaler. Trampolin: `sess["tramp"](sess)` så rekursion inte fångar en modul-`sess`. Nested `if`/`while` använder `pCondStack` / `eIfJmpStack` **på sess**. Nested named `fn` i en funktion: `emitNestedNamedFn` (save/restore `eFnOps`, `MakeArrow` + lokal).

1. **Fn-lokaler** — bytecode speglar lokaler på *aktuell* aktiveringsram. Captures: `local_captures`. **Lexer-ident:** `let cd`/`ok`/`start` i samma fn som loopen (`lxScanIdent`) — saknad `let` blir bytecode-global (`Undefined variable: cd`).
2. **`push` returnerar ny array** — skriv `arr = push(arr, item)`.
3. **Spara AST-fält före rekursion** — t.ex. `eSym = eNode["sym"]` innan `emitExpr(init)`.
4. **Bracket-access för AST-nycklar** — `node["sym"]` där `.then`/`.value` krockar.
5. **Radbrytning** — använd `CHAR_NL`, inte `"\n"` i serializer (SH3c).
6. **Assign: peek före bump** — `let tok = peek(); bump();`.
7. **Modulskala (L2)** — ≥40 top-level `fn` per modul OK. Densify till 5-radersfiler är **föråldrat** (ökar import-evals).
8. **Exporterade fn** — `pub let X = Ximpl` på facader (SH3b); wrapping `pub fn` ger extra Kab-VM-ram.
9. **Nested import** — `import "self_host/compile"` för hela kedjan; `parse.kab` för AST-only. Importera inte `parser.kab` tillsammans med `parse.kab`.
10. **CALL-args** — undvik var+literal i samma 2-arg `CALL` i heta fn.
11. **Windows stack** — `build.rs` sätter 16 MiB för `kabootar`-bin.
12–51. Nested if/while/call/clobber-workarounds (`pCondStack`, `eIfJmpStack`, `eCalleeStack`, …) är **session-fält**, inte nya modul-globaler. Full lista historiskt nedan; nya `let pSave*` / `let pPos` i facader är **förbjudna** (SH10).

## Historiska clobber-regler (session-fält, inte nya globals)

12. **Compare-parse** — spara lhs före rhs; inte `parsePostfix()` för compare-rhs.
13. **Emitter while** — loop-head i `eWhileHead`; jump-args relativa: `target - jmpIndex - 1`.
14. **Bytecode-cache** — `.kabootar/cache/*.kbc` + fingerprint (content + import-mtimes). SH7b kompilerar bara dirty.
15. **Serialize från `.kbc`** — `CHAR_NL`; inte privata fn från exporterade `serialize_bc`.
16. **Array literal** — `AST_ARRAY` + `make_array`.
17. **Emitter scratch** — `eBxL`/`eBxR` / `eList` / `eBodyStmts` på sess.
18. **Throw** — `AST_THROW` + `throw` opcode.
19. **Nested if/while** — `eIfJmpStack`/`eIfSkipStack`; trimma `eBreakIdxs` efter inner loop.
20. **Parser sym snapshot** — `pFnSym`/`pFnPub` på sess före rekursiv `parseStmt`.
21. **while/if cond** — `pCondStack` på sess.
22. **let/assign sym** — `pBindSym` (inte `pSaveSym`).
23. **assign lookahead** — `pNextTok = pToks[pPos+1]` med EOF-fallback.
24. **bracket index** — fn-lokal `indexObj`.
25. **compare rhs** — `pInAddSub`.
26. **`+`/`-` rhs** — `pAddLeftStack` + rekursiv `parseCompare`.
27. **&& expr** — `pExprLeft`.
28. **binary op** — fn-lokaler `binOp`/`binRight`.
29. **pub exports** — `isPub` / `eExports` / `exports=`.
30. **let/member** — `eStoreSym` / `eMemberFldStack` / `eAssignSym` / `eExprStmt`.
31. **module globals in fn** — `eFnLocals` först, sedan `eGlobals`.
32. **fn snapshot** — `snapArr(eFnOps)` vid push till `eFunctions`.
33. **block loop** — `eBlockIStack`/`eBlockNStack`.
34. **expr-loops** — `eObjIStack`, `eArrIStack`, `eCallArgIStack`.
35. **parseTokens EOF** — `while pDone == 0`.
36. **binary `+` i fn** — spara rhs före `emitExpr(left)`.
37. **let sym** — `pLetSym` före `bump()`.
38. **undefined literal** — `TOKEN_UNDEFINED` → `LIT_UNDEF`.
39. **postfix chains** — interleaved `()`, `.`, `[]`.
40. **`null` vs `undefined`** — `null == undefined` är `false`.
41. **Program body** — block-stack + `OP_HALT`.
42. **index assign** — `AST_INDEX_ASSIGN` + `OP_INDEX_SET`.
43. **emit index assign** — spara `eBxRhs` före `emitExpr`.
44. **popStack** — native `pop(stack)`.
45. **import emit vs compile(emit.kab)** — häng via `.kbc` → serialize/compile, inte bara emit-logik.
46. **SH3a** — `push(s, len(x))` argv N-path. Gate: `sh3a_*`.
47. **CHAR_NL** i serialize (SH3c).
48. **nested call emit** — `eCalleeStack`.
49. **nested call parse** — fn-lokaler `savedCallee`/`savedTypeArgs`.
50. **generic call type args** — `savedTypeArgs` med call.
51. **generic emit** — `eGenericTemplates`; ingen extra import från `emit.kab`.

## Nästa milstolpar (Våg SH)

Historiska 1–14 (roundtrip, facader, bootstrap, generics) är klara. **Inte nästa:** fler `_probe`-filer.

Kort ordning:

1. ~~SH0/SH1~~ ✅ · **SH2** nested named `fn` + sess ✅
2. ~~SH3–SH7b~~ ✅ · ~~SH5 densify~~ ✅ (serialize_sections, parser_expr, emit_fn_scope)
3. ~~**SH16**~~ ✅ appar: ingen rust-emit (`eval_file_cached` / `compile --rust`); toolchain `self_host/` får rust
4. **SH5** fler `parser_stmt`/`parser_postfix` sammanslagningar om leaf ≤10 s
5. ~~**SH17/SH18**~~ ✅ subset (`jitExecOk` + `gcMarkStep`); mmap/exec + radera host-GC deepen
6. ~~**SH19**~~ ✅ subset (`loadIsKab` + `loadIsKbc` + `loadImageName`); radera `main.rs` deepen
7. ~~**SH20**~~ ✅ subset (JSON/datum/regex leaves); radera natives deepen
8. ~~**SH21**~~ ✅ subset (`kabOsIsFile` + `kabOsArgvOk`); radera `runtime/os` deepen
9. ~~**SH22**~~ ✅ subset (`sqlIsWhere` + `sqlStoreOk`); radera `src/sql` deepen
10. ~~**SH23**~~ ✅ subset (`cryptoTls12Ok` + `cryptoRootPem`); rustls-delete deepen
11. ~~**SH24**~~ ✅ subset (`httpIsPost` + `httpIsJson`); radera `runtime/http.rs` deepen
12. ~~**SH25**~~ ✅ subset (`cliIsCompile` + `cliIsFmt`); radera `src/cli` deepen
13. ~~**SH26**~~ ✅ subset (`sciNdLenOk` + `sciFftPow2`); GPU kernel deepen
14. ~~**SH27**~~ ✅ subset (`uiIsCanvas` + `uiFpsOk`); kbrowser deepen
15. ~~**SH28**~~ ✅ subset (`nollAotReady=false` + `nollKeepSrc`); **radera inte `src/`**
16. ~~**F10 AOT native-image policy**~~ ✅ (x64/arm64 ship/ret-op/text/rodata + emit/load/verify data); native machine-code deepen; `nollAotReady` still false

## Historisk bootstrap-logg

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

13. ~~**Generics (språk):** Rust v1 + self-host G4~~ ✅ — `fn id<T>`, monomorphisering, `tests/generics.rs`, `test_parser.kab` / `test_emit.kab`. Design: [docs/GENERICS.md](../docs/GENERICS.md). **Struct (Våg R) ✅** — `self` / `&self` / `&mut self`. Kvarvarande self-host-arbete: **P6b** leaf-budget ([seed/README.md](seed/README.md)). Semikolon förblir valfria.

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
- Fas-profil (mid AccAdd): parse ≈ 37% | emit ≈ 48% | serialize ≈ 15%. Landade cuts:
  maps/`emitSym`, iterative compare, **`eIfDepth`/`eMemberDepth`/`eIndexDepth`**,
  CallArg/obj/arr + callee/block depth, early `IDENT=`, `eOpsN` patches, IR + AccAdd densify.
- Leaf densify plateau → host-VM **`Rc` Array/Object** (COW + cycle reject) + Len/IndexGet.
  `serialize_body` **~144 s** debug — still ≫ 10 s, **skip-list stays**.
- Efter `emit_impl` / `parser_impl` / `serialize_body`-ändring: regenerera motsvarande `self_host/seed/*.kbc`.
