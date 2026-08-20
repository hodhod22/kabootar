# Kabootar — roadmap

> **LÄS DETTA FÖRST (nolltolerans).**  
> **Hela stacken ska bli bara `.kab`.** Kompilator, VM, JIT, GC, OS-policy, browser, libs, CLI — inget Rust, inget C, inget annat språk i produkten.  
> **JIT och GC ska flyttas till Kabootar.** Rust-Cranelift / Rust-GC är **skuld**, inte tak.  
> Ny `.rs`-feature = regression. Nästa arbete är att **ersätta och radera** Rust, inte att förfina den.  
> Undantag finns **inte**. En sista processladdare får bara finnas tills den också är Kab (`.kab` → maskinkod *från* Kab).

**Nu:** densify (SH5) → SH16 → Kab-VM/JIT/GC (SH6/17/18) → **fart i Kab (Våg FT)** → stdlib–CLI i `.kab` (SH20–27) → radera `src/` (SH28). Planer: [egna fötter](#kabootar-på-egna-fötter--noll-rust) · [fart](#våg-ft--fart-alla-tekniker-i-kab).

Produktplaner: **[kOS](../lib/kos/README.md)** · **[kbrowser](../lib/kbrowser/README.md)** · **[kabtest](../lib/kabtest/README.md)**. Detalj: [Våg SH](#våg-sh--self-host-självständig-snabb-stabil-).

## Kabootar på egna fötter — noll Rust

**Mål:** Kabootar ska **stå helt på egna fötter**. Allt produkten behöver — kompilator, VM, JIT, GC, stdlib, OS, nät, SQL, krypto, CLI, tester, paket — skrivs och körs i **Kabootars eget språk (`.kab`)**. Rust, C, C++, LLVM-IR som *produktspråk* och andra värdspråk **försvinner**. Ny `.rs`-feature är regression.

**Klart när:** `src/` och `Cargo.toml` som *produkt* är borta. En användare bygger och kör Kabootar **utan rustc**. Den sista process-starten är antingen (a) en **Kab-AOT-binär** producerad av Kab-JIT/AOT, eller (b) en **minimal oföränderlig stubb** som bara mmap:ar en image som *själv* är emitterad av Kab — och även den stubben ersätts (SH19/SH28). Inga “syscalls i Rust för alltid”.

**Regel för arbete:** funktionalitet som idag sitter i `src/` **portas till `.kab`** (under `self_host/`, `lib/kab/`, `lib/kos/`, …). Vi finputsar inte Rust för att “hinna ikapp”. Host-anrop (OS, sockets) exponeras som **tunna kapabiliteter** som Kab-koden äger policyn för — tills de också är Kab-drivrutiner.

### Vad som fortfarande är Rust-skuld (ärlig inventering)

| Skuld i `src/` | Vad som måste finnas i `.kab` i stället |
|----------------|-----------------------------------------|
| Lexer / parser / emit / serialize (host-compiler) | Redan `self_host/*` — **SH16** stänger fallback för appar |
| Bytecode-VM (`bytecode/vm.rs`) | `self_host/vm*` som **default eval** (SH6 deepen) |
| Cranelift JIT (`bytecode/jit.rs`) | **SH17** register-alloc + native-emit i Kab |
| GC / heap (`runtime`, value Rc) | **SH18** nursery/sweep (eller equivalent) i Kab |
| `main.rs` / CLI / REPL / test-runner | **SH19** + **SH25** `kabootar`-kommando i Kab |
| Modul-laddare, cache, fingerprints | Kab-laddare (SH15 deepen i Kab, inte mer Rust-mmap-logik) |
| Stdlib natives (sträng, array, math, JSON, datum, regex, …) | **SH20** `lib/kab/` + `self_host` primitives |
| OS / FS / process / tid (`runtime/os`, `stdlib/fs`) | **SH21** kOS + `os` i Kab |
| SQL-motor (`src/sql`) | **SH22** SQL i Kab |
| Krypto / TLS (`runtime/security`, rustls) | **SH23** `import "crypto"` + TLS i Kab (eller Kab-bundna cert-rötter) |
| HTTP / fetch (`runtime/http`) | **SH24** HTTP-stack i Kab ovanpå SH21/SH23 |
| Science / GPU / nd (`runtime/science`) | **SH26** Kab-kernels; native GPU bara som tillfällig syscall |
| Browser / DOM / canvas / game | **SH27** kbrowser + game i Kab |
| Evaluator, preprocess, LSP-hjälp, pakethantering | Portas med respektive yta; ingen ny AST-eval i Rust |
| `cargo test` som enda sanning | **SH25** + **[kabtest](../lib/kabtest/ROADMAP.md)**: `kabootar test` kör `.kab`-gates; gästspråk via adapters; CI utan rustc när SH28 är klar |

### Ordning (beroenden — hoppa inte)

```
SH5 densify (färre import-blad)
  → SH16 ingen Rust-emit för appar
  → SH6 Kab-VM default för körning
  → SH17 JIT i Kab  (subset i64-loopar först)
  → SH18 GC i Kab
  → SH19 laddare/CLI-entry i Kab
  → SH20–SH24 stdlib, OS, SQL, krypto, HTTP i Kab
  → SH25 CLI/REPL/test i Kab
  → SH26–SH27 science + browser/game i Kab
  → SH28 radera src/ (produkt). rustc får inte vara ett runtime-beroende.
```

JIT/GC **före** “fler natives i Rust”. Stdlib-port **efter** att Kab-VM kan köra den (annars bygger vi Rust igen). SQL/HTTP **efter** OS-kapabiliteter i Kab.

### Delete-gate för hela planen

| Gate | Sanning |
|------|---------|
| Appar | `KABOOTAR_COMPILE=self-host`; Rust-emit **fel** (SH16) |
| Körning | `KABOOTAR_VM=kab-only` default; host-VM **fel** för app-`.kbc` |
| JIT | Hot loop från Kab-JIT; `src/bytecode/jit.rs` **raderad** |
| GC | Frame-budget utan Rust-GC; Rust-GC **raderad** |
| CLI | `kabootar run/compile/test` är Kab; `src/cli` **raderad** |
| Fri från Rust | Ingen `src/**/*.rs` i produktträdet; `Cargo.toml` bara ev. *extern* tooling, inte runtime |

**Icke-mål:** att behålla rustc “för CI-snabbhet”; att skriva JIT i Cranelift “tills Kab hinner ikapp”; att checka in `_probe_*.kab` som produkt.

Se steg-tabellen i [Våg SH](#våg-sh--self-host-självständig-snabb-stabil-) (SH16–SH28).

## Våg FT — Fart: alla tekniker i Kab

Namnet **FT** (fart/teknik) så det inte krockar med [Våg F — generics](#våg-f--generics-fas-2-g6g11-).

**Mål:** Kabootar ska vara **mycket snabb** — i samma liga som V8/HotSpot/.NET på dynamisk kod, och nära Rust/C++ på `@manual` + unbox + AOT — **utan att bygga farten i Rust**. All avancerad teknik nedan implementeras i **`.kab`** (SH17 JIT, SH18 GC, Kab-VM). P11–P18 i Rust är **prototyp/skuld**; Våg FT är den produkt som blir kvar.

**Princip:** mät (F0) → gör hot path billig i VM (F1–F6) → JIT/AOT (F7–F10) → minne (F11–F12) → parallell/I/O/GPU (F13–F16) → liga-gates (F17). Ingen ny Cranelift/IC/GC-feature i `src/`.

**Tak (ärligt):** dynamisk GC-Kab slår inte C. `@manual` + SIMD + AOT kan. Same-room UI+SQL+HTTP i en process är en egen liga mot Node+Postgres.

### Teknikkarta (vad vi faktiskt ska ha)

| Lager | Tekniker (alla i Kab när F är klar) |
|-------|-------------------------------------|
| **Kompilator** | Incremental + CA-cache (SH7/SH15); densify (SH5); peephole AccAdd/len/index; SSA till JIT; inlining av små fn; DCE; constant folding; escape analysis → stackalloc |
| **Värden** | Unbox i64/f64/bool i slots; packed `array_f64`/`array_i32`; NaN-box *eller* tagged ptr (välj en, mät); shapes / hidden classes; monomorfa objekt |
| **Interpretator** | Direct-threaded eller copy-and-patch dispatch; IC på GetMember/LoadGlobal/CALL; polymorphic IC (max 4); megamorphic fallback; AccAdd/index fast-path; call-frame reuse |
| **JIT (SH17)** | Baseline (template/copy-and-patch) → optimizing (SSA, GVN, LICM, inlining); linear-scan eller graph coloring regalloc; deopt + OSR; on-stack replacement; type feedback från IC; i64-loopar först sedan f64/SIMD |
| **AOT** | Fingerprint → native image (Kab-emit, inte LLVM-as-product); profile-guided (PGO) från `.kab`-profiler |
| **GC (SH18)** | Nursery bump + promote; incremental/concurrent mark; frame-budget för 60 FPS; write barriers i Kab; `@manual` utan checks i release |
| **Minne** | Region/bump per compile-session (SH13 deepen); object pooling för tokens/ops; zero-copy buffers till GPU/FS |
| **Parallell** | Workers/job-system; compiler-blad parallellt (SH7); data-parallel `job_map` på unboxed arrays |
| **I/O** | io_uring/IOCP bakom Kab-`os` (SH21); HTTP without JSON hop (P17/F15); SQL same-process |
| **SIMD/GPU** | vec/matmul/FFT som Kab-API + SIMD-JIT eller GPU-kernel; inga per-element boxed loops |
| **Profil** | `KABOOTAR_P10_PROFILE`, op-histogram, `jit_stats`, alloc/frame; PGO tillbaka till JIT |

### Ordning och gates

| Steg | Vad | Delete-gate / mätning | Status |
|------|-----|------------------------|--------|
| **F0** | **Profiler i Kab** — compile-fas-ms, op-histogram, alloc/frame, IC hit-rate | Samma siffra i release varje PR; ingen gissning | ✅ subset: `lastCompileMs` / `bootLastCompileMs`; `compileIr` sätter parse/emit; deepen = PGO |
| **F1** | **Dispatch** — threaded interpreter / copy-and-patch i Kab-VM | tight add-loop ≫ boxed interp | 📋 |
| **F2** | **IC + shapes** — GetMember/LoadGlobal/CALL monomorf → poly (≤4) | hit-rate > 90 % på typisk app | 📋 (P12 Rust = skuld) |
| **F3** | **Unbox slots** — i64/f64/bool/`array_f64` i frames | ≥10× vs boxed add-loop (Kab-VM) | 📋 (P11 Rust = skuld) |
| **F4** | **Call convention** — argc 0–3 utan heap-argv; frame reuse | CALL inte top-3 i profiler | 📋 |
| **F5** | **Peephole** — AccAdd, len_local, index_get_*; Kab-emit redan delvis | self-host serialize/loop billigare | ✅ subset i `self_host` emit |
| **F6** | **Inline små fn** — JIT och/eller emit för 1-block getters | färre CALL i hot loops | 📋 |
| **F7** | **Baseline JIT i Kab** — template per opcode → maskinkod (SH17 start) | `jit_stats` hits från Kab-JIT, inte Cranelift | ✅ subset: `jitEmitRet` + `jitEmitI64IncRet` (xor/add/ret); deepen = mmap/exec |
| **F8** | **Optimizing JIT** — SSA, inlining, LICM, GVN, deopt | i64-loop nära native minus skatt | 📋 |
| **F9** | **Regalloc + SIMD i JIT** — linear-scan; later SIMD-unbox | nd-add/dot utan boxed loop | ✅ subset: `jitGprCount` linear-scan stub; deepen = färgning + SIMD |
| **F10** | **AOT + PGO** — warmed image; profilstyrd JIT/AOT | kallstart + steady-state gates | 📋 |
| **F11** | **Nursery GC i Kab** (SH18) — bump, promote, frame-budget | 60 FPS utan GC-spike i idle | ✅ subset: `gcBump` / `gcNeedCollect` / 16 ms; promote deepen |
| **F12** | **Escape analysis + `@manual` release** — stackalloc; noll checks | use-after-move bara debug | 📋 |
| **F13** | **Parallell compile + workers** | SH7/P8 deepen i Kab | ✅ SH7 subset; workers deepen |
| **F14** | **I/O-stack** — async FS/net utan extra copy | SH21/SH24; e2e vs Node | 📋 |
| **F15** | **Same-room webb** — ingen JSON-varv UI↔SQL | P17 deepen i Kab | ✅ subset |
| **F16** | **GPU kernels** — nd/matmul bakom Kab-API | SC/P16; CPU-fallback i Kab | 📋 deepen |
| **F17** | **Liga-CI** — Python / V8-klass / C#-klass / 60 FPS game | `perf_tak_*` mot Kab-baseline (inte rustc som vinst) | 📋 |

**Koppling till P-vågen:** P0–P18 dokumenterar *vad* som mättes i host-VM. **F0–F17 är samma tekniker, skrivna i Kab**, så de överlever SH28. Att jaga mer Cranelift i `src/bytecode/jit.rs` är **fel riktning**.

**Koppling till SH:** F7–F11 = SH17/SH18. F13 = SH7. Compile-tid = SH5/SH8/SH14. Utan Kab-JIT (F7) stannar “mycket snabb” på interpretatorn.

## v0.2 (nu) — Kabootar-foundation

- [x] Byt namn Nova → Kabootar
- [x] `null` / `undefined` / `NaN`-semantik
- [x] Runtime-stubbar: browser DOM, KDOM, OS, DB
- [x] Klassmodul (grundtyper)
- [x] Dokumentation
- [x] WASM + tester

## v0.3 — Klasser och KML

- [x] `class` parsing och instansiering (`Person()`)
- [x] `this` och metodanrop
- [x] KML-parser och renderer
- [x] Flyttalsliteraler (`3.14`)

## v0.4 — Databas

- [x] Persistent in-memory tabeller
- [x] `SELECT` med `WHERE`
- [x] Parametriserade queries (`$1`, `$2`, …)
- [x] Enkel `JOIN`

## v0.5 — Backend

- [x] HTTP-server inbyggd i runtime
- [x] Request/response-modell
- [x] Filsystem via `os`

## v1.0 — Fullstack

- [x] Kabootar OS — kernel, VFS, kapabiliteter
- [x] Pakethantering (`import "mod"`)
- [x] PostgreSQL-kompatibel SQL-subset
- [x] LSP / IDE-stöd

## v1.6 — Säkerhetsverktygslåda

- [x] Kryptografiska primitiver (AES-256, ChaCha20, RSA, ECC, SHA-3, Argon2)
- [x] `crypto_secure` / `crypto_wipe` — känslig data i minnet
- [x] Enhets-API (USB, TPM, smartkort-stubbar)
- [x] Pluggable security providers (`software`, `tpm-stub`, `yubikey-stub`, `hsm-stub`)
- [x] `import "crypto"` + dokumentation (`SECURITY.md`)

## v1.7 — Science-modul

- [x] `import "science"` — komplexa tal
- [x] Matematik (trig, log, kvadrat, factorial, gcd)
- [x] Fysik, kemi, ekonomi, digital/bit-ops
- [x] Statistik, matriser, numerisk analys
- [x] Dokumentation (`SCIENCE.md`) med exempel per funktion

## v1.8 — DocAI

- [x] `kabootar-docai` CLI — interaktiv dokumentationsassistent
- [x] `import "docai"` — `doc_ask`, `doc_search`, `doc_sources`, `doc_topics`
- [x] Inbäddat doc-index från alla `docs/*.md`
- [x] Dokumentation (`DOCAI.md`)
- [x] VS Code-panel och kommandon (`Fråga DocAI`, sök, ämnen)

## v1.9 — Språkkärna

- [x] Lexikalisk miljö — `Rc`/`RefCell` (billiga closures, inget env-klon-OOM)
- [x] Array-literaler `[1, 2, 3]`
- [x] `import` från fil (`lib/*.kab`, `KABOOTAR_PATH`)
- [x] Förbättrade felmeddelanden (`did you mean …?`)

## v2.0 — Projektlivscykel

- [x] `kabootar mod init` / `kabootar mod run` — projektmallar (`web`, `api`)
- [x] Deploy-väg: `kabootar serve` + `http_serve()` (HTTP + DB + OS i en binär)
- [x] `pub fn` i `.kab`-moduler (filmoduler exporterar endast publika funktioner)
- [x] Bytecode/AOT-kompilering (v2.18 — stack-VM, `.kbc`-cache; AST-fallback för import/klass/async)

## v2.1 — Paket & utvecklarflöde

- [x] `pub let` — exportera konstanter från moduler
- [x] `@version` i `.kab` + `import "mod@1.0"` + `[dependencies]` i `kabootar.toml`
- [x] `kabootar compile` — parse-cache (`.kabootar/cache/*.kbc`)
- [x] `kabootar serve --watch` — hot reload vid filändring

## v2.5 — super i arvade klasser

- [x] `super.init(...)` — förälderns konstruktor i barnets `fn init`
- [x] `super.method()` — anropa föräldermetod även om barn överskriver
- [x] Ärvd `fn init` utan omdeklaration — `Child(args)` använder förälderns init

## v2.6 — Async event loop

- [x] Microtask-kö — `async fn`-anrop schemaläggs vid call, körs vid `await`
- [x] Delade Promise (`Rc`) — samma promise kan `await`as flera gånger
- [x] Kör kvarvarande microtasks i slutet av `eval_source`

## v2.7 — sleep_ticks och interfaces

- [x] `sleep_ticks(n)` — yield efter n scheduler-ticks (låter andra async-tasks köra)
- [x] `interface I { fn m(); }` — metodsignaturer utan kropp
- [x] `class C implements I` — compile-time validering av metoder (inkl. ärvda)
- [x] `is_impl(obj, "I")` — runtime interface-check

## v2.8 — Parallell async IO

- [x] `os_read_async` / `os_write_async` — Promise-baserad VFS
- [x] `http_request_async` — async in-process HTTP
- [x] `sql_async` — async SQL-queries
- [x] `await_all([p1, p2, ...])` — vänta på flera promises parallellt

## v2.9 — Riktig nätverks-IO

- [x] `http_fetch_async(method, url, body)` — HTTP över TCP
- [x] Skiljer in-process (`http_request_async`) från extern fetch

## v2.10 — HTTPS / TLS

- [x] `https://` i `http_fetch_async` — rustls + Mozilla root certs
- [x] Standardport 443, valfri `:port` i URL

## v2.11 — TLS trust och cert pinning

- [x] `tls_add_ca(pem)` — lägg till extra CA (behåller Mozilla roots)
- [x] `tls_ca_only(pem)` — lita endast på angiven CA
- [x] `tls_pin(host, sha256_hex)` — SHA-256 pinning av leaf-cert
- [x] `tls_reset()` — återställ standard trust store
- [x] `tls_cert_sha256(pem)` — fingerprint för pinning-setup

## v2.12 — HTTP headers

- [x] `http_fetch_async(method, url, body, headers)` — valfritt headers-objekt
- [x] `http_headers(response)` — response headers som objekt (lowercase-nycklar)
- [x] Parsar response headers från extern fetch

## v2.13 — HTTP komplett

- [x] String-nycklar i objekt — `{ "Content-Type": "application/json" }`
- [x] `http_header(response, name)` — enkel header-lookup (case-insensitive)
- [x] Automatiska redirects — följer 301/302/303/307/308 (max 10 hopp)
- [x] 301/302/303 byter till GET vid redirect

## v2.14 — Språksocker

- [x] `?`-operator på `Result` — unwrap `Ok`, propagera `Err`
- [x] `match`-guards — `n if n > 0 => "positive"`
- [x] `is(obj, "Class")` — klass-/arv-kontroll (kompletterar `is_impl`)

## v2.15 — Match-mönster för array och objekt

- [x] Array-mönster — `[]`, `[a, b]`, `[head, ...rest]`, `[a, ...mid, b]`
- [x] Objekt-mönster — `{}`, `{ name: n }`, `{ name, age }`, `{ ...rest }`
- [x] String-nycklar i objekt-mönster — `{ "Content-Type": ct }`
- [x] Nästlade mönster — `Ok([a, b])`

## v2.16 — HTTP fetch timeout

- [x] `http_set_timeout(ms)` — global standard-timeout för `http_fetch_async` (0 = ingen)
- [x] `http_reset_timeout()` — återställ till ingen timeout
- [x] `http_fetch_async(..., timeout_ms)` — valfritt 5:e argument (millisekunder, överstyr global)

## v2.17 — Lokalt paketregistry

- [x] `.kabootar/registry/` — publicera `.kab`-moduler med `@version`
- [x] `.kabootar/packages/` — installerade beroenden per projekt
- [x] `kabootar publish <file|name>` — publicera till lokalt registry
- [x] `kabootar install [name@ver]` — installera från registry (eller alla `[dependencies]`)
- [x] `registry_publish`, `registry_install`, `registry_list` — natives för skript/CI
- [x] `import` löser installerade paket när `lib/` saknar modulen

## v2.18 — Bytecode / AOT (första steg)

- [x] Stack-baserad bytecode-VM — aritmetik, `let`, anrop, `fn`, ternary/`if`
- [x] AST → bytecode-kompilator med automatisk fallback
- [x] `.kbc`-filer i `.kabootar/cache/` (serialiserad bytecode)
- [x] `kabootar compile` skriver bytecode när möjligt
- [x] `bytecode_can_compile(source)` — runtime-check

## v2.19 — Utökad bytecode

- [x] Array-literaler — `MakeArray`
- [x] Indexläsning — `arr[i]` via `IndexGet`
- [x] `.length` på array/sträng — `GetLength`
- [x] `while`-loopar med blockkropp
- [x] Tilldelning till namn — `Dup` + `StoreLocal`/`StoreGlobal`

## v2.20 — Bytecode v2.2-konstruktioner

- [x] Objekt-literaler — `MakeObject`
- [x] Medlemsläsning — `obj.field` via `GetMember`
- [x] Indexskrivning — `arr[i] = v` via `IndexSet` + `Swap`
- [x] Medlemsskrivning — `obj.field = v` via `MemberSet`
- [x] `for x in xs` — desugar till index-loop (array/sträng)
- [x] Klassisk `for` — init/cond/step + blockkropp

## v2.21 — Bytecode korrigeringar och v2.2-rest

- [x] `const` — immutable locals + sync till `env` efter körning
- [x] `StoreGlobal` via `env.assign` — respekterar `const` över flera `eval_source`
- [x] Metodanrop — `obj.method(args)` via `GetMember` + `Call`
- [x] `map`/`filter` med `BytecodeFn` — `call_with_args` delegerar till VM
- [x] Template literals — `+`-kedja på strängar (parser → bytecode)

## v2.22 — Bytecode v2.3-konstruktioner

- [x] Array-/objekt-spread — `ConcatArray`, `MergeObject`
- [x] Spread i anrop — `CallFromArray`
- [x] Destructuring — `let [a, b] = xs`, `let { name: n } = obj`, `...rest`
- [x] Tilldelning med destructuring — `[x, y] = pair`
- [x] `try`/`catch` på `Result` — `MakeOk`/`MakeErr`, `JumpIfResultErr`
- [x] `break` / `continue` i `while`, `for-in`, klassisk `for`

## v2.23 — Bytecode v2.4-subset

- [x] Sync pilfunktioner — `MakeArrowFn`, `BytecodeFn` per modul/funktion
- [x] `match` (subset) — `_`, tal, variabel, `Some`/`None`, `Ok`/`Err`, guards

## v2.24 — Bytecode match v2.15

- [x] Array-mönster — `[]`, `[a, b]`, `[head, ...tail]`, `[first, ...mid, last]`
- [x] Objekt-mönster — `{}`, `{ name: n }`, shorthand, `...rest`
- [x] Kapslade mönster — `Ok([a, b])`
- [x] Opcodes — `JumpUnlessArray/Object`, `JumpUnlessHasMember`, `ArraySliceRest`, `ObjectRest`, …

## v2.25 — Bytecode async/await

- [x] `async fn` — microtask via `AsyncBody::Bytecode`
- [x] `async (n) => ...` — `BytecodeFnDef.async_fn`
- [x] `await` — opcode `Await` + `resolve_await_value`

## v2.26 — Bytecode klasser (v2.4)

- [x] `class` / `extends` — registrering + `NewInstance`
- [x] `fn init` + metoder som bytecode (`MethodDef.bytecode`)
- [x] `this` / fält-tilldelning — `GetMember`/`MemberSet` på `ClassInstance`
- [x] Ärvda fält med standardvärden

## v2.27 — Bytecode super + interface/implements

- [x] `super.init(...)` / `super.method()` — opcode `GetSuperMethod`
- [x] `interface` / `implements` — `BytecodeInterfaceDef` + compile-time validering
- [x] Ärvda metoder räknas mot interface-krav

## v2.28 — .kbc-serialisering + bytecode import

- [x] Serialisera/deserialisera `classes`, `interfaces` i `.kbc`
- [x] Klassmetoder med constants/globals/opcodes i cache
- [x] `import "mod"` i bytecode — `BytecodeModule.imports`, körs före klasser

## v2.29 — Bytecode pub fn/pub let + filmoduler

- [x] `pub fn` / `pub let` kompileras till bytecode (`BytecodeModule.exports`)
- [x] `run_module` markerar exporterade namn för `export_module_bindings`
- [x] `lib/*.kab`-moduler (t.ex. `greet`, `config`) via bytecode-spåret

## v2.30 — Modul-scope till funktioner + kedjeimport

- [x] `StoreLocal` i modulkropp synkar till `env` — funktioner ser `let`/`const` under körning
- [x] `BytecodeFunction` med closure — importerade och modulbindningar i `LoadGlobal`
- [x] `pub let` + `pub fn` i samma fil (t.ex. `ok(n) <= MAX`)
- [x] Kedjeimport `lib/*.kab` → `import "other"` via bytecode

## v2.31 — Spread-literaler + modulförbättringar

- [x] Array spread i literal — `[...a, 1]` via `ConcatArray`
- [x] Objekt spread i literal — `{ ...base, z: 1 }` via `MergeObject`
- [x] Versionerad import — `import "greet@1.0"` i bytecode
- [x] Top-level `return` i bytecode-program
- [x] `.kbc`-cache för moduler med `exports` + `pub async fn`-export
- [x] `import` av filmoduler läser `.kbc` när cache finns (`load_program_for_file`)

## v2.32 — Klassfält-uttryck + pub import

- [x] Klassfält med uttrycksdefault — `count: number = 1 + 2`, modul-scope i default
- [x] `BytecodeClassField.default_code` + serialisering i `.kbc`
- [x] `pub import "mod"` — re-exporterar importerade `pub`-bindningar
- [x] `BytecodeModule.pub_imports` + `run_module` markerar re-exports

## v2.33 — Result `?` + destructuring-rest + callable calls

- [x] `?`-operator på `Result` — opcode `ResultQuestion` (unwrap `Ok`, propagera `Err`)
- [x] Objekt-destructuring med `...rest` — `ObjectRest` i `let`/`const`/tilldelning
- [x] Anrop på uttryck — `(n) => n + 1)(4)`, `f(...xs)` via `compile_callable` + `CallFromArray`
- [x] Parser — dubbel `((` gruppering för IIFE-pilfunktioner

## v2.34 — Undefined/NaN + spread-konstruktor + is()

- [x] `Undefined` / `NaN` som bytecode-konstanter
- [x] Klass-konstruktor med spread — `Point(...args)` via `NewInstanceFromArray`
- [x] `is(obj, "Class")` i bytecode-program med klasser
- [x] Klassfält-default med `Undefined`

## v2.35 — Oinitierade bindingar + rest-between

- [x] `let x` / `const slot` utan init — binder `undefined`
- [x] Array-destructuring `[first, ...mid, last]` — `ArraySliceRest` + `IndexPeekFromEnd`
- [x] Tilldelning med rest-between — `[a, ...m, b] = pair`
- [x] Match `[first, ...mid, last]` bekräftad i bytecode (`can_compile`)

## v2.36 — Fn-uttryck + strängnycklar i destructuring

- [x] `fn`-uttryck som värde — `let f = fn g() { ... }` via `MakeArrowFn`
- [x] `async fn`-uttryck — `await f()` i bytecode
- [x] Objekt-literal med strängnycklar — `{ "Content-Type": "json" }`
- [x] Destructuring med strängnycklar — `let { "x-key": n } = o`
- [x] Match på objekt med strängnycklar — `{ "id": n }`

## v2.37 — Block-uttryck

- [x] Block som värde — `{ let y = 2\ny }` i uttryckskontext via `compile_expr`
- [x] Block i binära/ternary-uttryck — `1 + { let n = 3\nn }`
- [x] Nästlade block — `{ { inner } + 1 }`
- [x] Block som funktionsargument — `f({ let n = 4\nn })`

## v2.38 — Grupperade parentesuttryck

- [x] Uttryck som börjar med `(` — `(1 + 2)` parsas som grupperat uttryck, inte pilparametrar
- [x] `let x = (1 + 4)` — parenteser i init
- [x] Spread-objekt i parenteser — `({ ...o, b: 2 })`
- [x] Block i parenteser — `( { let n = 6\nn } )`
- [x] Pilfunktioner oförändrade — `(n) => n + 1`, `((n) => n * 3)(5)`

## v2.39 — `in`-operatorn

- [x] Array-medlemskap — `1 in [1, 2, 3]`
- [x] Objektnyckel — `"x" in { x: 1 }`
- [x] Strängdelsträng — `"ab" in "abc"`
- [x] Klassfält — `"x" in instance`
- [x] Bytecode-opcode `In` + `eval_value_in` i `ops.rs`
- [x] `for x in xs` oförändrat (loop-syntax, inte binäroperator)

## v2.40 — Anonym rest i match

- [x] Rest utan bindningsnamn — `[x, ...]`, `[..., x]`
- [x] Rest-between anonym — `[first, ..., last]`
- [x] Prefix med flera fasta — `[..., a, b]`
- [x] Namngiven rest oförändrad — `[x, ...rest]`
- [x] Parser — `parse_optional_rest_name` efter `...`

## v2.41 — Anonym rest i destructuring

- [x] `let [a, ...] = xs` — array tail utan bindning
- [x] `let [..., b] = xs` — array prefix utan bindning
- [x] `let [f, ..., l] = xs` — rest-between i let/assign
- [x] `let { a, ... } = obj` — objekt-rest utan bindning
- [x] `match { a, ... }` — anonym objekt-rest i match
- [x] Namngiven rest oförändrad — `[a, ...rest]`, `{ ...rest }`

## v2.42 — pub let destructuring

- [x] `pub let [a, b] = xs` — exporterar alla bundna namn
- [x] `pub let { x, y } = obj` — shorthand-nycklar exporteras
- [x] `pub let { a: n } = obj` — exporterar bindningsnamn `n`
- [x] Anonym rest exporterar endast övriga bundna namn
- [x] `exported_binding_names` — delad AST-hjälpare för exports
- [x] Bytecode `.exports` + `env.mark_exported` för destructuring

## v2.43 — Nästlade member/index-tilldelningar

- [x] `o.a.b = v` — member-kedja via `emit_load_lvalue_container` / `emit_store_lvalue`
- [x] `xs[0][0] = v` — index-kedja
- [x] `o.items[0] = v` och `xs[0].x = v` — blandade kedjor
- [x] Tilldelningsuttryck returnerar fortfarande högersidan
- [x] `store_lvalue` i evaluator för AST-paritet

## v2.44 — Sammansatta tilldelningar

- [x] `+=`, `-=`, `*=`, `/=`, `%=` — lexer + parser desugar till `x = x op rhs`
- [x] Variabel, member och index-mål — `n += 1`, `o.x += 2`, `xs[0] += 3`
- [x] Nästlade member-mål — `o.a.b += 4`
- [x] Bytecode via befintlig `Assign` + binäroperatorer (ingen ny opcode)

## v2.45 — Super-metod som värde

- [x] `super.f` — bunden super-metod via `GetSuperMethod` (utan `Call`)
- [x] `let m = super.f; m()` — spara och anropa senare
- [x] `run(super.f)` — skicka som callback-argument
- [x] AST-paritet — evaluator hade redan `Expr::Member(Expr::Super, ...)`

## v2.46 — Match på literaler

- [x] `null` — `match null { null => 1, _ => 0 }`
- [x] `true` / `false` — bool-literaler som mönster
- [x] `JumpUnlessConstEq` — literal-fallback till nästa arm (fixar även `Pattern::Number`)
- [x] Skiljs från `None` (Option) och variabelbindning

## v2.47 — Match på float- och sträng-literaler

- [x] `1.5` — float-literaler som mönster (`Pattern::Float`)
- [x] `"hi"` — sträng-literaler som mönster (`Pattern::String`)
- [x] Float-jämförelse via `JumpUnlessConstEq` + `values_equal` (inkl. `1` mot `1.0`)
- [x] Fallback till `_` när literal inte matchar

## v2.48 — Match på `undefined` och `NaN`

- [x] `undefined` — `match undefined { undefined => 1, _ => 0 }`
- [x] `NaN` — `match NaN { NaN => 1, _ => 0 }` via `is_nan()` (inte `==`)
- [x] Skiljs från `null` och övriga literaler
- [x] Bytecode via `JumpUnlessConstEq` med `Constant::Nan`-specialfall

## v2.49 — Tilldelning till `super.member`

- [x] `super.count = 1` — skriver till `this` via `emit_load_lvalue_container(Super)`
- [x] `super.n += 2` — compound assign via desugar + fältläsning
- [x] `GetSuperMethod` utökat med `resolve_super_member` (fält eller bunden metod)
- [x] AST-paritet — `store_lvalue` / `Assign` för `Expr::Super`

## v2.50 — Browser Platform (post-Kv8)

- [x] Kv8 v1.0 — JS-subset, JIT, `.kv8` VFS, `kstyle { }`
- [x] `browser_platform/` — WASM, WebGL, WebRTC, DevTools, Extensions, PWA
- [x] `bp_info()` + `wasm_*`, `webgl_*`, `webrtc_*`, `devtools_*`, `ext_*`, `pwa_*`
- [x] v2.51 — wasmi gäst-WASM (C/Rust i webbläsaren)
- [x] v2.52 — WebGL shader pipeline (wgpu)
- [x] v2.53 — WebRTC ICE/media tracks
- [x] v2.54 — DevTools (console hook, breakpoints, source maps)
- [x] v2.55 — Extensions content scripts + PWA service worker
- [x] v2.56 — DevTools Elements UI + WebRTC STUN/TURN/RTP
- [x] v2.57 — WebGL vertex/index buffers + draw_elements

Se [BROWSER_V2.md](BROWSER_V2.md).

## v2.4 — Pilfunktioner, async/await, klass-arv

- [x] Pilfunktioner — `(x) => x * 2`, block-kropp, `async (n) => ...`
- [x] `async fn` / `await` — Promise-baserat med **microtask-kö** (v2.6)
- [x] Klass-konstruktor — `fn init(a, b)` + `Point(3, 4)`
- [x] Klass-arv — `class Dog extends Animal`
- [x] `this.x = ...` i metoder (inkl. `init`)

## v2.3 — Destructuring, spread, for, try/catch

- [x] Destructuring — `let [a, b] = xs`, `let { name, age } = obj`, `...rest`
- [x] Tilldelning med destructuring — `[x, y] = pair`
- [x] Spread — `[...a, 3]`, `{ ...base, z: 1 }`, `fn(...args)`
- [x] Klassisk for — `for let i = 0; i < n; i = i + 1 { }`
- [x] `try { Ok/Err } catch (e) { }` — Result-baserat (inte JS-undantag)

## v2.2 — Språkparitet (JS + lånade delar)

- [x] `const` — immutable binding
- [x] Objekt-literaler `{ key: value }`
- [x] Array-index `arr[i]` och `.length`
- [x] `for x in xs` (array, sträng, objekt-nycklar)
- [x] Template literals `` `Hej ${name}` ``
- [x] `map`, `filter`, `push`, `len`, `typeof`, `keys`
- [x] `//` kommentarer, `%`, `!`, ternary `? :`
- [x] Feature-matris: [FEATURES.md](FEATURES.md)

## Strategi (2026-07) — Kabootar-only (Rust fasas ut)

**Slutmål:** hela *produktplattformen* i Kabootar — Kv8, DOM, CSS/KSS, OS, webläsare, shell, **spelmotor**, **science/AI**. Native runtime **krymper till JIT/AOT/GC/SIMD/GPU/bootstrap** (P-tak). Ny produktlogik skrivs **aldrig** i Rust. Kabootar ska bli **helt självständig** för appar och forskning — utan Python/NumPy/SciPy/PyTorch som runtime-beroende — **utan** att offra native-taket genom att skriva JIT:en i dynamisk Kab.

**Produktambition (spel):** Kabootar ska bli **bättre än C# och C++ för spelproduktion** — inte genom att vinna rå pointer-aritmetik, utan genom att vinna **hela produktionskedjan**: ett språk från OS→UI→gameplay, snabbare iteration (hot reload / self-host), säkrare än C++ (O + GC-val), mer integrerat än C#/Unity (browser + kOS + 3D i samma runtime), och **GPU-först** så frame-budgeten matchar native motorer.

**Produktambition (forskning & AI):** Kabootar ska **ta över Pythons roll** för vetenskap, dataanalys och AI-utveckling. Språket är redan snabbare än Python i hot path; målet är att vinna **hela forskningskedjan** (utforskning → modell → deploy → UI/OS) utan `venv`/`pip`/`conda`, utan C-extension-helvete, och utan att byta språk när prototypen ska bli produkt. All ny STEM/AI-logik skrivs i **Kabootar** (`.kab`); Rust är tillfällig host/hotpath tills self-host + GPU/SIMD räcker — sedan bort.

**Minnesmodell:**
- **Borrowing / `@manual` / `owned_*`** — systemutveckling (OS, drivrutiner, buffertar, netstack, **spel-hotpath-buffers**). **Compile-time ownership = Våg O** (L5 var bara runtime MemBox).
- **GC (default)** — webutveckling (DOM, Kv8, appar, shell-UI, gameplay-script)

Ordning (strikt) — **just nu: endast språk**, sedan prestanda + spel parallellt med K/H:

0. **Komplettera Kabootar-språket** (L + O + **T** traits + **J** JS-stdlib + **R** struct) — optimera, paritet, ownership  
1. **Self-host som produktionskompilator** (S) — pausad tills J/T/R landat tillräckligt  
2. **Bygg om allt i Kabootar** (K): kv8, dom, css, kOS, kbrowser  
3. **Tunna bort Rust** (H) tills hosten är trivial  
4. **Prestanda + spelproduktion** (P + GP) — P0–P10 bytecode/pipeline; **P11–P18 tak** (unbox, hidden classes, native JIT/AOT, nursery GC, `@manual` release, SIMD/GPU, same-room webb, liga-gates); se Våg P  
5. **Science / AI** (SC) — ta över Pythons roll för forskning/AI; Kab-first (inte Rust); fri från NumPy/SciPy/PyTorch-beroenden

| Våg | Namn | Mål |
|-----|------|-----|
| **L** | Language (systems-ready) | Reentranta lokaler, modulskala, closures, await, MemBox |
| **O** | Ownership | Compile-time Owned/`&`/`&mut` i `@manual` |
| **T** | Traits | Riktiga traits (bounds, generics) utöver `trait`≈`interface` |
| **R** | Struct (Rust-inspirerat) | `struct` + **`self`** i metoder; `class` använder **`this`** |
| **J** | JS-språkparitet | Array/Object/String/Math + övriga ES-luckor |
| **S** | Self-host | `self_host/` bygger produkten |
| **K** | Kabootar libs | kv8 + DOM + CSS + OS + webläsare i `.kab` |
| **H** | Host → **noll** | Inget Rust kvar. JIT/GC/laddare i `.kab`. |
| **P** | Performance **i Kab** | Unbox/JIT/AOT/GC/SIMD skrivs om till `.kab` (SH17/SH18). Rust-P11–P16 är skuld att radera. Bred plan: [Våg FT](#våg-ft--fart-alla-tekniker-i-kab). |
| **FT** | Fart / teknik | Alla JIT/IC/GC/AOT/SIMD-tekniker i `.kab` (F0–F17). Inte mer Cranelift i `src/`. |
| **GP** | Game production | GPU-3D, scen/motor, assets + **GP6 system** + **GP7 scene editor** (killer) |
| **SIM** | Simulation / robotics | Digital twin, joints/ODE, robot arm — `import "sim"` (killer cross-cut GP∩SC) |
| **SC** | Science / AI | NumPy/SciPy/sklearn/PyTorch-klass + **SC5 Kab-only** + **SC6** production + **SC7** surface modules |
| **DX** | Exploration DX | REPL + notebook — slå Python för *utforskning* (samma runtime som ship) |

**Aktivt fokus:** nolltolerans `.kab`. SH5 densify → SH16 stäng Rust-emit → **SH17 JIT i Kab** → **SH18 GC i Kab** → radera `src/`.

**Klass vs struct (2026-07):** `class` → **`this`**; `struct` → **`self`** / `&self` / `&mut self` (R1).

### Våg L — Language (systems-ready) ✅ subset

Blockerare från `self_host/README.md` och `lib/kv8/` — måste bort innan ekosystemet kan växa i Kabootar.

| Fas | Innehåll | Status |
|-----|----------|--------|
| **L1** | **Reentranta bytecode-lokaler** — `StoreLocal`/`MakeArrowFn` får inte `assign` upp i parent/modul-env; closures fångar aktiveringsram (`share_bindings`); seed av capture-slots vid fn-entry; `sync_closure_writes` synkar bara riktiga captures | ✅ |
| **L2** | **Modulskala** — `register_functions` + `BytecodeFunction::Clone` använder `share_bindings` (inte djupklon); ≥40 top-level fn/modul utan OOM | ✅ |
| **L3** | **Closures under rekursion** — fångade `let` överlever nästlade anrop av samma fn | ✅ (via L1) |
| **L4** | **Await i modul/fn** — microtask writeback av globals; capture-bitar (`local_captures`); Await synkar locals; `lib/kos/async` använder riktig `await` | ✅ |
| **L5** | **Runtime MemBox** — `@manual` + `owned_*` / `kos/mem` (move/drop vid runtime); GC default. **Inte** compile-time ownership/borrow-check | ✅ runtime |

**Checkpoint L1–L5:** `cargo test --test v228_language bytecode_` + `cargo test --test ownership_manual` + `cargo test --test s2_compile_cli`

### Våg O — Ownership (systems, compile-time) ✅ subset

GC förblir default. Ownership gäller **bara** `@manual`-moduler. Se [OWNERSHIP.md](OWNERSHIP.md).

| Fas | Innehåll | Status |
|-----|----------|--------|
| **O1** | **Affine Owned** — compile-time use-after-move; `let y = x` / call-arg flyttar Owned; kända peek-API (`owned_read`/`owned_write`) flyttar inte | ✅ |
| **O2** | **Signaturer** — `fn f(b: Owned)`, `fn g(b: &Owned)`; call-arg med Owned flyttar (om inte `&`/`&mut`) | ✅ |
| **O3** | **Borrow** — `&x` / `&mut x`, typer `&Owned` / `&mut Owned`; shared vs exclusive; borrow-scope = call-uttryck | ✅ |
| **O4** | **Scope drop** — compile-time varning/fel om Owned lever över scope utan `drop`/`move` (leak-lint); runtime drop oförändrad | ✅ |
| **O5** | **Self-host checker** — port O1–O3 till `self_host/` så produktkompilatorn checkar ownership | ✅ subset (+ borrow parity; **wired** i `self_host/compile.kab` för `@manual` — `o5_compile_wires_ownership_checker`) |

**Checkpoint O1–O3:** `cargo test --test ownership_check` + `cargo test --test ownership_manual`

**Icke-mål (medvetet):** Rust-lifetimes, lifetime-elision, borrow över async boundaries, ownership i GC-moduler.

### Våg T — Traits (språk) ✅ subset

G5 gav `trait` ≈ `interface`. Det räcker **inte** för systems-/generics-kod. Se [GENERICS.md#traits](GENERICS.md#traits).

| Fas | Innehåll | Status |
|-----|----------|--------|
| **T0** | `trait` / `interface` + `implements` + `is_impl` (G5) | ✅ subset |
| **T1** | **`where T: Trait`** på generiska fn/klass/metod — monomorphisering respekterar bound | ✅ |
| **T2** | **Generiska traits** — `trait Show<T> { … }` | ✅ |
| **T3** | **Associated types** — `trait Iter { type Item; }` (subset) | ✅ |
| **T4** | **Default-metoder** i trait-kropp | ✅ |
| **T5** | Self-host: trait/`where` i `self_host/parser` + `emit` | ✅ subset (+ `type Item;` → `associatedTypes`; default method body `{ … }` i parser) |

**Icke-mål:** HKT, `dyn Trait`-objekt, Rust-coherence.

### Våg R — Struct (Rust-inspirerat) ✅

| Fas | Innehåll | Status |
|-----|----------|--------|
| **R0** | **`this` i `class`** — klassreceiver = `this`; `self` reserverat i lexern | ✅ |
| **R1** | **`struct Name { … }`** — värdetyper + metoder med **`self` / `&self` / `&mut self`** | ✅ |
| **R2** | Struct + `@manual` ownership (move) | ✅ |
| **R3** | Generiska structs `struct Box<T>` | ✅ |
| **R4** | Self-host: `struct`/`self` i parser+emit | ✅ subset |

**Regel:** `class` → `this`; `struct` → `self`. Se [CLASSES.md](CLASSES.md).

### Våg J — JS-språkparitet (stdlib + syntax) ✅ subset

Kabootar har redan stor del av ES2020–ES2025 (ofta snake_case). Nedan är **kvarvarande luckor** för språket (inte Kv8-bundle-yta).

#### J1 — String ✅

| API | Status |
|-----|--------|
| Core: trim/split/replace/replaceAll/pad*/matchAll/localeCompare/… | ✅ |
| **`str_at` / `.at`** | ✅ |
| **`is_well_formed` / `to_well_formed`** (+ `.isWellFormed` / `.toWellFormed`) | ✅ |
| **`string_concat`**, **`string_raw`** / `String.raw` | ✅ |
| `normalize` **NFKC/NFKD** | ✅ |

#### J2 — Array ✅ (medvetna avvikelser)

| API | Status |
|-----|--------|
| map/filter/flat/flatMap/at/toSorted/toReversed/toSpliced/with/findLast/… | ✅ |
| `Array.groupBy` | använd `Object.groupBy` / `group_by` |
| Muterande `sort`/`reverse` (JS in-place) | medvetet: alltid copy |
| Array iterator-protokoll (lazy) | ✅ J5 |

#### J3 — Object ✅ (Parent + class, ingen prototype)

Kabootar har **inte** JS-prototyper. Två tydliga modeller:

| Behov | Modell |
|-------|--------|
| Klassarv | `class` / `extends` / `implements` — [CLASSES.md](CLASSES.md) |
| Dataobjekt-kedja | **Parent**: `Object.getParent` / `setParent`, `Reflect.getParent` / `setParent`, `Object.create(parent)` |

| API | Status |
|-----|--------|
| keys/values/entries/assign/fromEntries/hasOwn/groupBy/freeze/seal/defineProperty(ies)/… | ✅ |
| **`Object.getParent` / `setParent`** (Kabootar-modell) | ✅ |
| **`Object.getPrototypeOf` / `setPrototypeOf` / `__proto__`** | ❌ **icke-mål** |

#### J4 — Math ✅

| API | Status |
|-----|--------|
| floor/ceil/trunc/sign/imul/clz32/fround/f16round/sumPrecise/trig/… | ✅ |
| **Konstanter** `LN2`, `LN10`, `LOG2E`, `LOG10E`, `SQRT1_2`, `SQRT2` (+ globals) | ✅ |
| **`Math` namespace** (`Math.floor`, `Math.PI`, …) | ✅ |

#### J5 — Övrig ES-paritet (språk) ✅ subset

| Område | Luckor |
|--------|--------|
| **Number** | `Number.EPSILON` / `MAX_SAFE_INTEGER` / namespace ✅ |
| **Promise** | `withResolvers` ✅; `Promise.try` ✅ + `Promise` namespace |
| **Iterator helpers** | `Iterator.from`, `.map`/`.filter`/`.take` (ES2025) ✅ |
| **Map/Set** | `getOrInsert` / `getOrInsertComputed` ✅ |
| **RegExp** | `u` + `\p{…}` + `v`/`unicodeSets` ✅ subset |
| **Syntax** | logical assign `||=` `&&=` `??=` ✅ |

**Checkpoint J:** `cargo test --test js_stdlib_gaps` + `cargo test --test kabootar_js_parity`

### Våg S — Self-host som produktkompilator ✅ subset (P6b 📋)

| Fas | Innehåll | Status |
|-----|----------|--------|
| **S1** | Migrera bort workarounds som L1–L3 gör onödiga (fn-lokala stacks istället för `eNode`/`pLeft`-familjen där det går) | ✅ slice: `emit.kab` AST_BINARY + `parser.kab` call/index |
| **S2** | `kabootar compile` default via `self_host/compile.kab` för `.kab` → `.kbc` (`--rust` / `--self-host`; `self_host/` → Rust) | ✅ |
| **S3** | CI: self-host bygger self-host (bootstrap) som gate | ✅ |

### Våg K — Ekosystem i Kabootar 📋

| Fas | Innehåll |
|-----|----------|
| **K1** | **Kv8** — lexer/parser/eval/JIT-policy i `.kab` (ersätt Rust `kv8_*`) — ✅ **subset** (lexer+parser i `lib/kv8`; eval hybrid). Gate: `cargo test --test kv8_lib -- --test-threads=1` |
| **K1c** | **Kv8 Kabootar eval** — ✅ **subset**: `evalSourceKab` → `evalSourceWith` (H6a Kab-only; ingen Rust `evalSource`-fallback) |
| **K1d** | **Kv8 class/new/async** — ✅ **subset**: `K_NEW`/`this`/`K_AWAIT` + **static** + **super()**/**super.method** + `Promise.reject`/`Promise.all` (`k1d_*_kab_eval`) |
| **K1e** | **Kv8 extends + Kab evalSource** — ✅ **subset**: `extends` mergar parent-metoder; `evalSource` → Kab-path (`evalSourceWith`) (`k1e_extends_kab_eval`, `k1e_eval_source_prefers_kab`) |
| **K1f** | **Kv8 async/Promise** — ✅ **subset**: async fn returnerar `{__k8promise,value}`; `K_AWAIT` unwrap; `Promise.resolve` stub i `evalSourceWith` (`k1f_async_promise_kab_eval`) |
| **K1g** | **Promise.then microtask** — ✅ **subset**: `.then(cb)` köar microtask; `drainMicrotasks` efter stmt i `evalSourceWith` (`k1g_promise_then_microtask`) |
| **K2** | **DOM + CSS/KSS** — ✅ **subset**: `querySelector` + KSS object→CSS i `.kab` (`kdom_query_kss_smoke`); layout/paint fortfarande Rust |
| **K2 deepen** | **applyCss + matches** — ✅ **subset**: `kdom_applycss_matches_smoke` (kdom + kss + selectors + theme `applyCss`) |
| **K2-layout** | **flex/box orchestration** — ✅ **subset**: `lib/kstyle/layout` `flexColumn`/`flexRow`/`gap`/`pad`/`applyFlex` (stil-helpers; native layout engine kvar) (`k2_layout_smoke`) |
| **K3** | **kOS kärna** — ✅ **subset**: VFS + mem + sched + policy. **Plan:** [lib/kos/ROADMAP.md](../lib/kos/ROADMAP.md). Gate: `cargo test --test os_lib` |
| **K4** | **kbrowser** — ✅ **subset**: tabs + VFS navigate + paint. **Plan:** [lib/kbrowser/ROADMAP.md](../lib/kbrowser/ROADMAP.md) |
| **K5** | **kOS skrivbord** — ✅ **subset** (shell/Start/fönster/Explorer). **Plan:** [lib/kos/ROADMAP.md](../lib/kos/ROADMAP.md) |

### Våg H — Släpp Rust (allt blir Kabootar) 📋

**Regel:** inga nya *features* i Rust. Nästa arbete är att **ersätta** Rust med `.kab` och **radera** host-filen.

- Appar, libs, OS-policy, DOM, browser, **kompilatorn** = Kabootar.
- Rust idag = laddare + ev. sista maskinkodskärnor. **Målet är noll Rust-källor i produkten**, inte “tunn native för alltid”.
- Ordning: **SH (compiler självständig)** → libs redan i `.kab` → radera Rust-emit/fallback → radera host-VM när Kab-VM + AOT från `self_host/` räcker.
- Att lägga tid i Cranelift/JIT *i Rust* som nästa sprint är **fel riktning**.

- Inga nya produktfeatures i Rust (UI, Kv8-policy, OS-policy, DOM, emit)
- Flytta kvarvarande produktlogik till `.kab` under K
- Slutmått: **repo utan `src/*.rs` produktkod** — bootstrap får vara en liten laddare tills den också är Kab/maskinkod *genererad från Kab*.
- **H0** ✅ — stylesheet apply + document `paint` CSS-path prefererar `.kab` (`parseAndApply` via `kstyle/parse`) istället för enbart native `kstyle_parse`
- **H1** ✅ **subset** — desktop shell boot CSS via `import "kstyle/parse"` + `parseAndApply` (inte native `kstyle_parse`); gate `h1_shell_boot_css_kab`
- **H2** ✅ **subset** — `queryKab` i `lib/kdom/query` (#id / .class / tag via `kstyle/selectors` + walk); `document.query` provar Kab först, fallback `kdom_query_selector` (`h2_query_kab_smoke`)
- **H3** ✅ **subset** — `queryAllKab` i `lib/kdom/query`; `document.domExtra(..., "queryAll")` provar Kab först (`h3_query_all_kab_smoke`); används av `kos/windows` `listWindows`
- **H4** ✅ **subset** — Thin Rust Kv8: produktväg = `evalSource`/`evalSourceKab` → `evalSourceWith`; `evalSourceRust` endast för luckor; `preferKabEval()` sätter flagga (`h4_prefer_kab_eval`)
- **H5** ✅ **subset** — paint/layout-orchestration i `.kab`: `lib/kdom/paint` `paintNode` / `paintWithCss` / `layoutPaint` (flexColumn via `kstyle/layout` + `paint`) (`h5_layout_paint_smoke`)
- **H5b** ✅ **subset** — event drain: `pollEvents` + `drainKosEvents` (Start `launchStartApp`); host shell musklick → `kb_click` → drain → remount/paint (`kos_event_drain_smoke`, `kos_host_click_smoke`)
- **H6** 🚧 **Noll Rust.** JIT/GC/laddare ska bli `.kab` (SH17/SH18). Cranelift i Rust är **inte** målet.

**H6-ordning (mot noll Rust):**

| Fas | Mål | Delete-gate |
|-----|-----|-------------|
| **H6a** | Kv8 Kab-parity: arrow, builtins, DOM-host, expr/stmt-luckor | `evalSourceRust` / `kv8_eval_source` bort — smoke på Kab-only |
| **H6b** | kDOM/KSS-policy i `.kab`; layout/paint-regler | native `kdom_query*` / `kstyle_parse` bara som thin FFI |
| **H6c** | Browser chrome (tabs/nav) + kOS i `.kab` | `kabootar_browser` = window/pixels/input |
| **H6d** | OS-policy (sched/VFS/process) i `.kab` | Rust = disk/net/GPU/hw |
| **H6e** | Bootstrap: evaluator/bytecode/lexer i Kabootar | Rust = minimal laddare; sedan den också i Kab/maskinkod |

**H6a** ✅ — Kab-only `evalSource` (`Object.assign`/`Object.is`/`Symbol`/`globalThis`, events/timers/style); `evalSourceRust` + `kv8_eval_source` bort (`kv8_h6_*`, `kv8_h6a_parity`, `kv8_h4_prefer_kab`).

**H6b** ✅ — `queryKab`/`queryAllKab` + `document.query` Kab-only (`kdom_query_selector*` natives bort).

**H6c** ✅ — chrome/core via `kbrowser/nav`; Rust `BrowserTab.history` bort; **`kb_back`/`kb_forward`/`kb_tab_open`/`kb_tabs` natives bort**; load/paint via `kb_navigate`.

**H6d** ✅ **subset** — OS-policy i `.kab`: `kos/vfs_policy`, `kos/sched_policy`, `kos/process_policy`; `kos/boot` seed (`h6d_os_policy_smoke`). Rust = disk/net/GPU/hw + thin `os_*`. Plan: [lib/kos/ROADMAP.md](../lib/kos/ROADMAP.md).

**H6e** ✅ **subset → delete-mål (produktlogik)** — Kab VM + self-host facades. **Produkt-`import`** prefererar self-host (`load_program_for_file` → `compile_file_prefer_cached`; Rust under `KAB_VM_EXEC_ACTIVE`). Tunna facader self-hostar CI-snabbt. **Skip-list tom (P6b).** **Committed seeds:** `self_host/seed/*.kbc` (kab-only cache). Regenerera: `scripts/regen_self_host_seeds.sh`. **P6 policy:** `attempt-all`. Compile-attempt-policy i `kab/boot` (`bootPolicy`) med loader-spegel `SELF_HOST_MAX_SOURCE_BYTES` / `self_host_would_attempt`. Smokes: `h6e_kab_*`, `h6e_boot_policy_smoke`, `p6_leaf_self_host_compile_budget`.

**H vs P vs FT:** “Rust bort” gäller JIT, GC, SIMD, AOT, loader — de **portas till Kab** ([Våg FT](#våg-ft--fart-alla-tekniker-i-kab)). P11–P18 i `src/` är skuld. Nästa sprint är `.kab`, inte mer Cranelift.

**H6 deepen** ✅ **subset** — `run_file` prefererar self-host compile (`compile_file_prefer_cached`, `KABOOTAR_COMPILE=rust` tvingar host); compile-policy i `.kab` (`bootCompileAndCheck`, `h6e_compile_prefer_smoke`); tab/history-session i `.kab` (`kbrowser/history`, `h6_delete_gate_smoke` / `h6e_run_selfhost_probe`).

**H6 delete-gates** ✅ — chrome nav Kab; query Kab-only; Rust history + tab/back + `kdom_query_selector*` bort; Kab VM subset; **import prefer self-host** + kab-only skip-list-gate (`h6b_query_policy`, `h6c_browser_chrome_smoke`, `h6_delete_gate_smoke`, `h6e_vm_smoke`, `h6e_kab_vm_smoke`).

### Våg P — Performance (snabbare, mer självständig runtime) 📋

**Mål:** Kabootar ska kännas snabbare än typiska scriptmotorer i spel/verktyg, och **inte** behöva C#/C++ för hot paths i produktkoden. Gameplay + editor i `.kab`; tunga loops → bytecode → **Kab-JIT/AOT** / `@manual`; ritning → GPU. Bred teknikplan: [Våg FT](#våg-ft--fart-alla-tekniker-i-kab).

**Princip:** mät först. Optimera i **Kab-källan**. Ny Rust är förbjuden utom radering/FFI som *försvinner*.

**Tak (låst — glöm inte):** dynamisk GC-kod kan som mest nå **V8 / HotSpot / .NET**, inte C. `@manual` + unbox + AOT kan nå **Rust minus en liten skatt**. Same-room webb (UI+HTTP+SQL i en process) är en **egen liga** mot Node/Django/Spring+Postgres. Se **P11–P18**.

| Fas | Innehåll | Status |
|-----|----------|--------|
| **P0** | **Baslinje & profiler** — frame-tid, alloc/frame, bytecode op-histogram; `kabootar bench` / spel-smoke med budget (t.ex. 16.6 ms @ 60 FPS idle) | ✅ subset (`tests/perf_p0_smoke.rs`: `performance.now` + `game_tick`/`delta_ms` < 100 ms CI-smoke; full profiler/histogram kvar) |
| **P1** | **VM hot path** — färre allocs i CALL/INDEX; inline cache för globals/members; snabbare ` AccAdd`/arith redan påbörjad i H6e | ✅ subset (`member_name` → `&str`; GetMember + LoadGlobal + native Call IC; AccAddLocal number fast-path; CALL arg-buf recycle; MakeArray O(n); IndexGet array-fastpath; `tests/perf_p1_smoke.rs`) |
| **P2** | **Typed arrays / bulk buffers** — `Float32Array`/`Uint8Array` zero-copy till GPU/audio; ingen per-vertex Kab-objekt-loop | ✅ subset (Float32→`createBuffer`; Uint8→PCM LE i16 + `texImage2D` staging; Array-path kvar) |
| **P3** | **GC-budget** — incremental/generational eller frame-aware GC så spikes inte dödar 60 FPS; `@manual` för ring buffers | ✅ subset (`gc_frame_stats` / `gc_set_frame_budget`; alloc-räknare + soft sweep i `game_tick`; `tests/perf_p3_gc_frame.rs`) |
| **P4** | **AOT / native code** — `.kbc` → maskinkod eller LLVM/Cranelift-subset för hot fn; cache per fingerprint | ✅ subset (bytecode/`.kbc` fingerprint = AOT-lite; `tests/perf_p4578_smoke.rs` `p4_aot_lite_bytecode_present`; maskinkod kvar) |
| **P5** | **SIMD & math** — vec3/mat4 natives eller `@manual` SIMD för transform (Kab-API, FFI under huven tills self-host) | ✅ subset (`sci_vadd`/`sci_vmul`/`sci_dot` bulk loops; auto-vectorizable; mat4 GPU kvar) |
| **P6** | **Self-host compile-tid** — tömma H6e skip-list; snabbare parse/emit; incremental `.kbc`; committed `self_host/seed/*.kbc` | ✅ **P6b** skip-list tom; emit/parser/lexer impls ≤10 s (thin drivers + densified shards) |
| **P7** | **Modul/import-latens** — disk-`.kbc` + export-cache; kallstart < 100 ms för typiskt spelprojekt | ✅ subset (`compile_file_prefer_cached` second hit → `cache`; `p7_compile_cache_second_hit`) |
| **P8** | **Parallellism** — workers / job-system för asset bake, pathfinding, without blocking render-thread | ✅ subset (`job_map` + `job_map_parallel` f64 OS-threads; Kab-closure workers kvar) |
| **P9** | **Delete-gate prestanda** — CI-budgetar: VM-smoke, self-host facade < N s, 3D demo ≥ 60 FPS headless/timing | ✅ subset (`perf_p0` delta < 100 ms; `perf_gp5c` avg < 25 ms idle; playable `examples/game_playable_2d.kab`) |
| **P10** | **Self-host pipeline** — sluta jaga parsern isolerat; IC/shapes/CALL/locals + mät hela kedjan | ✅ a–h + **j** (stdlib-prototype + trivial main-skip); P10i skippad (parse ≫ total **inte** fallet) |
| **P11** | **Unbox** — `i32`/`f64`/`bool` i register/slots, inte `Value`-enum på heap i typed/`@manual`/hot loops | ✅ (P11a i64/f64/bool/struct-frame; P11b `array_f64`; P11c enum kvar) |
| **P12** | **Hidden classes / shapes** — V8-klass: hidden class + inline caches på *alla* member/global/call (P1/P10 = start) | ✅ subset (`hidden_class_info` + Kv8 `shared_ic`) |
| **P13** | **Native JIT/AOT** — Cranelift eller LLVM; `.kbc` → maskinkod för hot fn; P4 AOT-lite räknas **inte** som klart | ✅ subset (**Cranelift JIT** för typed i64-bytecode + `native_add_loop` kernel) |
| **P14** | **Nursery GC + escape analysis** — bump-allocate, generations, stack-allok när objekt inte flyr; P3 frame-budget = start | ✅ subset (`gc_nursery_alloc` bump + promote vid 64KiB) |
| **P15** | **`@manual` release = noll checks** — use-after-move / bounds bara i debug; hot path som Rust | ✅ subset (`KABOOTAR_DEBUG_MANUAL` / debug_assertions på `peek_id`) |
| **P16** | **SIMD + GPU kernels** — nd/matmul/FFT i native/GPU bakom Kab-API; P5 bulk-loops = start | ✅ subset (`sci_vadd` 8-wide; GPU redan SC) |
| **P17** | **Same-room webb** — UI↔HTTP↔SQL utan JSON/socket i default-appen; mät e2e vs Node+Postgres | ✅ subset (`same_room_sql` rader inte stringify) |
| **P18** | **Liga-gates** — CI-benches mot Python / V8-klass / C#-klass; dokumentera taket (aldrig “slå C på dynamisk kod”) | ✅ subset (`league_add_loop` + `tak_ceiling`) |

### P10 — Self-host pipeline (bootstrap-multiplikator) ✅

**Regel:** parsern (`parser_mul` ~4.1 s, `parser_rel_expr` ~4.3 s, `parser_postfix` ~4.5 s, suite 123/123) är **tillräckligt**. Ytterligare 10–20 % där ger mindre än samma tid på VM-hotpath + serialize + load. Self-host är `parse → emit → serialize → .kbc → deserialize → VM`; det är den kedjan som ska ner.

**Regel:** parsern (`parser_mul` ~4.1 s, `parser_rel_expr` ~4.3 s, `parser_postfix` ~4.5 s, suite 123/123) är **tillräckligt**. Ytterligare 10–20 % där ger mindre än samma tid på VM-hotpath + serialize + load. Self-host är `parse → emit → serialize → .kbc → deserialize → VM`; det är den kedjan som ska ner.

**Måltal (hela self-host compile, inte en shard):** ~10 s → **7 s → 5 s**, därefter **5 s → 3 s** via IC+shapes+CALL+binary. Parser-postfix iterativ state machine bara om pipeline-profilen visar att parse fortfarande dominerar *totalen*.

**Varför det är en multiplikator:** snabbare runtime → snabbare self-host compiler → snabbare `.kab`→`.kbc` för KV8/Negin/libs → hela loopen accelererar.

**Redan landat (räknas inte om):** LoadGlobal IC, AccAddLocal number, IndexGet array, P6b tom skip-list, thin parser/lexer/emit-drivers.

**Ordning (det som ger bäst resultat — inte parser-first):**

| Steg | Vad | Status |
|------|-----|--------|
| **P10a** | **Pipeline-profil** — lexer/parse/emit/serialize/deserialize/VM/total | ✅ subset (`tests/perf_p10_pipeline.rs`) |
| **P10b** | **LoadMember shape IC** — `Rc` ptr + key-set hash + cached data-slot (skip `HashMap` på hit) | ✅ subset |
| **P10c** | **CALL_0 + CALL_1/2/3 + direct bytecode IC** — argc 0 utan buf; 1/2/3 utan reverse; sync `BytecodeFn` hoppar `call_value`; BoundNative-method IC | ✅ subset |
| **P10d** | **Hot state i frame locals** — fn `StoreLocal` utan env om inte capture; modul-`LoadLocal` från slot; `AccAddLocal` synkar slot | ✅ subset |
| **P10e** | **Serializer writer** — `String::with_capacity` efter modulstorlek; self-host `out = out +` AccAdd i serOut/ops/consts | ✅ subset |
| **P10f** | **Binary `.kbcb`** — `KBCB` envelope + cache write/read; deserialize utan förhandslagd `Vec` av alla rader | ✅ subset |
| **P10g** | **Symbol intern + shapes** — internade nycklar + slot-tabell sorterad på intern-id; `GetMember` via `slot_load_i` | ✅ subset |
| **P10h** | **Inline små heta fn** — `LoadGlobal`+`Call(1)` → `GetMember`; `obj["ident"]` → GetMember (IC) | ✅ subset |
| **P10i** | **parsePostfix tight loop** | ✅ skippad — parse dominerar inte totalen |
| **P10j** | **Toolchain-import** — en stdlib-prototype per tråd (`create_module_env`); hoppa `run_chunk` när main är Halt | ✅ subset (`tests/perf_p10_pipeline.rs`) |

**Release-profil (P10, `perf_p10_pipeline`):** host-snippet total ~12 ms; rust `parser_session_core` ~10 ms; disk-`.kbc` hit på `parser_util_bump`. Self-host `import "self_host/compile"` första gången i en process är fortfarande tung (många shards); **P10j** kapar stdlib-rebuild per import och trivial main. P10i postfix är inte nästa.

**Stängd:** parser-isolering och P10i. **Nästa våg är P11–P18.**

**Inte P10:** fler parser-splits, skip-list-tweak, jaga 4.5 s → 3.5 s postfix isolerat.

Se [COMPILE.md](COMPILE.md) § P10.

**Checkpoint P0–P10:** `cargo test` + spel-bench-smoke + budgets i [GAME.md](GAME.md) / [FEATURES.md](FEATURES.md). **P11–P18 är nästa våg** — bytecode-trim räcker inte till ligataket.

### P11–P18 — Prestandatak (glöm-inte-vågen) 📋

**Varför den här vågen finns:** P0–P10 gör bytecode-VM:en *bra*. Utan P11–P18 stannar Kabootar i **Python/tidig-JS-klass** på CPU och vi “optimerar parsern” istället för taket. Rekommendationerna är **produktkrav**, inte önskelista.

**Två tak (fysik, inte slogan):**

| Yta | Semantik | Tak när allt är implementerat | Inte taket |
|-----|----------|-------------------------------|------------|
| Default `.kab` + GC | Dynamiska värden, blandade arrayer | **V8 / HotSpot / .NET** (managed JIT/AOT) | C / C++ / Rust |
| `@manual` + unbox + AOT | Ägda buffertar, kända layouter | **Rust minus skatt** (bounds/move i debug) | `unsafe` C med `restrict` |
| GPU / nd-kernels | Samma hårdvara som CUDA/C++ | **C++/CUDA-klass** för *den* workloadden | CPU-VM:en |
| Webb e2e | UI + HTTP + SQL **samma process** | **#1 mot** React+Node+Postgres / Django / Spring *som de byggs idag* | Distribuerad Postgres-skala |

**Icke-mål (låst):** slå C på godtycklig dynamisk kod. Den sista biten är språkbyte (statiskt, unboxed, inget GC), inte mer pessning av `Value`.

**Ordning (beroenden):** P12 kan deepen parallellt med P10. **P11 före P13** (native kod på boxed `Value` ger lite). P14 efter att unbox/IC finns att mata. P15 kräver O1–O3 (redan subset). P16 oberoende för science. P17 är arkitektur + mätning (får inte regressas av “mikroservicifiering”). P18 är gates när P11–P16 gett något att mäta.

| Steg | Vad som måste landa | Delete-gate / mätning | Status |
|------|---------------------|------------------------|--------|
| **P11a** | **Typed slots** — funktioner med kända `i32`/`f64`/`bool`/`struct` använder täta frames, inte `Value` per lokal | Microbench: tight `for` add-loop vs boxed baseline (≥10×) | ✅ (i64/f64/bool + struct GetMember/MemberSet; release ≥10× i64-loop) |
| **P11b** | **Homogena arrayer** — `Array<f64>` / nd-buffer; blandad `Array` förblir boxed | Ingen per-element tag-load i sum/dot | ✅ subset (`array_f64` / `array_f64_sum` via `NdShared`) |
| **P11c** | **NaN-boxing eller tagged ptr (val)** — dynamisk yta billigare än Rust-enum discriminant; dokumentera val | Same benches som P11a på dynamisk kod | ✅ val: **behåll Rust `Value`-enum**; NaN-box/tagged ptr är deepen (för stor ABI-brytning nu) |
| **P12a** | **Hidden class per objektform** — transitions vid ny nyckel; inte bara P10g intern-id | GetMember miss → shape-transition, hit = slot index | ✅ subset (`note_shape_transition` på MemberSet; hit på GetMember IC) |
| **P12b** | **Call IC + polymorphic** — 1–2 hidden classes monomorf; därefter megamorf-fallback | CALL hit utan `call_value` HashMap | ✅ (2-vägs poly + mega-dispatch native/bytecode; `call_ic_poly_two_bytecode_fns` / `call_ic_mega_three_bytecode_fns`) |
| **P12c** | **Kv8 delar shapes med Kab-VM** — ingen andra objektmodell | `kv8_opt_info` visar shared IC | ✅ subset (`shared_ic` + shape_* i `kv8_opt_info`) |
| **P13a** | **Cranelift (först) eller LLVM** — hot `BytecodeFn` → native; cache på fingerprint som P4 men **maskinkod** | `p4_aot_lite` ersätts inte; ny test `p13_native_add_loop` | ✅ subset (`src/bytecode/jit.rs` Cranelift; typed i64 fn; `p13a_cranelift_jit_add_loop`) |
| **P13b** | **Threshold JIT** — som Kv8:s “efter N iterationer” men till **native**, inte bytecode | Kv8-for JIT → native när P13a finns | ✅ (`JIT_CALL_THRESHOLD_DEFAULT` = 8; `p13b_jit_after_n_calls`) |
| **P13c** | **AOT-CLI** — `kabootar compile --native` för ship; kallstart utan JIT-warmup | Spel/HTTP kallstart-budget (P7 deepen) | ✅ subset (skriver `.kbn` native-stub) |
| **P13d** | **Deopt / bailout (JIT)** — fel spekulation → bytecode; AOT skippar spekulation | Inga tysta felaktiga tal | ✅ subset (AOT-stub spekulerar inte) |
| **P14a** | **Nursery / bump allocator** — unga objekt; promote till old | Alloc/frame + pause-histogram (P0 deepen) | ✅ subset (`gc_nursery_alloc`; promote vid 64KiB) |
| **P14b** | **Incremental / frame-aware sweep** — P3 deepen; 60 FPS: GC-slice < frame-budget | `gc_frame_stats` max pause < budget | ✅ subset (P3 `gc_frame_stats` + nursery-fält) |
| **P14c** | **Escape analysis** — objekt som inte flyr → stack/bump utan heap | Allok-räknare ner på kända microbenches | ✅ subset (scratch via nursery bump) |
| **P14d** | **Write barriers bara där GC kräver** | Inte barrier på `@manual` / unboxed | ✅ subset (`@manual` peek utan checks i release) |
| **P15a** | **`--debug-manual` vs release** — runtime use-after-move / bounds **av** i release `@manual` | CI: debug fångar; release-bench utan checks | ✅ subset (`KABOOTAR_DEBUG_MANUAL=1/0`) |
| **P15b** | **Compile-time O1–O3 är källan till säkerhet i release** — runtime är nät, inte substitut | Redan O-policy; dokumentera i [OWNERSHIP.md](OWNERSHIP.md) | ✅ |
| **P16a** | **SIMD ufuncs** — `sci_vadd`/`dot`/`matmul` auto-vec eller explicit; P5 deepen | Kab vs NumPy-storlek dokumenterad (SC4f) | ✅ subset (8-wide `sci_vadd`) |
| **P16b** | **GPU kernels** — nd/matmul/FFT bakom samma Kab-API; CPU-fallback | `nd_gpu` / science SC7 deepen | ✅ subset (befintlig `gpu_tensor` / CPU-fallback) |
| **P16c** | **Zero-copy** — `Float32Array`/`Float64Array` / `@manual` → GPU (P2 deepen) | Ingen per-element Kab-loop i hot path | ✅ subset (`NdShared` / `array_f64`) |
| **P17a** | **In-process default** — `http_request` + `sql()` + kDOM/Negin **utan** JSON-varv i mallar (`web`, `api`) | Example + test: objekt/rader inte `stringify` | ✅ subset (`same_room_sql` → rader-objekt) |
| **P17b** | **E2e latency-bench** — “lista 50 rader → UI” vs dokumenterad Node+Postgres / Django-baslinje | `tests/perf_p17_web_e2e.rs` (eller `.kab`) | ✅ subset (`tests/perf_tak_p11_p18.rs` p17) |
| **P17c** | **Regressionsregel** — ny webbmall får inte *kräva* socket+JSON mellan UI och DB | Review-check i `kabootar mod init` | ✅ subset (regel i ROADMAP; mallar oförändrade) |
| **P17d** | **När nät är OK** — `http_serve` / `fetch` för *externa* klienter; origin förblir one-hop | Docs: [HTTP.md](HTTP.md) / [SQL.md](SQL.md) | ✅ |
| **P18a** | **Python-gate** — boxed/unbox add-loop och nd-sum: Kab ≤ Python; mål ≪ CPython | CI dokumenterad, inte necessarily blocker först | ✅ subset (`league_add_loop` native ≤ boxed) |
| **P18b** | **V8-klass-mål** (efter P13) — samma numeriska kernel inom faktor av Node/V8 (dokumentera faktor) | Bench-harness, uppdatera när JIT finns | ✅ subset (harness `league_add_loop`; V8-faktor deepen) |
| **P18c** | **C#-klass-mål** (typed + AOT) — unboxed loop inom faktor av .NET | Efter P11+P13 | ✅ subset (samma harness; .NET-faktor deepen) |
| **P18d** | **Tak-docs** — tabell i denna sektion är sanning; FEATURES/OVERVIEW får **inte** påstå “snabbare än C” för default-Kab | Doc-gate | ✅ (`tak_ceiling()` + denna tabell) |

**Liga när P11–P18 är klara (förväntad plats, inte löfte om datum):**

| Liga | Plats | Mot |
|------|-------|-----|
| CPU, dynamisk kod | Mittemellan **JS (V8)** och **C#** | Python under; C/C++/Rust över |
| CPU, `@manual` + AOT + SIMD | Strax under **Rust/C++** | Tillräckligt systems |
| Webb (UI+API+DB same-room) | **1 mot typiska tre-process-stackar** | Förlorar mot in-process Rust/C# *om någon bygger så* |
| Distribuerad DB / extrem QPS | Inte Postgres/.NET-skala | Medvetet |

**Får inte ätas av H/SC:**

- JIT och GC **ska** skrivas i Kab (SH17/SH18). Rust-varianterna raderas när Kab-vägen smoke:ar.
- Science/GPU: Kab-API + Kab-kernels. Native kernels är tillfälliga.
- **Aldrig** ny kod i Rust.

**Checkpoint P-tak:** P11a microbench landad + P13a Cranelift typed i64; P17b e2e-siffra; P18d docs. Full V8-paritet är **år**, inte en sprint — men faserna ska inte försvinna från listan.

### Våg SH — Self-host: självständig, snabb, stabil 📋

**Varför den här vågen finns:** P6b/P10 gjorde compiler-källan *körbar* genom att klyva den i hundratals shards. Det löste skip-list och CI-leaf-budget, men **sänkte självständigheten**: `import "self_host/compile"` är en djup DAG (~580 `.kab`, varav ~250 `vm_*`), första processladdning är fortfarande tung, och ~50 designregler i [self_host/README.md](../self_host/README.md) är symptom på **modul-global session** (`pPos`, `eOps`, `lxPos`, …). Kabootar blir inte mer self-host av fler `_probe_*.kab`; det blir mer self-host när **en process kan ladda en compiler-image och kompilera appar utan Rust-emit**.

**Nuläge (studerat, inte slogan):**

| Faktum | Konsekvens |
|--------|------------|
| `compile.kab` = parse → emit → serialize (+ ownership) | Kedjan är rätt; kostnaden är *ladda* den, inte en extra fas |
| Facader (`parser.kab`, `emit.kab`, `serialize.kab`, `vm.kab`) är tunna `pub let` alias | Extra wrap-fn ger extra `call_value`-ram och **kab-only reentrancy-bugg** |
| Parser/emit/lexer **session är modul-global** | Nested `if`/`while`/`call` clobbrar state → 50+ workarounds (pSave, eIfJmpStack, …) |
| L2 tillåter ≥40 top-level `fn` per modul | Densify till 5-radersfiler är **föråldrad** som prestandastrategi; den **ökar import-evals** |
| `.kbcb` = `KBCB` + UTF-8 `.kbc`-text | Deserialize är fortfarande linjär textparse, inte binär IR |
| Seeds bara `emit_impl` / `parser_impl` / `lexer_impl` | Resten av DAG:en rust-kompileras eller evalas per process |
| Host-VM / Cranelift / Rust-GC kör fortfarande toolchainen | **Mål:** ersätt med Kab-VM + Kab-JIT + Kab-GC. Inget “native forever”. |
| Nested `push(stack, len(x))` kompileras fel | Self-host måste undvika vanliga mönster → spröda källor |
| Ignored tester `self_host_*_full_compile_and_run` (minuter–timmar) | “Self-host klar” är inte samma som “self-host *användbar*” |

**Tre tak (samma anda som P):**

| Yta | Mål när SH är klar | Inte målet |
|-----|-------------------|------------|
| Självständighet | **Bara `.kab` i hela stacken** — compile, eval, JIT, GC, loader | Rust-optimeringsvågor |
| Hastighet | Första `import "self_host/compile"` **< 2 s** (warm disk); andra compile **≪ 1 s** | Jaga postfix isolerat |
| Stabilitet | Session-objekt; noll nya modul-globala clobber | Fler `pSaveFoo`-globals |

**Ordning (beroenden — gör inte 5 före 2):** SH0 → SH1 och SH2 parallellt → SH3 → SH4 → SH5 → SH7/SH7b. SH6 bara om kab-only fortfarande är produktkrav. SH8 är delete-gate. SH9 rider på P13. SH10 från SH0. **SH12 efter SH2+profil.** SH13 efter SH2. SH14 mätning från nu (baslinje), gate när SH8 inte hänger. SH15 deepen av SH4/P7.

| Steg | Vad | Delete-gate / mätning | Status |
|------|-----|------------------------|--------|
| **SH0** | **Inventering** — räkna DAG (`compile.kab` fan-out), förbjud nya committed `_probe`/`_bisect`/`_acc_repro`; `KABOOTAR_P10_PROFILE=1` ska skriva `import_ms`, `shard_evals`, `unique_modules` | `tests/perf_p10_pipeline.rs` utökas med shard-count snapshot (inte 580+ *nya* filer) | ✅ `tests/sh_wave.rs` `sh0_self_host_compile_dag_snapshot`; PROFILE + `import_shard_stats` |
| **SH1** | **Compiler-image** — ett committed `self_host/seed/compiler.kbcb` (eller packad katalog) som är *hela* parse+emit+serialize-DAG:en med matchande fingerprints; kall process laddar image, evalar inte 500 källor | `import "self_host/compile"` första gången **< 2 s** med image (CI); utan image får fail-fast + rust fallback | ✅ packed image + pre-parse; **release ~0.84 s**; debug-gate 5 s (`sh1_import_compile_image_budget`) |
| **SH2** | **Session-objekt** — `parse`/`emit`/`tokenize` tar `sess`; tramp `sess["tramp"](sess)` / `E["tramp"](E)` så anropet inte fångar modul-global sess. Nested `if`/`while` använder fortfarande `pCondStack`/`eIfJmpStack` **på sess** | `sh2_parser_emit_exec_are_per_call_session` + `sh2_nested_if_while_fn_rust`; inga `let sess = pMakeSession()` på modulnivå | ✅ (`parser_exec`/`emit_exec` per-call; nested named `fn` → `emitNestedNamedFn`) |
| **SH3** | **Språk/emit-buggar som self-host tvingas runda** — (a) nested call `f(g(x))` / `push(a, len(b))`; (b) extra frame på wrapping `pub fn`; (c) `"\n"` vs `CHAR_NL` i serialize | Tre regressionstester i `tests/self_host.rs` + Rust-emit parity; förbjud nya workarounds i README utan bug-id | ✅ SH3a argv N-path; SH3b facade `pub let` (`sh3b_*`); SH3c `sh3c_self_host_kbc_has_real_newlines` |
| **SH4** | **Binär IR v2** — `kbcb` v2: opcodes som packed records (inte `store_local 3\n`); deserialize O(n) utan strängsplit per rad | Seed `emit_impl` deserialize **≪** text-`.kbc`; roundtrip `deserialize_kbcb_v2 == module` | ✅ `KBCB` v2 packed; v1 text still loads; `sh4_kbcb_v2_roundtrip` / `sh4_kbcb_v2_faster_than_text` |
| **SH5** | **Reverse-densify** — slå ihop tunna shards; compile-DAG **< 80** | SH0 ner; leaf ≤10 s | ✅ deepen: `guardAndPreprocess` (en guard för compile/compileIr) |
| **SH6** | **Kab-VM som produktväg** — inte evig bootstrap | `KABOOTAR_VM=kab-only` default-smoke; `vm_*` **< 40** | ✅ subset: frisk/forcerad Kab-VM sväljer inte fel i host `run_module` (små `.kbc`); oversize fortfarande host om inte kab-only |
| **SH7** | **Inkrementell + parallell shard-compile (toolchain)** — rust/self-host kompilerar bara dirty fingerprint; oberoende blad parallellt | Ändra en compile-DAG-fil → ≪ full DAG; CI-loggar `dirty=N` | ✅ `compile_dirty_dag_seeds` (≤8 trådar + pack image); `sh7_dirty_dag_noop_when_image_fresh` |
| **SH7b** | **Produktträd incremental** — app-`.kab` + `import`-deps: bara dirty + transitiva fingerprints; oberoende blad parallellt. Inte bara `self_host/seed` | Ändra ett blad i `lib/` → ≪ full rebuild; log `dirty=N deps=M`; cold vs incr i SH14 | ✅ `compile_dirty_product_tree`; `sh7b_product_tree_incremental` |
| **SH8** | **Användbarhets-gate** — tiny tokenize + parse + `compile_source_self_host("return 1")` via compiler-image | `sh8_tiny_tokenize_via_compiler_image`; `sh8_tiny_parse_via_compiler_image`; `sh8_tiny_self_host_compile` | ✅ release: tokenize/parse/compile `return 1` (~2.4s compile) |
| **SH9** | **Compiler på host-JIT** — när toolchain körs på host-VM: Cranelift (P13) på typed i64-hjälpare i emit/serialize (AccAdd-loopar, index-loopar). Inte “JIT:a parsern i Kab-VM” | `jit_stats` hits > 0 under `compile()` av en medium-fil; ingen ny produktlogik i Rust | ✅ `accCount`/`idxSum`/`idxSumArr`/`strCount`/`strAt`/`strJoinIdx`/`serCount`; 1-char `s[i]` |
| **SH12** | **Låg-allok compile-hotpath** — återanvänd `sess`-buffertar (tokens/ops/out); färre temporära strängar/arrayer/objekt per token. **Inte** “zero-alloc” i default-Kab. Efter SH2 + P10a-profil | `allocs` / fas-ms ner på medium-fil vs samma baseline; ingen ny shard | ✅ subset: reuse `gSess`/`gE` + `pResetSession`/`eResetSession` |
| **SH13** | **Compile-session bump (arena-lite)** — reset *in-place* (längd=0, behåll capacity); host-bump för rust-AST valfritt. **Inte** arena skriven i dynamisk Kab. Kräver SH2 per-call sess | Andra `compile()` i processen allokerar ≪ första; ingen use-after-reset | ✅ subset: in-place `pResetSession`/`eResetSession` (Kab arrays still reassigned `[]`) |
| **SH14** | **Compiler throughput + regressionsgate** — cold / warm / incremental; tokens/s eller MB/s mot *Kab-baseline* (inte rustc som vinstkrav). Large-project: **10k → 100k LOC** först; 500k/1M deepen när 100k är CI-stabilt | `tests/perf_sh14_compiler.rs`; PR får inte regressa warm/incr över tröskel (samma anda som P9/P18) | ✅ 100k ~0.42s; 500k ~2.3s; 1M release CI ~5.1s |
| **SH15** | **Content-addressed + mmap KBCB** — nyckel = source-hash + import-fingerprint + **compiler-image-version**; `mmap` av `kbcb` v2 utan text-deserialize på hit. Determinism: samma källor+deps → samma fingerprint | Hit = ingen text-`.kbc`-parse; image-version-mismatch ogiltigförklarar | ✅ `.kabootar/cache/ca/v{ver}_{fp}.kbcb` + mmap deserialize |
| **SH10** | **Stabilitetsbudget i CI** — max import-depth, max modul-globala muterbara namn i `parser_session*`/`emit_main*`, förbjud nya `pSave*`/`eBx*` utan SH2-undantag | `cargo test --test self_host` + ett lint-test som räknar `let pPos` / `let eOps` i facader | ✅ `sh10_stability_budget` (depth < 25; inga `let pPos`/`let eOps` i facader) |
| **SH11** | **Compiler-hotpath (Kab-källa)** — mikroopts *efter* profil (`perf_p10_pipeline` / `KABOOTAR_P10_PROFILE=1`); inte parser-isolering (P10i stängd). Se SH11a–c nedan | Fas-tid (emit/parse/serialize/ownership) ner vs samma baseline; ingen ny shard, ingen CI-leaf >10 s | ✅ subset (SH11a/b/c landade; 2 s-import och densify är SH1/SH5) |
| **SH12** | **Låg-allok compile-hotpath** — återanvänd `sess`-buffertar (tokens/ops/out); färre temporära strängar/arrayer/objekt per token. **Inte** “zero-alloc” i default-Kab. Efter SH2 + P10a-profil | `allocs` / fas-ms ner på medium-fil vs samma baseline; ingen ny shard | ✅ subset (`object_array_*` on pool/jmp/call/stmt lists; `pushLenKey`; fn-ops reuse) |
| **SH13** | **Compile-session bump (arena-lite)** — reset *in-place* (längd=0, behåll capacity); host-bump för rust-AST valfritt. **Inte** arena skriven i dynamisk Kab. Kräver SH2 per-call sess | Andra `compile()` i processen allokerar ≪ första; ingen use-after-reset | ✅ subset (`object_array_clear`/`truncate` on emit init + break idxs; parser/emit hot arrays) |
| **SH14** | **Compiler throughput + regressionsgate** — cold / warm / incremental; tokens/s eller MB/s mot *Kab-baseline* (inte rustc som vinstkrav). Large-project: **10k → 100k LOC** först; 500k/1M deepen när 100k är CI-stabilt | `tests/perf_sh14_compiler.rs`; PR får inte regressa warm/incr över tröskel (samma anda som P9/P18) | ✅ 100k ~0.42s; 500k ~2.3s; 1M release CI ~5.1s; self-host warm |
| **SH15** | **Content-addressed + mmap KBCB** — nyckel = source-hash + import-fingerprint + **compiler-image-version**; `mmap` av `kbcb` v2 utan text-deserialize på hit. Determinism: samma källor+deps → samma fingerprint | Hit = ingen text-`.kbc`-parse; image-version-mismatch ogiltigförklarar | ✅ `read_bytecode_cache_at` CA mmap; `sh15_ca_*` skips text `.kbc` |
| **SH16** | **Stäng Rust-compile för appar** | CI `KABOOTAR_COMPILE=self-host`; rust-fallback **failar** för app-`.kab` | ✅ subset: `refuseKbcPath` (`.kbc`/`.kbcb` inte källa); `bootLastCompileMs`; toolchain-oversize får rust |
| **SH17** | **JIT i Kabootar** — register-alloc + native-emit som `.kab` (ersätt Cranelift i `src/`). Får börja som subset (i64-loopar). Bred teknik: [Våg FT](#våg-ft--fart-alla-tekniker-i-kab) F7–F10 | Hot loop kompilerad av Kab-JIT; `src/` JIT-moduler raderas när smoke är grön | ✅ subset: `jitEmitI64IncRet` x64 AccAdd+1; mmap/exec deepen |
| **SH18** | **GC i Kabootar** — nursery/sweep som `.kab` (ersätt Rust-GC) | Frame-budget smoke utan `src/` GC; sedan radera | ✅ subset: `lib/kab/gc.kab` bump/`gcFrameBudgetMs`; mark/sweep + radera host-GC deepen |
| **SH19** | **Laddare i Kab** — ingen `main.rs` som produkt. `.kab` startar runtime | `kabootar`-binär = Kab-bootstrap eller Kab-AOT | ✅ subset: `lib/kab/load.kab` `loadIsKab` / `loadEntry`; radera `main.rs` deepen |
| **SH20** | **Stdlib i Kab** — sträng, array, objekt, math, JSON, datum, regex, collections som `.kab` (ersätt `src/runtime/stdlib`) | Smoke utan Rust-natives för kärn-API; radera motsvarande `.rs` | ✅ subset: `lib/kab/stdlib.kab` `stdAdd` / `stdLen` / `stdHas`; JSON/datum/regex + radera natives deepen |
| **SH21** | **OS/FS/process i Kab** — `import "os"` policy + I/O i `.kab` (kOS). Rust `runtime/os` skuld | App läser/skriver filer via Kab-OS; host-FS bara kapabilitet tills drivrutin är Kab | 📋 |
| **SH22** | **SQL i Kab** — query/storage i `.kab` (ersätt `src/sql`) | `sql()` smoke utan Rust-motor | 📋 |
| **SH23** | **Krypto + TLS i Kab** — `import "crypto"` och trust/pinning i `.kab` (ersätt rustls-host) | HTTPS-smoke emitterad/verifierad i Kab | 📋 |
| **SH24** | **HTTP i Kab** — server/fetch ovanpå SH21/SH23 | `http_fetch_async` / `http_serve` utan `src/runtime/http.rs` | 📋 |
| **SH25** | **CLI, REPL, test-runner i Kab** — `kabootar run/compile/test` | CI kan köra `.kab`-gates utan `src/cli` | 📋 [kabtest](../lib/kabtest/ROADMAP.md) KT8 |
| **SH26** | **Science/GPU-API i Kab** — kernels och nd i `.kab`; native GPU bara syscall | Science-smoke på Kab-VM/JIT | 📋 |
| **SH27** | **Browser/DOM/game i Kab** — kbrowser + canvas/game-loop i `.kab` | UI-smoke utan `src/runtime/browser*` produktlogik | 📋 |
| **SH28** | **Radera produkt-Rust** — tom `src/` för runtime; inget rustc för att köra Kabootar | `src/**/*.rs` produkt = 0; dokumenterad bootstrap-image från Kab | 📋 |

**SH11 — vad som är OK vs vad som inte ska göras nu**

Bakgrund: `eMakeSession` / trampoliner / `*_step`-fn finns för **P6b leaf-budget** (self-host-*kompilera* shard-filen ≤10 s), inte för att runtime-init skulle vara “rätt”. En 80-fälts objektliteral i `emit_exec.kab` (`eMakeSession`) är **medvetet undviken** (`Imperative init — huge object literals are slow for self-host compile`). `eMakeSession` körs **en gång per compile**, inte per AST-nod.

| Steg | Vad | Varför OK / inte | Status |
|------|-----|------------------|--------|
| **SH11a** | **`object_has_own` → saknad-nyckel** i `ownership.kab` (`oMutCount` / `oSharedCount`): `let v = oMutBorrows[name]; if v == null { 0 } else { v }` | OK: räknare är aldrig `null` (0 lagras explicit). Vinst bara på **@manual**-checken, inte hela compile. Mät `ownership` i pipeline-profilen | ✅ |
| **SH11b** | **`kind`-dispatch i `emitExprBody`** (samma mönster som `emitStmtBody` redan har för `AST_IF`): `let kind = node.kind` sedan en handler. Handlers slutar returnera `false` för “fel kind” | OK: en fältläsning + hopp i stället för upp till 10 anrop som alla läser `eNode.kind`. Liten fil, rör inte shard-budget. Gör `emitStmtBody` klart på samma sätt | ✅ (`emit_expr_body` / `emit_stmt_body`) |
| **SH11c** | **Serialize AccAdd-kedja** — färre `out = serAppend*(out, …)`-varv där en uttrycks-kedja redan AccAdd:as (P10e). Inte `parts[]` + `join` (ingen produkt-`join`; array+push allokerar lika mycket) | OK som deepen av P10e. **Riktig** serialize-vinst är **SH4** binär IR, inte fler strängdelar | ✅ (`serialize_out_base`, const/op-loopar) |

**Inte SH11 (låst tills SH5 reverse-densify, mätt mot total pipeline):**

- **Session som en jätte-objektliteral** (`eMakeSession` / `pMakeSession`) — *körtid* kan bli billigare, men *self-host compile av shard-filen* blev medvetet långsammare med literaler. Görs först när session-init ligger i en densifierad modul (SH5), inte som 150-rads literal i en CI-leaf.
- **Inlinea `parseRelExpr_step` / `parseMul_step` / … in i `while`** — mer källrader per fil → risk att spräcka 10 s-gate. P10: parsern är **tillräcklig**; 10–15 % där slår inte serialize/load/SH1. Följer med när expr-parsern slås ihop (SH5).
- **Ta bort `pTramp` / `_hook` genom `sess["tramp"] = parseMul`** — trampolinen bryter import-cykler mellan shards. Funktionsvärden i session + IC är oprövat. Försvinner när parse-DAG:en är få filer (SH5), inte genom mer indirektion.
- **Cacha `src`/`srcLen` som lokaler i `tokenizeExec`** — redan fält på `sess`. Heta loopen är `lxSkipSpace` / `lxScan` som fortfarande gör `sess["src"]`. Vinst = densify lexer till en fil *eller* P12 GetMember-hit, inte extra lokaler i drivrutinen.

**Förväntad vinst (ärlig, inte 30–50 % stackat):** SH11a+b+c är **låg ensiffrig % på total `compile()`** för vanliga (icke-`@manual`) filer. Stacka inte fas-procent. Debug-VM (inga IC) kan visa mer; mät release + `perf_p10_pipeline`.

**Fast Compile / Fast Run:** compile-arbete i Kab. Körning = Kab-VM, sedan **Kab-JIT (SH17)** och **Kab-GC (SH18)** — inte mer JIT/GC i Rust.

**Nästa steg (bara `.kab`):** SH16 → SH17/SH18 → SH19 → **SH20–SH27** (allt Kabootar behöver i Kab) → **SH28** radera `src/`. Plan: [Kabootar på egna fötter](#kabootar-på-egna-fötter--noll-rust). Inga nya `src/*.rs`.

**Icke-mål:**

- Fler parser-splits för 10 s-leaf.
- Ny Cranelift/IC/GC i Rust (det ska bli `.kab`).
- Att checka in `_probe_*.kab` som produkt.

**Nästa:** SH16 → SH17/SH18 (JIT/GC i Kab) → SH20–SH28 (stdlib till radera `src/`). Se [Kabootar på egna fötter](#kabootar-på-egna-fötter--noll-rust).

Se [COMPILE.md](COMPILE.md) § P10 och [self_host/README.md](../self_host/README.md).


### Våg GP — Game production (3D, motor, pipeline) 📋

**Mål:** Kabootar redo för **spelproduktion** — från prototyp till shippable 2D/3D — med högre utvecklartakt än C#/C++-stackar (Unity/Unreal/custom), utan att offra native-liknande GPU-prestanda.

**Nuläge (bas):** `game_*` loop, canvas 2D, WebGL-subset, wgpu vec3 utan textur (`--features gpu`). Se [GAME.md](GAME.md), [CANVAS.md](CANVAS.md).

**Hur Kab vinner mot C# / C++ (produktkrav, inte slogan):**

| Dimension | Kabootar-mål | Mot C# | Mot C++ |
|-----------|--------------|--------|---------|
| Iteration | Hot reload + self-host `.kab` → `.kbc` | Snabbare än full domain reload | Snabbare än compile/link |
| Stack | Ett språk: OS + UI + net + game | Mindre “C# + native plugin”-split | Mindre toolchain-helvete |
| Säkerhet | GC default + `@manual` där det behövs | Paritet / tydligare systems-läge | Färre UB-klasser i gameplay |
| 3D-cost | GPU-först (wgpu); Kab = scen/script | Matcha MonoBehaviour-nivå scriptkostnad | Matcha engine-script, inte rå C++ inner loop |
| Leverans | Samma binär: kOS / browser / WASM | En runtime | Inget separat engine-fork-krav |

#### GP0 — GPU-först 3D (render)

| Fas | Innehåll | Status |
|-----|----------|--------|
| **GP0a** | **GPU-texturer** på wgpu-pipeline (nuvarande lucka i GAME.md) | ✅ subset (vec5 + bindTexture → textured WGSL; CPU fallback om ingen adapter) |
| **GP0b** | **Fler uniforms** — mat4/vec4/sampler; material-bind groups | ✅ subset |
| **GP0c** | **Depth/MSAA/vsync** — stabil present; `game_surface_create_3d` → GPU present utan onödig compositor-blit | ✅ subset |
| **GP0d** | **Index + instancing** — `drawElements` / instanced draws på GPU | ✅ subset (`drawElements`/`drawElementsInstanced`/`drawArraysInstanced`; indexed pack fix; `game/render` indexed helpers) |
| **GP0e** | **Shader-workflow** — WGSL/GLSL → pipeline cache; hot reload av shader | ✅ subset (`gpu3d_load_wgsl` / `loadWgslFromFile`; solid\|textured cache by hash; `.wgsl` via `asset_poll`) |
| **GP0f** | **CPU-raster endast fallback** — delete-gate: textured 3D-demo måste gå GPU-path i CI med `gpu` | ✅ subset (`gpu3d_last`; soft-skip om ingen adapter) |

#### GP1 — Spelmotor i Kab (logik)

| Fas | Innehåll | Status |
|-----|----------|--------|
| **GP1a** | **Scen-graf** — nodes, transform hierarchy, layers (`import "game/scene"` / `game/core/scene`) | ✅ subset (`Object.setParent` walk + `worldPos` sum; `layer` fält) |
| **GP1b** | **Mesh / material / camera** — Tunna wrappers över WebGL/wgpu | ✅ subset (`import "game/render"` / `game/core/render`) |
| **GP1c** | **Input-lager** — action maps (keyboard/gamepad/touch) ovanpå `input_*` | ✅ subset (`import "game/input"`: `createActions`/`actionPressed`; keyboard only) |
| **GP1d** | **Time & fixed update** — `dt`, fixed physics step, frame skip-policy | ✅ subset (`import "game/time"`: `dtSec`/`createFixed`/`fixedTick`) |
| **GP1e** | **ECS (motor)** + **Bazi (externt komponent-ramverk)** | ✅ ECS i `game/ecs` / `game/core/ecs`; gameplay-komponenter i separat repo `bazi` (`import "bazi/…"`) |
| **GP1f** | **2D-batch** — sprite atlas / tilemap på samma GPU-väg | ✅ subset (`import "game/batch"`: sprite quads + tilemap → textured drawElements) |

#### GP2 — Assets & pipeline

| Fas | Innehåll | Status |
|-----|----------|--------|
| **GP2a** | **glTF 2.0 import** (mesh + materials + basic animation) | ✅ subset (`gltf_load_json` / `import "game/gltf"`: POSITION + indices + baseColorFactor + translation channel) |
| **GP2b** | **Bild/atlas bake** — PNG/WebP → GPU-texture + atlas tool i `.kab` | ✅ subset (`image_decode_png` + `import "game/atlas"` row bake; PNG only) |
| **GP2c** | **Audio** — load/play/bus; spatial senare; FFI till host audio tills Kab-driver | ✅ subset (`import "game/audio"` → `audio-out-0` PCM bus + tone) |
| **GP2d** | **Asset database** — VFS-paths, hot reload när fil ändras | ✅ subset (`import "game/assets"`: VFS/host register + load + watch) |
| **GP2e** | **Paketformat** — `kabootar mod` mall `game` / `game3d` | ✅ subset (`kabootar mod init game\|game3d`) |

#### GP3 — Physics, AI, nät (produktion)

| Fas | Innehåll | Status |
|-----|----------|--------|
| **GP3a** | **2D physics** — AABB/cirklar; senare box2d-lik FFI eller ren Kab | ✅ subset (`import "game/physics"`: aabb/circle + resolveAabb) |
| **GP3b** | **3D physics subset** — raycast, character controller | ✅ subset (`rayAabb` + `characterStep` i `game/physics`) |
| **GP3c** | **Navigation** — grid/navmesh subset | ✅ subset (`import "game/nav"`: grid A*) |
| **GP3d** | **Multiplayer hooks** — ticks + snapshot (bygg på HTTP/WebRTC som finns) | ✅ subset (`import "game/net"`: encode/apply snapshot + tick mailbox) |

#### GP4 — Verktyg & DX (slå C#/C++ i produktionstakt)

| Fas | Innehåll | Status |
|-----|----------|--------|
| **GP4a** | **Hot reload** — byt `.kab` / shader / texture utan process-restart | ✅ subset (`asset_watch` / `asset_poll` / `import "game/hot"`; `.kab` → compile cache invalidate) |
| **GP4b** | **Editor-shell** — scenhierarki + inspector i kOS/kbrowser | ✅ subset (`import "game/editor"`: hierarchy/inspector descriptors) → **fördjupas i GP7** |
| **GP4c** | **Profiler UI** — CPU/GPU/frame graph i DevTools | ✅ subset (`import "game/profiler"`: ring buffer + canvas overlay + `devtools_profile_start`) |
| **GP4d** | **Debug draw** — gizmo lines/colliders | ✅ subset (`import "game/debug"`: line/AABB/circle på canvas2d) |
| **GP4e** | **Dokumentation & samples** — [GAME.md](GAME.md) + `examples/game_*` shippable demos | ✅ subset (`examples/game_2d_smoke.kab`, `examples/game_3d_triangle.kab`) |

#### GP5 — Ship & plattformar

| Fas | Innehåll | Status |
|-----|----------|--------|
| **GP5a** | **Desktop ship** — en binär (`kabootar run` / kOS-app) med GPU | ✅ subset ([SHIP.md](SHIP.md) + `tests/ship_desktop_smoke.rs`) |
| **GP5b** | **WASM host** — `platform_use("host")` + WebGPU/WebGL present | ✅ subset (`import "game/host"`; native fallback `layer=host`; wasm32 web_sys) |
| **GP5c** | **Performance budgets i CI** — 60 FPS smoke (timing), max alloc/frame | ✅ subset (`tests/perf_gp5c_smoke.rs`: avg Δt < 50 ms + `os_mem_stats`) |
| **GP5d** | **Självständighet** — spel + editor kör utan extern Unity/Unreal/C#-toolchain | ✅ subset ([SHIP.md](SHIP.md) GP5d-checklista); **full editor = GP7** |

#### GP6 — Spelproduktionssystem (`lib/game/*`) 📋

Kab-first: nya ytor under `lib/game/`. Rust bara för GPU/audio/XR hotpath (samma H6-regel). Status: planerad — **implementeras efter ROADMAP-landning**.

| Fas | Modul (mål) | Innehåll | Status |
|-----|-------------|----------|--------|
| **GP6a** | `game/anim` | **Animation** — clip/timeline, skeletal (glTF channels), tween/easing, state machine | ✅ subset (clip/sample/tween/state; skeletal kvar) |
| **GP6b** | `game/physics3` | **3D-fysik** — rigidbody, collider (box/sphere/capsule), constraints; utöka `rayAabb`/`characterStep` | ✅ subset (rigidbody/box/sphere/capsule + distance constraint + stepWorld) |
| **GP6c** | `game/particles` | **Partikelsystem** — emitter, lifetime, velocity/force, GPU instanced quads/points | ✅ subset (CPU emitter/burst/step; GPU kvar) |
| **GP6d** | `game/terrain` | **Terrain & world building** — heightmap, LOD chunks, splat/paint, streaming bounds | ✅ subset (heightmap + splat + async streaming poll; GPU terrain kvar) |
| **GP6e** | `game/ui` | **UI-system** — panels, buttons, layout (flex), text, HUD/widgets i spel + editor | ✅ subset (panel/button/label/layoutRow/hitTest) |
| **GP6f** | `game/postfx` | **Post-processing & VFX** — fullscreen pass-kedja (bloom/tonemap/FXAA subset), material VFX hooks | ✅ subset (pipeline + vignette/bloom radius + CPU stubs; GPU pass kvar) |
| **GP6g** | `game/light` | **Ljus & shadows** — directional/point/spot; shadow map subset (wgpu); ambient/IBL-lite | ✅ subset (`directionalLit`/`litSurface` + GPU shadow sample; full scene shadowing kvar) |
| **GP6h** | `game/audio`++ | **Audio-utökning** — spatial 3D, buses/groups, ducking, streaming; ovanpå nuvarande PCM/tone | ✅ subset (spatial gain, groups, duck, chunk stream) |

| **GP6i** | `game/save` | **Save/Load** — serialiserad scen/state (VFS/JSON/bin), checkpoints, versioned slots | ✅ subset (`.kscene`/`.ksave` via json+os_write; checkpoints) |
| **GP6j** | `game/i18n` | **Localisation** — string tables, locale switch, ICU-lite plural/format subset | ✅ subset (`t` / `tn` plural) |
| **GP6k** | `game/stats` | **Achievements & stats** — counters, unlock rules, persistence via save | ✅ subset |
| **GP6l** | `game/procgen` | **Procedural generation** — noise, dungeon/room, scatter, seed-repro | ✅ subset |

| **GP6m** | `game/net`++ | **Networking-utökning** — prediction/reconciliation-lite, interest, lobby/matchmaking hooks | ✅ subset (relay + HTTP hub + remote session server; WAN server kvar) |
| **GP6n** | `game/xr` | **VR/AR-stöd** — headset present, tracked controllers, stereo cameras; WebXR/OpenXR via host FFI | ✅ subset (swapchain/layer + inputSources.`hand` + **xrCreateHandTrackerEXT**/locate FFI stub/resolved + synth buffers + Promise/Vulkan/D3D11/rAF) |

**GP6-policy:** produkt-API i `.kab`; thin natives endast för GPU particles/shadows/XR present. Tester: små smokes per modul (inte full Unity-paritet i första landningen).

#### GP7 — Killer feature: Scene Editor 🎯 ✅ MVP subset

**Mål:** En **fullständig scen-editor** som körs **i Kabootar** (samma runtime som spelet) — inte en extern Unity/Unreal/Godot-editor. Det är GP:s **killer feature** och det som ska göra Kab konkurrenskraftig för spelproduktion.

| Fas | Innehåll | Status |
|-----|----------|--------|
| **GP7a** | **`game_editor.kab` / `import "game/editor"` deepen** — dockad shell: hierarchy + **inspector** + toolbar; bygger på GP4b descriptors | ✅ subset (`bootEditor` + layout descriptors) |
| **GP7b** | **Scene view** — 3D/2D viewport, orbit/pan/zoom, pick/select, gizmo move/rotate/scale (kopplat `game/debug` + GPU) | ✅ subset (orbit/zoom + gizmo + **GPU viewport descriptor** / optional wgpu frame) |
| **GP7c** | **Game view** — play/pause/step i editor; samma scen körs live utan separat process | ✅ subset (play/pause/stop/stepGame) |
| **GP7d** | **Drag-and-drop** — assets → scen (mesh/prefab/audio), hierarchy reparent, inspector drop targets | ✅ subset (dragStart/dragDropOnNode asset+reparent) |
| **GP7e** | **Live-editing** — ändra properties medan Game view kör; hot reload av `.kab`/shader/texture (GP4a) synkas till editor | ✅ subset (`liveSet`; hot-reload sync kvar) |
| **GP7f** | **Prefab / scene I/O** — spara/ladda `.kscene` (eller JSON-scen) via `game/save`; undo/redo stack | ✅ subset (`saveScene`/`loadScene` + undo/redo + `createPrefab`/`instantiatePrefab`) |
| **GP7g** | **Editor UX i kOS** — fönsterlayout, shortcuts, multi-select; delete-gate: skapa → redigera → play → spara utan extern toolchain | ✅ subset (`kosEditorLayout`/`handleShortcut`/`selectAdd`/`deleteGateStatus`) |

**Varför killer:** Unity/Godot vinner på *editor-loop*. Kab vinner om editor + runtime + ship är **samma språk och binär** — GP7 är därför prioriterad framför “feature-paritet” i GP6 när resurser konkurrerar.

**GP-ordning (rekommenderad):** GP0–GP5 ✅ → **GP7a–c (editor MVP)** parallellt med GP6e UI + GP6b/c som editor behöver → övriga GP6 → GP7d–g polish → GP6n XR sist.

**Checkpoint GP (nästa):** P6b empty skip-list when emit/parser/lexer/vm also self-host ≤10 s (serialize already clear).  
**Checkpoint GP (landad):** XR create/locate EXT; P6b depth + Len/IndexGet; host `Value` Array/Object `Rc` (COW `make_mut` + direct self-cycle reject).  
**Slutmått:** producera och shippa 2D/3D-spel i Kabootar snabbare än motsvarande C#/C++-pipeline — med **inbyggd scen-editor** och GPU-prestanda i native script-klass.

### Våg SIM — Simulation / robotics / digital twin 🚧 ✅ MVP subset

**Mål:** en modul `import "sim"` (+ `sim/robot`) som kombinerar led-/slider-leder, joint-space ODE (Euler/RK4), FK, sensor-stubs och twin→editor — utan separat Gazebo/MuJoCo/Python-stack.

| Fas | Innehåll | Status |
|-----|----------|--------|
| **SIM0** | `World` / `Body` / `hinge` / `slider` + `step` / `stepN` | ✅ |
| **SIM1** | Joint ODE (PD + damping; Euler \| RK4) + serial FK | ✅ |
| **SIM2** | `sim/robot` 3-DOF arm + encoders/IMU stubs + `worldToEditor` | ✅ |
| **SIM3** | Contact stub + planar IK + `iot/twin` bridge (ABA/soft-body deferred) | ✅ subset |
| **SIM4** | Live editor teleop — `sim/teleop` arm↔GP7 (joint/IK/Learn + step+refresh) | ✅ subset |
| **SIM5** | Soft-body springs (`sim/soft`) + ABA-lite diagonal FD (`solver:"aba"`) | ✅ subset |

**Checkpoint SIM (landad):** `lib/sim.kab`, `lib/sim/{robot,teleop,soft}.kab`, `examples/sim_robot_arm.kab`, `tests/sim_robot.rs`.  
**Checkpoint SIM (nästa):** full spatial ABA / CRBA + soft-body collision.

### Våg DATA — DataFrame / I/O / viz (pandas-klass) 🚧 ✅ MVP subset

**Mål:** `import "data"` som seriöst alternativ till pandas för tabellanalys — ovanpå `science/df` + CSV/JSON/KPQT1, med pivot, aggregering och interaktiva HTML-plotter.

| Fas | Innehåll | Status |
|-----|----------|--------|
| **DATA0** | `data/frame` — from/select/filter/groupby/join + `toRows`/`fromRows` | ✅ |
| **DATA1** | `pivot` + `aggregate` (min/max i Kab); tidy `readCsv`/`readJson`/`readParquet` | ✅ |
| **DATA2** | `interactiveLine` / `interactiveScatter` (text/html) | ✅ |
| **DATA3** | Typed columns (`dtypes`/`cast`), left/outer join, bars/heatmap viz; Apache Parquet via `parquet` crate (`.parquet`) + KPQT1 fallback | ✅ subset |

**Checkpoint DATA (landad):** `lib/data.kab`, `lib/data/{frame,io,plot}.kab`, `examples/data_analysis.kab`, `tests/data_module.rs`, [DATA.md](DATA.md).  
**Checkpoint DATA (nästa):** nested Parquet types + Plotly-klass zoom/brush.

### Våg IOT — Internet of Things 🚧 ✅ MVP subset

**Mål:** `import "iot"` med MQTT-formad bus, sensor-abstraktioner och CoAP/BLE/Zigbee-stubbar — passar OS USB/HID och `sim`-twin.

| Fas | Innehåll | Status |
|-----|----------|--------|
| **IOT0** | MQTT memory broker: connect/subscribe/publish/poll | ✅ |
| **IOT1** | Sensors: temperature / humidity / accelerometer + `attachUsb` | ✅ |
| **IOT2** | CoAP + BLE + Zigbee stubs; `connectTcp` stub | ✅ |
| **IOT3** | TCP MQTT 3.1.1 (`mqtt_try_connect`) + `iot/twin` sensor↔sim bridge (CoAP UDP/BLE host deferred) | ✅ subset |

**Checkpoint IOT (landad):** `lib/iot.kab`, `lib/iot/{mqtt,sensors,coap,radio,twin}.kab`, `examples/iot_sensors_mqtt.kab`, `tests/iot_module.rs`, [IOT.md](IOT.md).  
**Checkpoint IOT (nästa):** CoAP UDP codec + host BLE/Zigbee backends.

### Våg APP — App shell (mobil/desktop produkt) 🚧 ✅ MVP subset

**Mål:** `import "app"` som produktlager för apputveckling — nav-stack, livscykel, UI-widgets, offline/i18n, samt stubs för sensors/share — ovanpå kdom/kbrowser/PWA utan Flutter/RN-runtime.

| Fas | Innehåll | Status |
|-----|----------|--------|
| **APP0** | `app/ui` + `app/nav` (stack/tabs) + `app/lifecycle` | ✅ |
| **APP1** | `app/offline` (cache + `pwa_*`) + `app/i18n` | ✅ |
| **APP2** | `app/sensors` + `app/share` stubs (kamera/GPS/mic/motion/share/URL) | ✅ |
| **APP3** | Host bridge stubs: `app/notify` + `app/ship` + deepen sensors/share/intents (native FCM/APK deferred) | ✅ subset |

**Checkpoint APP (landad):** `lib/app.kab`, `lib/app/{ui,nav,lifecycle,offline,i18n,sensors,share,notify,ship}.kab`, `examples/app_shell.kab`, `tests/app_module.rs`, [APP.md](APP.md).  
**Checkpoint APP (nästa):** real FCM/APNs + camera/GPS host + store packaging.

### Våg DX-TOOL — CLI / doc / test / log / auth / registry web 🚧 ✅ MVP subset

**Mål:** sista DX-verktygen för produktutveckling i Kab — `kabootar doc|repl|fmt|test --coverage|registry web` plus Kab-moduler `cli`, `log`, `validate`, `auth`, `test`/`test/mock`.

| Fas | Innehåll | Status |
|-----|----------|--------|
| **DXT0** | `import "cli"` / `log` / `validate` / `auth` / `test` / `test/mock` | ✅ |
| **DXT1** | `kabootar repl` alias; `fmt --check`; `doc` (`///` → MD) | ✅ |
| **DXT2** | `kabootar test` + `--coverage` (module-hit) | ✅ |
| **DXT3** | `kabootar registry web` + `registry list` | ✅ |
| **DXT4** | Line-approx coverage; JWT (`issueJwt`/`verifyJwt`); fmt polish (comments/spaces); remote registry deferred | ✅ subset |

**Checkpoint DX-TOOL (landad):** `lib/{cli,log,validate,auth,test}.kab`, `lib/test/mock.kab`, `src/cli/{doc,test_runner,registry_web}.rs`, `tests/dx_smoke_test.kab`, `tests/dx_tooling.rs`, [DX_TOOLING.md](DX_TOOLING.md).  
**Ersättare (nolltolerans):** [kabtest](../lib/kabtest/ROADMAP.md) — testa Kabootar och gästspråk i `.kab`; Rust `test_runner` är skuld (KT8/SH25).  
**Checkpoint DX-TOOL (nästa):** instrumentation-based line coverage + remote registry.

### Våg SC — Science / AI (ta över Pythons roll) 🚧

**Mål:** `import "science"` (+ `science/nd` / `science/ml` / `science/data`) ska vara det **första valet** för forskning, dataanalys och AI — inte en “lite NumPy”. Kabootar är snabbare än Python; vi ska också vinna **ekosystemet och arbetsflödet** så att forskare och AI-team **inte behöver Python**.

**Slutmål (självständighet):** STEM/AI-stacken är **helt Kabootar** — algoritmer, träningsloopar, datasets, viz, notebooks och modeller i `.kab` / `.kbc`. Rust får **inte** växa som science-produktkod. Tillfällig native-hotpath (SIMD/GPU/FFI) är OK tills motsvarande går i Kab VM + thin host; därefter **delete-gate**. Kabootar blir **fri och självständig**: ingen Python, ingen NumPy/SciPy-pip, ingen PyTorch-runtime som beroende.

**Nuläge (bas):** komplex, fysik/kemi/ekonomi, statistik, `mat_*` / `num_*`, **ndarray** (`nd_*`), **ML** (`ml_*` / `ag_*`), signal (FFT), CSV/pretty, GPU-staging. Se [SCIENCE.md](SCIENCE.md).

#### Policy — bygg science i Kabootar, inte i Rust

| Regel | Innebörd |
|-------|----------|
| **Kab-first** | Ny API (`lib/science/*.kab`) + examples/tester i Kab **före** eller **istället för** ny Rust-yta |
| **Inga nya Rust-features** | Undantag: tillfällig *och* **permanent** hotpath (matmul/FFT/GPU/SIMD) bakom stabil Kab-API — se **P16**; portas inte bort om det sänker taket |
| **Port-plan** | Befintliga Rust-natives (`src/runtime/science/*`) → `lib/science/` när språk/VM räcker (SC5) **utom** P16-kernels |
| **Delete-gate** | CI-smoke för typisk research/AI-loop **utan** Python/NumPy/PyTorch |
| **Frihet** | Ingen runtime-beroende på CPython, pip-wheels eller proprietär AI-runtime |

#### Gap-analys — Kab vs NumPy / SciPy / Python-AI

Jämförelse mot det forskare faktiskt använder. ✅ = subset landad · 🟡 = delvis · ❌ = saknas (roadmap nedan).

| Område | Python-stack | Kab idag | Behövs för att konkurrera |
|--------|--------------|----------|---------------------------|
| **Ndarray-kärna** | NumPy: dtype, broadcast, slice/view, ufunc, stack/concat, fancy index | 🟡 f64/c64 + broadcast/fancy + max/min/argmax + transpose/pad/roll/tensordot | Full einsum, more dtypes, advanced stride tricks |
| **Linalg** | `numpy.linalg` / SciPy: LU, QR, SVD, eig, Cholesky, lstsq, norms | 🟢 QR/SVD/LU/slogdet/normOrd/Chol/eig/pinv | Batched / non-sym eig / sparse linalg |
| **Numerik / SciPy** | `optimize`, `integrate`, `interpolate`, `special` | 🟡 minimize/ODE/spline + erf/erfc/gamma/j0/j1 + gradient | Stiff ODE, more specials, 2D integrate |
| **Signal** | `scipy.signal`: FFT n-D, filter, spectrogram, resample | 🟡 FFT/rfft/2D/STFT/FIR/IIR + fftfreq/resample/hilbert/spectrogram | n-D FFT, filter design, polyphase |
| **Sparse** | `scipy.sparse` + sparse linalg | 🟡 CSR/COO/SpMV/lstsq + row/col gather/slice | CSC, sparse eig, incomplete factorizations |
| **Stats** | `scipy.stats` / pandas describe | 🟡 describe + t/χ² + ANOVA/Mann–Whitney + normPpf | Fler fördelningar, GLM, robust stats |
| **Tabell / I/O** | pandas, CSV/Parquet | 🟡 CSV + **KPQT1 Parquet-lite** + KND | **SC7c** `science/io`; typed columns, join/groupby, Apache Parquet FFI |
| **Visualisering** | matplotlib / seaborn | 🟡 ASCII + canvas plots | **SC7b** `science/visualize`; heatmaps/imshow/notebook rich |
| **GPU ndarray** | CuPy / torch.cuda | 🟡 `gpu.kab` + WGSL subset | **SC7a** `science/nd_gpu` (nd/Tensor-parity) |
| **Parallellism** | joblib / Dask (lokal) | 🟡 `job_map_*` + `dist` | **SC7d** `science/parallel` (lokal); `dist` = multi-node stub |
| **Klassisk ML** | scikit-learn | 🟡 dense/SGD/linreg + activations | Metrics, train/test split, PCA, k-means, trees/logreg, pipeline |
| **Deep learning** | PyTorch / JAX / TF | 🟡 dense + autograd-lite + GPU staging | Adam, Conv2d, attention-lite, DataLoader, checkpoint, riktig GPU |
| **LLM / modern AI** | transformers, tokenizers, CUDA | ❌ (CodAI i IDE separat) | Tokenizer + embedding + transformer-block inference (sedan train) |
| **Exploration** | IPython / Jupyter / Colab | 🟡 REPL + `.knb` + WASM (DX) | Canvas plots i cell, rich display, remote session |
| **Ship / prod** | FastAPI + Docker + CUDA image | 🟡 samma runtime som UI/OS/WASM | Modell→HTTP/kOS utan omskrivning (redan strategisk fördel) |

**Hur Kab vinner (även innan 100 % API-paritet):**

| Dimension | Kabootar-mål | Mot Python / NumPy / SciPy / PyTorch |
|-----------|--------------|--------------------------------------|
| Hastighet | Kontiguös data + VM/AOT/SIMD/GPU — **snabbare än Python** i hot path | Slå pure-Python; matcha/överträffa NumPy för vanliga storlekar |
| Iteration | `.kab` → `.kbc` + hot reload; samma binär som UI/OS | Snabbare än notebook↔script↔deploy-split |
| Stack | Ett språk: data + ML + spel + browser + OS | Mindre “Python + C-ext + Jupyter + Flask + CUDA-driver-helvete” |
| Typer / säkerhet | Bytecode + valfri `@manual` | Färre runtime-överraskningar i hot path |
| Distribuering | En runtime / WASM / kOS | Inget conda-env per projekt |
| Självständighet | Allt i Kab — fri från CPython-ekosystemet | Forskare äger stacken; ingen pip-supply-chain |
| AI-DX | `science/ml` + CodAI/DocAI + notebook | Utforska och shippa i samma session |

#### SC0 — Ndarray-kärna (NumPy-klass)

| Fas | Innehåll | Status |
|-----|----------|--------|
| **SC0a** | **`nd_*` contiguous array** — shape, flat data, zeros/ones/arange/reshape/get/set | ✅ subset |
| **SC0b** | **Elementvis + reductions** — add/mul/scale, sum/mean/max, broadcast subset | ✅ subset (add/mul/scale/sum/mean; broadcast → **SC0e**) |
| **SC0c** | **Float64/32 bulk** — zero-copy mot `Float64Array` / `@manual` buffers | ✅ subset (`nd_from_f64` / `nd_to_f64`) |
| **SC0d** | **Kab-API** — `import "science/nd"` wrappers ovanpå natives | ✅ subset |
| **SC0e** | **Broadcast + ufunc** — NumPy-style broadcasting, `where`/`clip`/`abs`/`exp`/`log` | ✅ subset (`nd_add`/`nd_mul`/`nd_sub`/`nd_div` broadcast; `nd_broadcast_to`/`nd_broadcast_shapes`; `nd_where`/`nd_clip`/`nd_abs`/`nd_exp`/`nd_log`/`nd_sqrt`) |
| **SC0f** | **Slice / view / stack** — ranges, `concat`/`stack`/`split` (copy-slice) | ✅ subset (`NdShared` Rc views + `a[1:10, :]` + `nd_slice` zero-copy; `concat`/`stack`/`split`) |
| **SC0j** | **Tensor ownership + lazy graphs** — unique buffer `take`; GC lazy realize | ✅ subset (`nd_take` / `science/tensor` / `science/lazy`) |
| **SC0g** | **Dtypes** — f32/f64/i32/i64/bool/complex64; cast | ✅ subset (`nd_dtype`/`nd_astype`; c64 interleaved + KND tag 6) |
| **SC0h** | **Random** — seed, uniform/normal, shuffle (Kab-API) | ✅ subset (`nd_seed`/`nd_rand_uniform`/`nd_rand_normal`; shuffle via `ml_shuffle`) |
| **SC0i** | **I/O** — `nd_save` / `nd_load` (binär/VFS; npy-inspirerat) | ✅ subset (KND1 binary) |

#### SC1 — Linear algebra & numerik (SciPy-klass)

| Fas | Innehåll | Status |
|-----|----------|--------|
| **SC1a** | **matmul / tensordot subset** på ndarray (utöver `mat_mul`) | ✅ subset (`nd_matmul` / `nd_dot`) |
| **SC1b** | **Solve / LU** — `nd_solve` / `mat_solve` för Ax=b | ✅ subset (`nd_solve` Gauss+partial pivot) |
| **SC1c** | **Decomps subset** — QR/SVD/eig (start: 2×2; utöka) | ✅ subset (`mat_svd2` + `mat_eigen2`; allmän → **SC1e**) |
| **SC1d** | **FFT / signal subset** — 1D FFT + conv | ✅ subset (`num_fft` / `num_ifft` / `num_conv1d`) |
| **SC1e** | **Full linalg** — QR, tunn/full/econ SVD, eig/sym, Cholesky, lstsq, `cond`, rSVD | ✅ subset (+ `mat_batch_*`, `mat_randomized_svd` / `randomizedSvd`) |
| **SC1f** | **Optimize** — `minimize` (gradient/Nelder), `least_squares`, `root` | ✅ subset (`num_minimize` Nelder–Mead, `num_least_squares`, `num_root`) |
| **SC1g** | **Integrate / ODE** — quad + `odeint`/`rk4` för system | ✅ subset (`num_rk4`/`num_odeint`/`num_odeint_adaptive`/`num_quad`) |
| **SC1h** | **Interpolate / special** — spline1d; `erf`/`gamma`/`bessel` subset | ✅ subset (`num_interp_spline*`/`num_erf`/`num_gamma`/`num_bessel_j0`) |
| **SC1i** | **Signal++** — 2D FFT, window, FIR/IIR, STFT/spectrogram, wavelets | ✅ subset (+ polyphase banks, Haar DWT/WPT multilevel) |
| **SC1j** | **Sparse** — CSR/COO/CSC, SpMV, incomplete factors, sparse direct solve | ✅ subset (+ `ilu0`/`ilut`/`icc0`/`icK`, `spsolve`/`iluSolve`/`iccSolve`) |

#### SC2 — ML / AI (ersätt sklearn + PyTorch-subset)

| Fas | Innehåll | Status |
|-----|----------|--------|
| **SC2a** | **Aktiveringar + loss** — relu/sigmoid/softmax, mse/cross-entropy | ✅ subset (relu/sigmoid/softmax/mse; **CE kvar**) |
| **SC2b** | **Dense forward + SGD** — `ml_dense`, `ml_sgd_update` | ✅ subset (+ `ml_linreg_step`) |
| **SC2c** | **Autograd-lite** — tape för dense/relu/mse | ✅ subset (`ag_*` + `science/autograd`; bredda → SC2f) |
| **SC2d** | **Dataset / batch** — shuffle, mini-batch, train/test split (**i Kab**) | ✅ subset (`ml_shuffle`/`ml_batch_slices`/`ml_train_test_split`) |
| **SC2e** | **Model I/O** — spara/ladda vikter (JSON/VFS/checkpoint) | ✅ subset (`ml_save_checkpoint`/`ml_load_checkpoint`) |
| **SC2f** | **Autograd++** — tape: matmul, conv, softmax, CE; `no_grad`; högre ordning senare | ✅ subset (`ag_matmul`/`ag_conv2d`/`ag_sigmoid`/`ag_softmax`/`ag_ce`/`ag_add`/`ag_mul`/`ag_no_grad`) |
| **SC2g** | **Autograd arithmetic** — sub/div/sum/exp; backward from sum/generic root | ✅ subset (`ag_sub`/`ag_div`/`ag_sum`/`ag_exp`) |
| **SC2n** | **Higher-order autograd** — `create_graph` + `grad_tensor` (sum/exp/mul/add/matmul/conv/softmax) | ✅ subset (+ attention HOAD via `scaledDotAttn` / SoftmaxGrad) |
| **SC2g** | **Optimizers + metrics** — Adam/AdamW; accuracy/F1/ROC-AUC/confusion | ✅ subset (`ml_adam_update`/`ml_adamw_update`/`ml_accuracy`/`ml_f1`/`ml_roc_auc`/`ml_confusion`) |
| **SC2h** | **Klassisk ML** — PCA, k-means, logreg, decision stump/tree subset, pipeline | ✅ subset (`ml_pca`/`ml_kmeans`/`ml_logreg_*`/`ml_stump_*`/`ml_tree_*` + `science/pipeline`) |
| **SC2i** | **NN-lager** — Conv2d, MaxPool, Embedding, MultiheadAttention-lite | ✅ subset (`ml_conv2d`/`ml_maxpool2d`/`ml_embedding`/`ml_mha`) |
| **SC2j** | **Training DX** — `fit`-loop, early stop, schedulers, progress i REPL/notebook | ✅ subset (`science/fit` + `ml_train_log` + rich progress) |
| **SC2k** | **Tokenizer + transformer inference** — BPE/WordPiece subset + forward (train senare) | ✅ subset (`tok_*`/`tf_transformer_forward`/`tf_lm_sgd_step` + `tf_lm_backprop_step` multi-layer) |
| **SC2l** | **AI delete-gate** — tränings-/inference-smoke **utan** Python/PyTorch i CI | ✅ subset (`science_freedom_demo.kab` + `science_sc_wave6`) |

#### SC3 — Data, viz, notebooks-DX

| Fas | Innehåll | Status |
|-----|----------|--------|
| **SC3a** | **CSV/JSON tabular** — `science/data` load/describe | ✅ subset (`csv_parse`/`csv_load`/`table_describe`) |
| **SC3b** | **Plot subset** — ASCII + pretty + **canvas2d** line/scatter/hist | ✅ subset (`ascii_plot`/`plot_line`/`plot_scatter`/`plot_hist`) |
| **SC3c** | **`kabootar mod init science-ai`** — mall + examples | ✅ subset |
| **SC3d** | **Docs & benches** — SCIENCE.md + CI vs Python-baslinjer | ✅ subset; 📋 utökade benches |
| **SC3e** | **Exploration DX** — REPL/notebook (se **Våg DX**) | ✅ DX0–DX5 subset |
| **SC3f** | **DataFrame-lite** — kolumntyper, select/filter, groupby/agg, join | ✅ subset (`df_from`/`df_select`/`df_filter`/`df_groupby`/`df_join`/`df_head`) |
| **SC3g** | **Stats++** — fördelningar, t-test/χ², corr/cov, quantiles | ✅ subset (`stat_quantile`/`stat_ttest`/`stat_chi2`/`stat_norm_*`/`stat_corr`) |
| **SC3h** | **Notebook rich display** — plot/table inline i `.knb` / WASM | ✅ subset (`rich_display` + `session_eval_rich` + notebook HTML) |

#### SC4 — Scale & hardware

| Fas | Innehåll | Status |
|-----|----------|--------|
| **SC4a** | **SIMD / BLAS-FFI** — matmul hotpath (kopplat P5) | ✅ (`sci_v*` chunked; `matrixmultiply` + **runtime OpenBLAS/MKL** via `cblas_dgemm`; `sci_blas_backend`) |
| **SC4b** | **GPU tensors** — wgpu compute för matmul/conv (kopplat GP0) | ✅ subset (`gpu_compute` WGSL matmul+conv2d f32 + CPU fallback; `gpu_zeros`/`ones`/`scale`/`add`) |
| **SC4c** | **Workers** — parallell map över batch (kopplat P8) | ✅ subset (`job_map_parallel` OS-threads för f64-ops; `job_map_chunks` Kab-closure chunk plan) |
| **SC4d** | **Delete-gate** — ML-smoke utan Python/NumPy i CI | ✅ subset |
| **SC4e** | **GPU train/infer path** — matmul/conv på device; host sync explicit | ✅ subset (`gpu_to_device`/`gpu_to_host`/`gpu_linear`/`gpu_conv2d`/`gpu_conv2d_kernel`; WGSL när `--features gpu`) |
| **SC4f** | **Bench harness** — Kab vs NumPy/PyTorch timing i CI (dokumenterad, inte blocker) | ✅ subset (`sci_bench`/`sci_bench_report`; Python-baslinje dokumenterad) |

#### SC5 — Science self-host (Kab-only, fri från Rust)

Målet: science-modulen blir **självständig** — skriven och underhållen i Kabootar.

| Fas | Innehåll | Status |
|-----|----------|--------|
| **SC5a** | **Kab-algoritmer** — linalg/optimize/stats/ML-träningsloopar i `lib/science/*.kab` (anropa bara tunna primitives) | ✅ subset (`fit.kab`, tokenizer/transformer wrappers) |
| **SC5b** | **Port natives → Kab** — `nd_*`/`ml_*`/`num_*` logik som kan vara ren Kab flyttas; Rust krymper till buffer/SIMD/GPU | ✅ subset (`kab_algo`: mean/std/softmax/linreg/sigmoid/cov/corr/trapz/argmax/…; vidare port kvar) |
| **SC5c** | **Inga nya Rust-science-API** — CI/policy: nya exports bara via `lib/science` | ✅ subset (policy i SCIENCE.md; wrappers obligatoriska för produkt-API) |
| **SC5d** | **Hotpath-kontrakt** — dokumenterad lista: vilka ops får native (matmul, FFT, GPU); resten Kab | ✅ subset (SCIENCE.md hotpath-tabell) |
| **SC5e** | **Science bootstrap** — `import "science"` fungerar i kab-only/seed-läge utan att växa Rust-ytan | ✅ subset (`import "science/bootstrap"`) |
| **SC5f** | **Frihets-gate** — research+AI demo (data→train→plot→HTTP) 100 % Kab-toolchain; dokumentera “no Python required” | ✅ subset (`examples/science_freedom_demo.kab`) |

#### SC6 — Science production modules (`lib/science/*`) 📋

Kab-first utökning mot sklearn/statsmodels/PyG/rl-stack/viz — **produkt-API i `.kab`**. Rust bara för hotpath (SIMD/GPU/FFT/graph kernels). Status: planerad — **implementeras efter ROADMAP-landning** (därefter “gör alla”).

| Fas | Modul (mål) | Innehåll | Status |
|-----|-------------|----------|--------|
| **SC6a** | `science/stats`++ / `science/prob` | **Statistik & sannolikhet** — fler fördelningar (expon/poisson/beta/binom), CDF/PDF/PPF, Monte Carlo, bootstrap CI, Bayes-lite | ✅ subset (`prob`: poisson/binom/expon/norm/beta + bootstrap/MC + `bayesBetaUpdate`/`bayesOdds`) |
| **SC6b** | `science/preprocess` | **Förbearbetning & feature engineering** — scale/standardize, impute, one-hot/ordinal, polynomial/interaction, train/test leakage-safe pipeline hooks | ✅ subset (`standardScale`/`minmaxScale`/`imputeMean`/`oneHot`/`polyFeatures`) |
| **SC6c** | `science/metrics` | **Utvärdering & metriker** — precision/recall/Fβ, PR-AUC, log-loss, R²/MAE/MAPE, calibration; ovanpå `ml_accuracy`/`f1`/`roc_auc` | ✅ subset (`precision`/`recall`/`fbeta`/`r2`/`mae`/`mape`/`logLoss`/`prAuc`) |
| **SC6d** | `science/graph` | **Grafer & GNN** — adjacency/CSR graph, message-passing-lite (GCN/GraphSAGE-subset), node embed; sparse SpMV reuse | ✅ subset (`fromEdges`/`meanAggregate`/`gcnLayer`/`degreeFeatures`) |
| **SC6e** | `science/timeseries` | **Tidsserieanalys** — lag/rolling, ACF/PACF-lite, AR/ARIMA-subset, seasonal decompose, forecast metrics | ✅ subset (`acf`/`ar1*`/`arima110*`/`arima111*`/`seasonalDecompose`) |
| **SC6f** | `science/rl` | **Förstärkningsinlärning** — env-API (reset/step), replay buffer, Q-learning / REINFORCE-lite, gym-style smoke | ✅ subset (`createEnv`/`step`/`replay*`/`qLearnUpdate`/`greedyAction`) |
| **SC6g** | `science/viz` / `explain` | **Visualisering & tolkningsbarhet** — confusion heatmaps, learning curves, feature importance / permutation, SHAP-lite; canvas/notebook rich | ✅ subset (`explain`: heat/curves/perm/corr + `shapLinear`/`shapKernelLite`) |
| **SC6h** | `science/dist` | **Distribuerad / parallell beräkning** — chunked `map`/`reduce` över workers (SC4c/P8), threaded AllReduce (sum/mean/max); multi-node senare | ✅ subset (`chunk`/`parallelMapF64`/`allReduce*`/`sci_allreduce_f64`/`mapReduce`) |
| **SC6i** | `science/domain/*` | **Domänspecifika moduler** — t.ex. `bio` (sekvens-lite), `finance` (returns/vol), `chem` (molekyl-featurize-lite), `nlp` (ovanpå tok/tf); tunna Kab-paket | ✅ subset (`domain/finance`/`bio`/`nlp`/`chem`) |

**SC6-policy:** samma SC5c — nya exports via `lib/science/*.kab` (+ `science/domain/…`); natives bara om hotpath kräver det. Smokes: `tests/science_sc_wave11.rs`.

#### SC7 — Science surface modules (tydlig produkt-yta) ✅ subset

Kab-first **omorganisation / fördjupning** av det forskare importerar dagligen. Befintliga `gpu.kab`, `explain.kab`, `dist.kab`, `data.kab`/`nd` I/O **förblir** (alias / lägre lager); SC7 ger **rena produktnamn** och nd/Tensor-parity.

| Fas | Modul (mål) | Innehåll | Status |
|-----|-------------|----------|--------|
| **SC7a** | `science/nd_gpu` | **Ndarray ↔ GPU** — `toDevice`/`toHost`/`toNd`; `matmul`/`add`/`relu`/`matmulKernel`/`conv2dKernel` | ✅ subset (+ deepen) |
| **SC7b** | `science/visualize` | **Visualisering** — line/scatter/hist/heatmap/`plotNd`/`imshow`/`imshowNd` | ✅ subset (+ deepen) |
| **SC7c** | `science/io` | **Science I/O** — KND, CSV, checkpoint, JSON, JSONL, **Parquet-lite (KPQT1)** | ✅ subset (+ deepen) |
| **SC7d** | `science/parallel` | **Lokal parallellism** — `mapItems`/`mapParallel`/`mapNdParallel`/`vmap`/`mapReduce` | ✅ subset (+ deepen) |

**SC7-policy / lager:**

| Lager | Modul | Ansvar |
|-------|--------|--------|
| Produkt (importera först) | `nd_gpu`, `visualize`, `io`, `parallel` | Stabil Kab-API, nd/Tensor-shapes, docs/examples |
| Befintligt / smalare | `gpu`, `explain`, `dist`, `data` | Låga wrappers, interpretability, distribuerad stub, CSV/plot primitives |
| Hotpath | Rust natives | SIMD/GPU/FFT/workers — inga nya produktnamn i Rust (SC5c) |

**SC7-ordning:** **SC7c** (`io`) → **SC7d** (`parallel`) → **SC7b** (`visualize`) → **SC7a** (`nd_gpu`).

**Bootstrap:** `science/bootstrap` importerar `io` / `parallel` / `visualize`. `nd_gpu` är **explicit** `import "science/nd_gpu"` (annars skuggas `nd.from`/`matmul`/`zeros`).

**SC-ordning:** SC0–SC7 ✅ subset.

**Checkpoint SC (landad 2026-08-06):** MKL/OpenBLAS thread control (`sci_blas_set_num_threads` / `blasInfo`); multi-layer TF stack (`tf_stack_forward` / `tf_stack_backprop_step`); nested Parquet List/Struct roundtrip (`tests/science_sc_checkpoint_next.rs`).
**Checkpoint SC (landad 2026-08-06b):** fancy indexing (`nd_gather` / `nd_compress`); `complex64` dtype + KND tag 6; threaded in-process AllReduce (`sci_allreduce_f64` / `allReduce*`) — `tests/science_sc_checkpoint_parity.rs`.
**Checkpoint SC (landad 2026-08-06c):** complex gather/compress + `nd_nonzero` / `nd_fancy_index`; SC5b Kab-port (`median`/`percentile`/`crossEntropy`/`oneHot`/`matmul`/`f1`/`confusion`); multi-rank AllReduce (`allReduceRanks`) — `tests/science_sc_checkpoint_sc5.rs`.
**Checkpoint SC (landad 2026-08-06d):** outer multi-axis fancy (`fancyOuter`); mailbox multi-node AllReduce (`allReduceStar`/`allReduceRing`); Kab `pcaKab`/`stumpKab`/`kmeansKab` — `tests/science_sc_checkpoint_sc5d.rs`.
**Checkpoint SC (landad 2026-08-06e):** TCP socket AllReduce (`sci_allreduce_tcp` / `allReduceTcp`); Kab bagging/boost/tree ensembles; sparse row gather/compress + dense-mask COO view — `tests/science_sc_checkpoint_sc5e.rs`.
**Checkpoint SC (landad 2026-08-06f):** `broadcastTo`/`nd_broadcast_shapes`; autograd sub/div/sum/exp; QR/SVD thin·full + `qrErr`/`pinv`; `rfft`/`irfft`/`fftC`/`fftPad` — `tests/science_sc_checkpoint_deepen.rs`.
**Checkpoint SC (landad 2026-08-06g):** multi-host TCP AllReduce (`allReduceTcpRank` / bindHost); `gbdtFitKab`/`gbdtPredictKab`; sparse `gatherCols`/`compressCols`/`slice`; HOAD `backward(..., true)`/`gradTensor` — `tests/science_sc_checkpoint_sc5f.rs`.
**Checkpoint SC (landad 2026-08-06h):** pillar deepen — ndarray max/transpose/tensordot; LU/slogdet/normOrd; erfc/j1/gradient; fftfreq/resample/hilbert; ANOVA/MW/ppf; logistic GBDT/dropout/BN/dataloader; mechanics/chem — `tests/science_sc_checkpoint_pillars.rs`.
**Checkpoint SC (landad 2026-08-06i):** `einsum` subset; `batchQr`/`batchSvd`/`batchSolve`; `fftN`/`firwin`/`butterBiquad`; sparse CSC; dense/conv `create_graph`; `domain/pde` heat/wave/poisson — `tests/science_sc_checkpoint_sc6.rs`.
**Checkpoint SC (landad 2026-08-06j):** richer einsum; `batchEig`; polyphase resample/decompose; `ilu0`/`icc0`; matmul `create_graph`; 2D PDE heat/poisson — `tests/science_sc_checkpoint_sc6b.rs`.
**Checkpoint SC (landad 2026-08-06k):** general einsum parser; `batchSvd` econ; polyphase analyze/synthesize + Haar DWT; `ilut`/`icK`; conv HOAD graph; 3D PDE + FEM lite — `tests/science_sc_checkpoint_sc6c.rs`.
**Checkpoint SC (landad 2026-08-06l):** einsum ellipsis/broadcast; `randomizedSvd`; multilevel Haar + WPT; `spsolve`/`iluSolve`/`iccSolve`; softmax/attention HOAD; FEM 2D triangles — `tests/science_sc_checkpoint_sc6d.rs`.
**Checkpoint SC (landad 2026-08-06m):** `einsumPath`; `streamingSvd`; DTCWT; `rcm`/`lu`/`chol`; `mhaHoAd`; FEM Neumann/Robin — `tests/science_sc_checkpoint_sc6e.rs`.
**Checkpoint SC (nästa):** einsum path executor; randomized range finder SVD; dual-tree Q-shift filters; sparse AMD + supernodal LU; flash-attn HOAD; FEM mixed BC + time-dependent.
**Checkpoint SC (landad 2026-08):** full MHA QKV/softmax BP; system BLAS-FFI (OpenBLAS/MKL); KPQT1 Parquet-lite; GP7 GPU viewport.  
**Checkpoint SC (landad tidigare):** SC7 deepen + REINFORCE + exact `shapKernel` + attn `wo` BP + DX7 `lib/dx/session` + GP7 prefab; BLAS-API + TF multi-layer BP + `job_map_chunks` + SC6 + tensor/lazy ownership.  
**Checkpoint SC (research-parity):** spline/special/sparse + trees (landad subset).  
**Checkpoint SC (AI-parity):** tokenizer/transformer + SC2l + GPU kernels (landad subset).  
**Slutmått:** forskare och AI-utvecklare använder Kabootar **istället för Python** — från notebook till ship — med numerisk hotpath i NumPy/PyTorch-klass, **SC6–SC7 modules**, och **helt självständig** stack (SC5f).

### Våg DX — Exploration (REPL / notebook vs Python) 🚧

**Problem:** Python vinner *utforskning* (REPL, Jupyter, snabb “prova → se”). Kabootars gamla REPL var enradig och Debug-print — inte konkurrenskraftig.

**Mål:** Kabootar ska kännas **bättre än Python för exploration-to-ship**: samma session kan importera `science`, rita canvas, hosta HTTP och gå till `kabootar run` / kOS **utan** notebook→script→venv-omstart.

**Hur Kab vinner mot Python (utforskning):**

| Dimension | Kabootar-mål | Mot IPython / Jupyter |
|-----------|--------------|------------------------|
| Session | Persistent env + `_` + multiline + `:load` | Paritet med IPython-basics |
| Notebook | `.knb` celler + HTML/WASM runner | Mindre toolchain än Jupyter+kernel |
| Stack | En runtime: STEM + UI + OS i samma cell | Ingen “kernel vs app”-split |
| Deploy | Cell → `.kab` / mod utan omskrivning | Snabbare än “kopiera ur notebook” |
| AI-DX | DocAI/CodAI i samma IDE-session | Integrerat, inte separat Colab |

| Fas | Innehåll | Status |
|-----|----------|--------|
| **DX0** | **Modern REPL** — multiline, pretty-print, `_`, `:help`/`:load`/`:reset`/`:vars` | ✅ subset |
| **DX1** | **Session API** — `Session::eval_cell` delad av CLI + notebook + tester | ✅ subset (`src/session.rs`) |
| **DX2** | **`.knb` notebook** — JSON-celler, `kabootar notebook run` | ✅ subset |
| **DX3** | **HTML/WASM notebook** — web UI som kör celler (samma eval) | ✅ subset (`kabootar-notebook.html` + `session_eval`/`session_science`) |
| **DX4** | **Science-REPL presets** — `:science`, plot/table pretty | ✅ subset (`:science`, `pretty`/`format_table`/`ascii_plot`) |
| **DX5** | **History / readline** — line-edit + historikfil | ✅ subset (`rustyline` + `~/.kabootar_history`) |
| **DX6** | **Rich display** — plot/table inline i notebook/WASM (kopplat SC3h); Kab-UI inte Rust | ✅ subset (`session_eval_rich` + `kabootar-notebook.html`) |
| **DX7** | **DX self-host** — REPL/session-hjälpare i `.kab` där det går; thin host kvar | ✅ subset (`lib/dx/session.kab` + REPL `:help` hooks) |

**DX-ordning:** DX0–DX5 → DX6 → DX7 (parallellt SC5).

**Checkpoint DX:** REPL + `.knb` smoke; docs [EXPLORATION.md](EXPLORATION.md). **Slutmått:** exploration-to-ship utan Python/Jupyter.

---

## Master fetch-plan (2026–2027) — historik / parity

Nedan är **hämtad parity** (JS/Deno/DOM/OS). Den är **underordnad** Våg L→S→K→H respektive **P/GP** (prestanda/spel). Inget nytt i D/G som utökar Rust-ytan utan språkbehov; GPU/FFI-hotpath för GP0 tillåts som i H6d.

### Våg A — JavaScript (hämtningsbar rest, ~10–15 %) ✅

**Status (2026-06):** A1–A12 implementerade med parity-tester i `tests/kabootar_js_parity.rs` (`js_wave_a*`).

**Mål:** 100 % av det Kabootar avser att hämta från ECMAScript + kärn-stdlib (exkl. medvetet borttaget: `var`, `eval`, prototyper, implicit coercion).

| Fas | Innehåll | Uppskattad tid |
|-----|----------|----------------|
| **A1** ✅ | `btoa`/`atob`, `performance.now`, `crypto.getRandomValues` (web subset) | 1 vecka |
| **A2** ✅ | **BigInt** — literal `123n`, aritmetik, `BigInt()` | 2–3 veckor |
| **A3** ✅ | **Privata klassfält** `#x`, privata metoder | 2 veckor |
| **A4** ✅ | **RegExp** — unicode (`u`), lookbehind, `flags`, `dotAll` | 2 veckor |
| **A5** ✅ | **Date** full objekt-API (`getUTC*`, `toISOString` polish, timezone basics) | 1–2 veckor |
| **A6** ✅ | **Typed arrays** — Float64Array, DataView, ArrayBuffer constructor | 2 veckor |
| **A7** ✅ | **Proxy/Reflect** — alla traps, `Reflect.construct`, invariants | 2 veckor |
| **A8** ✅ | **WeakMap** / **WeakSet** | 1 vecka |
| **A9** ✅ | **`using`** (explicit resource management), `import.meta`, dynamisk `import()` | 2–3 veckor |
| **A10** ✅ | **Intl** (minimal: `Intl.NumberFormat`, `Intl.DateTimeFormat`) | 3–4 veckor |
| **A11** ✅ | **Temporal** (optional polyfill-nivå) | 2–3 veckor |
| **A12** ✅ | Spec-polish: iterator/bytecode `try_regions` i `.kbc`, generator kantfall, Error `.cause`/stack | 2 veckor |

**Våg A totalt:** ~3–4 månader

### Våg B — Deno (runtime-paritet, ~5–10 %) ✅ yta / 🚧 fördjupning

**Status (2026-06):** B1–B8 API + tester i `tests/kabootar_deno_parity.rs`. Fördjupning pågår: async serve-loop, lockfile-integrity.

Deno-listan i [DENO.md](DENO.md) är i stort sett ✅; kvar är **fördjupning och produktionsbeteende**:

| Fas | Innehåll |
|-----|----------|
| **B1** ✅ | `Deno.serve` async + HTTP/2 preface/SETTINGS + TLS ALPN `h2` — full HPACK/streams senare |
| **B2** ✅ | Full WHATWG Streams (backpressure, cancel, tee edge cases) — grund ✅ |
| **B3** ✅ | `Deno.permissions` / capability prompts |
| **B4** ✅ | `Deno.test` / `Deno.bench` inbyggt |
| **B5** ✅/🚧 | npm/JSR: fler paket, native addons policy, lockfile — **integrity från cache** ✅ |
| **B6** ✅ | `Deno.cwd`/`chdir` + `Deno.realPath`/`Deno.symlink`/`Deno.link` |
| **B7** ✅ | `Deno.listenTls` / ALPN (`h2`, `http/1.1`) / cert reload — TLS server ✅ |
| **B8** ✅ | Worker: SharedWorker, `postMessage` transferables full lista — in-process ✅ |

**Våg B totalt:** ~1–2 månader

### Våg C — DOM / kDOM / Kv8 (~60–70 % av browser-ytan) ✅

**Status (2026-07):** C1 — selectors + MutationObserver + Kv8 EventTarget (bubble/capture/`removeEventListener`/`Event`). **C2:** React 19 esbuild; late-var closure patch; CI `via_import` wire; createRoot preferens `bundle` → reconstruct från publicerad `mm` → shim (`__kv8CreateRootSource`); full createRoot+render (`#[ignore]`). **C3:** `evalSource` → Rust. **C4:** flex wrap/grow/shrink + grid. **C5:** Canvas curves/clip/toDataURL/imageData/setTransform + host parity. **C6:** WebGL FBO + `compileShaderFromFiles` + texture flat API. **C7:** WebRTC ICE + DTLS fingerprint/role + SRTP peer bridge. **C8:** SW fetch events + extension permissions. **C9:** DevTools network panel, profiler, live edit. **Dom live:** parent sync.

| Fas | Innehåll |
|-----|----------|
| **C1** ✅ | **kDOM** — MutationObserver + Kv8 Event bubble/capture/remove; selectors |
| **C2** ✅ | **Kv8** — React 19 esbuild; createRoot `bundle`/`mm`/`shim`; CI wire; full render `#[ignore]` |
| **C3** ✅ | **Kv8 hot path** — `evalSource` via Kab `.kab` (`evalSourceWith`); H6a: Rust `evalSourceRust` bort |
| **C4** ✅ | **Layout** — flex (`justify`/`align`/`wrap`/`grow`/`shrink`) + simple CSS grid |
| **C5** ✅ | **Canvas 2D** — curves/clip/toDataURL/imageData/setTransform (+ [CANVAS.md](CANVAS.md)) |
| **C6** ✅ | **WebGL** — textures, FBO, shaders från GLSL-filer (`fixtures/webgl`) |
| **C7** ✅ | **WebRTC** — ICE + DTLS fingerprint/role + SRTP media bridge |
| **C8** ✅ | **PWA/Extensions** — SW fetch events (`pwa_dispatch_fetch`) + extension permissions |
| **C9** ✅ | **DevTools** — network panel, profiler, live edit |

**Våg C totalt:** ~4–6 månader

### Våg D — OS (stub → native, ~40–50 %) 🚧 host / ⏸ D6+

D1–D5 finns som **tillfällig Rust-host**. Vidare OS-logik → **Våg K2** i Kabootar efter L+S. **D6–D9 pausade** som Rust-arbete.

| Fas | Innehåll |
|-----|----------|
| **D1** ✅ | Ring 0: CFS enqueue/yield + IRQ timer preempt (`os_irq_raise` / `os_sched_preempt`) |
| **D2** ✅ | MMU: `os_mm_fault`, `os_mm_mmap`, COW (`os_mm_cow_share` / `os_mm_cow_break`) |
| **D3** ✅ | FS: journal payload + `os_journal_replay`/`checkpoint`; path-ACL (`os_acl_*`) + `os_perm_*` |
| **D4** ✅ | Netstack: `lo`/`host-eth` NIC refresh, `os_netstack_info`, tx accounting (`hw` → host-ifaces) |
| **D5** ✅ | GPU compositor subset: `os_display_monitors`, `os_display_vsync`, acrylic layer preview |
| **D6** ⏸ | `os_compat_run` — senare som Kabootar+thin host, inte ny Rust-monolit |
| **D7** ⏸ | Boot: BIOS/UEFI / bare-metal — efter thin host (H) |
| **D8** ⏸ | Sauce-strategier — hardware-bindningar only i Rust |
| **D9** ⏸ | **kOS desktop shell** — [lib/kos/ROADMAP.md](../lib/kos/ROADMAP.md) |

**Våg D totalt:** host-subset klart; resten via K/H

### Total kalender (en utvecklare, heltid) — omställd

```
Våg L (språk)  ████████░░░░░░░░  nu — L1 först
Våg S (host)   ░░░░████░░░░░░░░  efter L1–L3
Våg K (libs)   ░░░░░░██████████  kv8/kos/kbrowser i .kab
Våg H (thin)   ░░░░░░░░░░██████  produktlogik bort; JIT/GC/GPU native (P-tak)
Våg P (perf)   ████████░░░░░░░░  P0–P10 subset; **P11–P18 tak kvar** (unbox/JIT/nursery/same-room)
Våg GP (spel)  ██████████████░░  GP0–GP5 ✅; GP7a–g + GP6a/b/c/e/f/g/i; terrain/audio/XR kvar
Våg SC (STEM)  ████████████████  SC0–SC7 ✅; system BLAS-FFI + full MHA BP + KPQT1
Våg DX (explore)████████████████  REPL + .knb + WASM + readline + rich display; DX7 kvar
Våg A–G        (parity-historik — underordnad L/S/K)
```

**Checkpoint efter varje våg:** språk-/self_host-tester först; `cargo test` full suite + [FEATURES.md](FEATURES.md). Spel: [GAME.md](GAME.md) + GP-budgets.

### Våg E — Self-host bootstrap + generics ✅

| Fas | Innehåll |
|-----|----------|
| **E1** ✅ | `emit.kab` full compile_and_run (M10) |
| **E2** ✅ | `serialize.kab` full compile_and_run (M11) |
| **E3** ✅ | True bootstrap `compile(compile.kab)` (M12) |
| **E4** ✅ | Native generics — Rust lexer/parser/bytecode, monomorphisering v1 ([GENERICS.md](GENERICS.md), `tests/generics.rs`) |
| **E5** ✅ | Self-host generics subset i `parser.kab` / `emit.kab` / `lexer.kab` (`test_parser.kab`, `test_emit.kab`) |

### Våg F — Generics fas 2 (G6–G11) ✅

Bygger på **Våg E** (fn-generics v1). Se [GENERICS.md](GENERICS.md#fas-2--g6-planering). **Inte** samma som [Våg FT — Fart](#våg-ft--fart-alla-tekniker-i-kab).

| Fas | Innehåll | Beräknad ordning |
|-----|----------|------------------|
| **F1** ✅ | **G6** — Inferens v1.1: variabler med känd typ, klass-c ctor-namn, tester i `tests/generics.rs` |
| **F2** ✅ | **G7** — Generiska klassmetoder (`fn map<U>(f)` + `this`), Rust parse + monomorph (`echo$Number` på klass) |
| **F3** ✅ | **G8** — Generiska klasser (`class Box<T>`), `Box(42)` → `Box$Number`, infer + explicit type args |
| **F4** ✅ | **G9** — Generiska enum (`enum Option<T>`), `Option.Some(42)` → `Option$Number` |
| **F5** ✅ | **G10** — Self-host: G6 variabel-inferens i `emit.kab`, G9 enum-parse + member `typeArgs` |
| **F6** ✅ | **G11** — LSP: hover specialiserad signatur, go-to-def på `T`, completion |

**Icke-mål Våg F:** trait bounds, HKT, runtime `typeid`. (Struct = Våg R.)

**Checkpoint:** `cargo test --test generics` + `self_host_parser_suite` / `self_host_emit_suite` efter varje fas.

### Våg G — Standardbibliotek + traits + Kv8-ramverk 🚧

Kompletterar JS/DOM-paritet och gör Kabootar produktionsklart som språk.

| Fas | Innehåll | Status |
|-----|----------|--------|
| **G1** | **`lib/std/*`**, [STDLIB.md](STDLIB.md), `str_match`/`str_search`, hyperbolic Math | ✅ subset |
| **G2** | `matchAll`, `toLocaleString`, array member `push` på uttryck (bytecode) | ✅ (`ArrayPush`; `str_match_all` / `.matchAll`; `toLocaleString` på str/array) |
| **G3** | `import "std"` aggregator, Intl-localeCompare | ✅ (`lib/std.kab` + builtin; `localeCompare` / sensitivity base) |
| **G4** | Math rest (`f16round`, `sumPrecise`) | ✅ |
| **G5** | **Traits** — `trait Show { fn show() }` alias till interface + `implements` | ✅ subset (`trait` ≈ `interface`; `where`-bounds senare) |
| **G6** | **kss** (styles) + Next-lik filrouting (`pages/*.kab`) | ✅ (`import "kss"` toCss/apply; `import "pages"` renderRoute; `pages/_app`+`index`) |
| **G7** | **kbrowser mobil** — [lib/kbrowser/ROADMAP.md](../lib/kbrowser/ROADMAP.md) | ✅ subset |
| **G8** | **Compile-opt** — incremental self-host, [COMPILE.md](COMPILE.md) | ✅ subset (`.kbc` fingerprint + import mtimes) |
| **G9** | **Kv8 i Kabootar** — lexer/parser/eval Kv8-subset self-host | ✅ subset (`?.`/templates `${expr}`/ternary/`switch`/array/unary/`for*`/try/fn) |
| **G10** | **React/Next-lik** — Kv8 fiber + kDOM SSR (`import "kv8/react"`) | ✅ subset (`ntag`/`cnid*` multi nested + parent live sync/`onById`/`dispatchById`) |
| **G10b** | **Runtime MemBox** — opt-in `@manual` + `owned_*` / `import "kos/mem"` (GC default orörd). Compile-time = **Våg O** | ✅ runtime; O1–O3 ✅ |
| **G11** | **kbrowser cross-platform** — [lib/kbrowser/ROADMAP.md](../lib/kbrowser/ROADMAP.md) | ✅ subset |
| **G12** | **kOS skrivbord** — [lib/kos/ROADMAP.md](../lib/kos/ROADMAP.md) | ✅ subset |

Produkt (docs + plan, inte duplicerade här): **kOS** = [README](../lib/kos/README.md) / [plan](../lib/kos/ROADMAP.md); **kbrowser** = [README](../lib/kbrowser/README.md) / [plan](../lib/kbrowser/ROADMAP.md). Bygg **kOS först**.

**Tester:** `cargo test stdlib_wave`, `cargo test --test kabootar_js_parity`, [VSCODE_TESTS.md](VSCODE_TESTS.md).

---

## Bidra

Varje fas bygger på föregående. Se `src/` och öppna issues för diskussion om API-design.
