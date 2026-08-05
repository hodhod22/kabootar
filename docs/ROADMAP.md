# Kabootar — roadmap

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

**Slutmål:** hela plattformen i Kabootar — Kv8, DOM, CSS/KSS, OS, webläsare, shell, **spelmotor**, **science/AI**. Rust försvinner **småningom** tills bara (möjligen) en minimal bootstrap/GPU-FFI finns kvar; ny produktlogik skrivs **aldrig** i Rust. Kabootar ska bli **helt självständig och fri** — forskning och AI utan Python/NumPy/SciPy/PyTorch som runtime-beroende.

**Produktambition (spel):** Kabootar ska bli **bättre än C# och C++ för spelproduktion** — inte genom att vinna rå pointer-aritmetik, utan genom att vinna **hela produktionskedjan**: ett språk från OS→UI→gameplay, snabbare iteration (hot reload / self-host), säkrare än C++ (O + GC-val), mer integrerat än C#/Unity (browser + kOS + 3D i samma runtime), och **GPU-först** så frame-budgeten matchar native motorer.

**Produktambition (forskning & AI):** Kabootar ska **ta över Pythons roll** för vetenskap, dataanalys och AI-utveckling. Språket är redan snabbare än Python i hot path; målet är att vinna **hela forskningskedjan** (utforskning → modell → deploy → UI/OS) utan `venv`/`pip`/`conda`, utan C-extension-helvete, och utan att byta språk när prototypen ska bli produkt. All ny STEM/AI-logik skrivs i **Kabootar** (`.kab`); Rust är tillfällig host/hotpath tills self-host + GPU/SIMD räcker — sedan bort.

**Minnesmodell:**
- **Borrowing / `@manual` / `owned_*`** — systemutveckling (OS, drivrutiner, buffertar, netstack, **spel-hotpath-buffers**). **Compile-time ownership = Våg O** (L5 var bara runtime MemBox).
- **GC (default)** — webutveckling (DOM, Kv8, appar, shell-UI, gameplay-script)

Ordning (strikt) — **just nu: endast språk**, sedan prestanda + spel parallellt med K/H:

0. **Komplettera Kabootar-språket** (L + O + **T** traits + **J** JS-stdlib + **R** struct) — optimera, paritet, ownership  
1. **Self-host som produktionskompilator** (S) — pausad tills J/T/R landat tillräckligt  
2. **Bygg om allt i Kabootar** (K): kv8, dom, css, os, kbrowser  
3. **Tunna bort Rust** (H) tills hosten är trivial  
4. **Prestanda + spelproduktion** (P + GP) — snabb VM/AOT, GPU-3D, asset-pipeline; se nedan  
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
| **H** | Host → noll | Rust krymper till bootstrap; därefter bort |
| **P** | Performance | VM/AOT/GC/SIMD — Kab snabb nog för gameplay + verktyg |
| **GP** | Game production | GPU-3D, scen/motor, assets, audio — redo för spelproduktion |
| **SC** | Science / AI | NumPy/SciPy/sklearn/PyTorch-klass i Kab — ta över forskning & AI; SC5 = Kab-only |
| **DX** | Exploration DX | REPL + notebook — slå Python för *utforskning* (samma runtime som ship) |

**Aktivt fokus (2026-08):** **SC** (gap → NumPy/SciPy/AI + **SC5 Kab-only**) + **DX** parallellt med **P**; språk **O/T/J/R** långsiktig bas. **P/GP** subset landad.

**Klass vs struct (2026-07):** `class` → **`this`**; `struct` → **`self`** / `&self` / `&mut self` (R1).

### Våg L — Language (systems-ready) 🚧

Blockerare från `self_host/README.md` och `lib/kv8/` — måste bort innan ekosystemet kan växa i Kabootar.

| Fas | Innehåll | Status |
|-----|----------|--------|
| **L1** | **Reentranta bytecode-lokaler** — `StoreLocal`/`MakeArrowFn` får inte `assign` upp i parent/modul-env; closures fångar aktiveringsram (`share_bindings`); seed av capture-slots vid fn-entry; `sync_closure_writes` synkar bara riktiga captures | ✅ |
| **L2** | **Modulskala** — `register_functions` + `BytecodeFunction::Clone` använder `share_bindings` (inte djupklon); ≥40 top-level fn/modul utan OOM | ✅ |
| **L3** | **Closures under rekursion** — fångade `let` överlever nästlade anrop av samma fn | ✅ (via L1) |
| **L4** | **Await i modul/fn** — microtask writeback av globals; capture-bitar (`local_captures`); Await synkar locals; `lib/os/async` använder riktig `await` | ✅ |
| **L5** | **Runtime MemBox** — `@manual` + `owned_*` / `os/mem` (move/drop vid runtime); GC default. **Inte** compile-time ownership/borrow-check | ✅ runtime |

**Checkpoint L1–L5:** `cargo test --test v228_language bytecode_` + `cargo test --test ownership_manual` + `cargo test --test s2_compile_cli`

### Våg O — Ownership (systems, compile-time) 🚧

GC förblir default. Ownership gäller **bara** `@manual`-moduler. Se [OWNERSHIP.md](OWNERSHIP.md).

| Fas | Innehåll | Status |
|-----|----------|--------|
| **O1** | **Affine Owned** — compile-time use-after-move; `let y = x` / call-arg flyttar Owned; kända peek-API (`owned_read`/`owned_write`) flyttar inte | ✅ |
| **O2** | **Signaturer** — `fn f(b: Owned)`, `fn g(b: &Owned)`; call-arg med Owned flyttar (om inte `&`/`&mut`) | ✅ |
| **O3** | **Borrow** — `&x` / `&mut x`, typer `&Owned` / `&mut Owned`; shared vs exclusive; borrow-scope = call-uttryck | ✅ |
| **O4** | **Scope drop** — compile-time varning/fel om Owned lever över scope utan `drop`/`move` (leak-lint); runtime drop oförändrad | ✅ |
| **O5** | **Self-host checker** — port O1–O3 till `self_host/` så produktkompilatorn checkar ownership | ✅ subset (+ `&`/`&mut` peek i `ownership.kab`) |

**Checkpoint O1–O3:** `cargo test --test ownership_check` + `cargo test --test ownership_manual`

**Icke-mål (medvetet):** Rust-lifetimes, lifetime-elision, borrow över async boundaries, ownership i GC-moduler.

### Våg T — Traits (språk) 🚧

G5 gav `trait` ≈ `interface`. Det räcker **inte** för systems-/generics-kod. Se [GENERICS.md#traits](GENERICS.md#traits).

| Fas | Innehåll | Status |
|-----|----------|--------|
| **T0** | `trait` / `interface` + `implements` + `is_impl` (G5) | ✅ subset |
| **T1** | **`where T: Trait`** på generiska fn/klass/metod — monomorphisering respekterar bound | ✅ |
| **T2** | **Generiska traits** — `trait Show<T> { … }` | ✅ |
| **T3** | **Associated types** — `trait Iter { type Item; }` (subset) | ✅ |
| **T4** | **Default-metoder** i trait-kropp | ✅ |
| **T5** | Self-host: trait/`where` i `self_host/parser` + `emit` | ✅ subset (+ `type Item;` → `associatedTypes`) |

**Icke-mål:** HKT, `dyn Trait`-objekt, Rust-coherence.

### Våg R — Struct (Rust-inspirerat) 📋

| Fas | Innehåll | Status |
|-----|----------|--------|
| **R0** | **`this` i `class`** — klassreceiver = `this`; `self` reserverat i lexern | ✅ |
| **R1** | **`struct Name { … }`** — värdetyper + metoder med **`self` / `&self` / `&mut self`** | ✅ |
| **R2** | Struct + `@manual` ownership (move) | ✅ |
| **R3** | Generiska structs `struct Box<T>` | ✅ |
| **R4** | Self-host: `struct`/`self` i parser+emit | ✅ subset |

**Regel:** `class` → `this`; `struct` → `self`. Se [CLASSES.md](CLASSES.md).

### Våg J — JS-språkparitet (stdlib + syntax) 🚧

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

### Våg S — Self-host som produktkompilator 🚧

| Fas | Innehåll | Status |
|-----|----------|--------|
| **S1** | Migrera bort workarounds som L1–L3 gör onödiga (fn-lokala stacks istället för `eNode`/`pLeft`-familjen där det går) | ✅ slice: `emit.kab` AST_BINARY + `parser.kab` call/index |
| **S2** | `kabootar compile` default via `self_host/compile.kab` för `.kab` → `.kbc` (`--rust` / `--self-host`; `self_host/` → Rust) | ✅ |
| **S3** | CI: self-host bygger self-host (bootstrap) som gate | ✅ |

### Våg K — Ekosystem i Kabootar 📋

| Fas | Innehåll |
|-----|----------|
| **K1** | **Kv8** — lexer/parser/eval/JIT-policy i `.kab` (ersätt Rust `kv8_*`) — ✅ **subset** (lexer+parser i `lib/kv8`; eval hybrid). Gate: `cargo test --test kv8_lib -- --test-threads=1` |
| **K1c** | **Kv8 Kabootar eval** — ✅ **subset**: `evalSourceKab` → `evalSourceWith` (literals/ops/control); class/async kvar via Rust `evalSource` |
| **K1d** | **Kv8 class/new/async** — ✅ **subset**: `K_NEW` + `this` + `K_CLASS`/`K_NEW`/`K_AWAIT` i `evalSourceKab` (`k1d_class_new_kab_eval`); async kör sync |
| **K1e** | **Kv8 extends + Kab evalSource** — ✅ **subset**: `extends` mergar parent-metoder; `evalSource` → Kab-path (`evalSourceWith`); Rust kvar som `evalSourceRust` (`k1e_extends_kab_eval`, `k1e_eval_source_prefers_kab`) |
| **K1f** | **Kv8 async/Promise** — ✅ **subset**: async fn returnerar `{__k8promise,value}`; `K_AWAIT` unwrap; `Promise.resolve` stub i `evalSourceWith` (`k1f_async_promise_kab_eval`) |
| **K1g** | **Promise.then microtask** — ✅ **subset**: `.then(cb)` köar microtask; `drainMicrotasks` efter stmt i `evalSourceWith` (`k1g_promise_then_microtask`) |
| **K2** | **DOM + CSS/KSS** — ✅ **subset**: `querySelector` + KSS object→CSS i `.kab` (`kdom_query_kss_smoke`); layout/paint fortfarande Rust |
| **K2 deepen** | **applyCss + matches** — ✅ **subset**: `kdom_applycss_matches_smoke` (kdom + kss + selectors + theme `applyCss`) |
| **K2-layout** | **flex/box orchestration** — ✅ **subset**: `lib/kstyle/layout` `flexColumn`/`flexRow`/`gap`/`pad`/`applyFlex` (stil-helpers; native layout engine kvar) (`k2_layout_smoke`) |
| **K3** | **OS** — ✅ **subset**: VFS + mem + `lib/os/sched` (enqueue/tick/yield/preempt). Gate: `cargo test --test os_lib` |
| **K4** | **Webläsare** — ✅ **subset**: `lib/kbrowser` tabs + VFS navigate + paint (`k4_kbrowser_tabs_smoke`) |
| **K5** | **kOS desktop** — ✅ **subset** (G12.1–G12.5 + launch + Start click + event drain + app body): `launchApp` / `clickStartApp` / `drainKosEvents` → `openWindow` med VFS-body (`kos_launch_app_smoke`, `kos_start_click_smoke`, `kos_event_drain_smoke`, `kos_app_body_smoke`) |

### Våg H — Rust → noll 📋

- Inga nya features i Rust  
- Flytta kvarvarande logik till `.kab` under K  
- Slutmått: produktkod = Kabootar; Rust borta (eller minimal bootstrap som sedan också skrivs om)
- **H0** ✅ — stylesheet apply + document `paint` CSS-path prefererar `.kab` (`parseAndApply` via `kstyle/parse`) istället för enbart native `kstyle_parse`
- **H1** ✅ **subset** — desktop shell boot CSS via `import "kstyle/parse"` + `parseAndApply` (inte native `kstyle_parse`); gate `h1_shell_boot_css_kab`
- **H2** ✅ **subset** — `queryKab` i `lib/kdom/query` (#id / .class / tag via `kstyle/selectors` + walk); `document.query` provar Kab först, fallback `kdom_query_selector` (`h2_query_kab_smoke`)
- **H3** ✅ **subset** — `queryAllKab` i `lib/kdom/query`; `document.domExtra(..., "queryAll")` provar Kab först (`h3_query_all_kab_smoke`); används av `kos/windows` `listWindows`
- **H4** ✅ **subset** — Thin Rust Kv8: produktväg = `evalSource`/`evalSourceKab` → `evalSourceWith`; `evalSourceRust` endast för luckor; `preferKabEval()` sätter flagga (`h4_prefer_kab_eval`)
- **H5** ✅ **subset** — paint/layout-orchestration i `.kab`: `lib/kdom/paint` `paintNode` / `paintWithCss` / `layoutPaint` (flexColumn via `kstyle/layout` + `paint`) (`h5_layout_paint_smoke`)
- **H5b** ✅ **subset** — event drain: `pollEvents` + `drainKosEvents` (Start `launchStartApp`); host shell musklick → `kb_click` → drain → remount/paint (`kos_event_drain_smoke`, `kos_host_click_smoke`)
- **H6** 🚧 **Zero Rust (produkt → bootstrap)** — aktiv huvudlinje. **Regel: inga nya features i Rust.** All produktlogik (Kv8, CSS, DOM, OS, webläsare) → `.kab`; Rust krymper till syscall och sist bootstrap som också skrivs om.

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

**H6d** ✅ **subset** — OS-policy i `.kab`: `os/vfs_policy` (ensureDir/writeFile/apps), `os/sched_policy` (runFairTick), `os/process_policy` (spawnSandbox/caps); `kos/boot` seed via policy (`h6d_os_policy_smoke`). Rust kvar som disk/net/GPU/hw + thin `os_*` syscalls.

**H6e** ✅ **subset → delete-mål** — Kab VM + self-host facades. **Produkt-`import`** prefererar self-host (`load_program_for_file` → `compile_file_prefer_cached`; Rust under `KAB_VM_EXEC_ACTIVE` / skip-listade löv). Tunna facader self-hostar CI-snabbt. **Skip-list (löv kvar):** `emit_impl`, `parser_impl`, `lexer_impl`, `serialize_body`, `vm_run_body`. **Committed seeds:** `self_host/seed/*.kbc` (fingerprint) — kab-only laddar löv utan live-Rust (`backend=seed`); saknas/stale seed → delete-gate. Regenerera: `scripts/regen_self_host_seeds.sh`. **P6 policy:** seed-only (empty skip-list deferred). Smokes: `h6e_kab_*`, `h6e_skip_listed_kab_only_uses_seed`, `p6_seed_only_all_leaves_have_seeds`.

**H6 deepen** ✅ **subset** — `run_file` prefererar self-host compile (`compile_file_prefer_cached`, `KABOOTAR_COMPILE=rust` tvingar host); tab/history-session i `.kab` (`kbrowser/history`, `h6_delete_gate_smoke` / `h6e_run_selfhost_probe`).

**H6 delete-gates** ✅ — chrome nav Kab; query Kab-only; Rust history + tab/back + `kdom_query_selector*` bort; Kab VM subset; **import prefer self-host** + kab-only skip-list-gate (`h6b_query_policy`, `h6c_browser_chrome_smoke`, `h6_delete_gate_smoke`, `h6e_vm_smoke`, `h6e_kab_vm_smoke`).

### Våg P — Performance (snabbare, mer självständig runtime) 📋

**Mål:** Kabootar ska kännas snabbare än typiska scriptmotorer i spel/verktyg, och **inte** behöva C#/C++ för hot paths i produktkoden. Gameplay + editor i `.kab`; tunga loops → bytecode/AOT/`@manual`; ritning → GPU.

**Princip:** mät först (`Deno.bench` / `performance.now` / frame-timing), optimera flaskhalsar, behåll delete-gates (ingen ny produktlogik i Rust utöver tunn GPU/FFI).

| Fas | Innehåll | Status |
|-----|----------|--------|
| **P0** | **Baslinje & profiler** — frame-tid, alloc/frame, bytecode op-histogram; `kabootar bench` / spel-smoke med budget (t.ex. 16.6 ms @ 60 FPS idle) | ✅ subset (`tests/perf_p0_smoke.rs`: `performance.now` + `game_tick`/`delta_ms` < 100 ms CI-smoke; full profiler/histogram kvar) |
| **P1** | **VM hot path** — färre allocs i CALL/INDEX; inline cache för globals/members; snabbare ` AccAdd`/arith redan påbörjad i H6e | ✅ subset (`member_name` → `&str`; GetMember monomorphic IC; CALL skip arg-clone utan Object; MakeArray O(n); IndexGet array-fastpath; `tests/perf_p1_smoke.rs`) |
| **P2** | **Typed arrays / bulk buffers** — `Float32Array`/`Uint8Array` zero-copy till GPU/audio; ingen per-vertex Kab-objekt-loop | ✅ subset (`float32_array_new/get/set`, bulk `createBuffer` från Float32Array; Array-path kvar; Uint8 zero-copy/audio kvar) |
| **P3** | **GC-budget** — incremental/generational eller frame-aware GC så spikes inte dödar 60 FPS; `@manual` för ring buffers | ✅ subset (`gc_frame_stats` / `gc_set_frame_budget`; alloc-räknare + soft sweep i `game_tick`; `tests/perf_p3_gc_frame.rs`) |
| **P4** | **AOT / native code** — `.kbc` → maskinkod eller LLVM/Cranelift-subset för hot fn; cache per fingerprint | ✅ subset (bytecode/`.kbc` fingerprint = AOT-lite; `tests/perf_p4578_smoke.rs` `p4_aot_lite_bytecode_present`; maskinkod kvar) |
| **P5** | **SIMD & math** — vec3/mat4 natives eller `@manual` SIMD för transform (Kab-API, FFI under huven tills self-host) | ✅ subset (`sci_vadd`/`sci_vmul`/`sci_dot` bulk loops; auto-vectorizable; mat4 GPU kvar) |
| **P6** | **Self-host compile-tid** — tömma H6e skip-list (`emit_impl`/`parser_impl`/`lexer_impl`/`serialize_body`/`vm_run_body`); snabbare parse/emit; incremental `.kbc`; **tills vidare:** committed `self_host/seed/*.kbc` | ✅ seed-only (`SELF_HOST_SKIP_LISTED_LEAVES` + `p6_seed_only_all_leaves_have_seeds`; empty list kvar 📋) |
| **P7** | **Modul/import-latens** — disk-`.kbc` + export-cache; kallstart < 100 ms för typiskt spelprojekt | ✅ subset (`compile_file_prefer_cached` second hit → `cache`; `p7_compile_cache_second_hit`) |
| **P8** | **Parallellism** — workers / job-system för asset bake, pathfinding, without blocking render-thread | ✅ subset (`job_map` sequential API; parallel workers kvar) |
| **P9** | **Delete-gate prestanda** — CI-budgetar: VM-smoke, self-host facade < N s, 3D demo ≥ 60 FPS headless/timing | ✅ subset (`perf_p0` delta < 100 ms; `perf_gp5c` avg < 25 ms idle; playable `examples/game_playable_2d.kab`) |

**Checkpoint P:** `cargo test` + spel-bench-smoke + dokumenterade budgets i [GAME.md](GAME.md) / [FEATURES.md](FEATURES.md).

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
| **GP1a** | **Scen-graf** — nodes, transform hierarchy, layers (`import "game/scene"`) | ✅ subset (`Object.setParent` walk + `worldPos` sum; `layer` fält) |
| **GP1b** | **Mesh / material / camera** — Tunna wrappers över WebGL/wgpu | ✅ subset (`import "game/render"`: mesh + indexed/instanced draw helpers) |
| **GP1c** | **Input-lager** — action maps (keyboard/gamepad/touch) ovanpå `input_*` | ✅ subset (`import "game/input"`: `createActions`/`actionPressed`; keyboard only) |
| **GP1d** | **Time & fixed update** — `dt`, fixed physics step, frame skip-policy | ✅ subset (`import "game/time"`: `dtSec`/`createFixed`/`fixedTick`) |
| **GP1e** | **ECS eller komponent-subset** — data-oriented gameplay utan GC-churn | ✅ subset (`import "game/ecs"`: spawn/add/query AoS) |
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
| **GP4b** | **Editor-shell** — scenhierarki + inspector i kOS/kbrowser | ✅ subset (`import "game/editor"`: hierarchy/inspector descriptors) |
| **GP4c** | **Profiler UI** — CPU/GPU/frame graph i DevTools | ✅ subset (`import "game/profiler"`: ring buffer + canvas overlay + `devtools_profile_start`) |
| **GP4d** | **Debug draw** — gizmo lines/colliders | ✅ subset (`import "game/debug"`: line/AABB/circle på canvas2d) |
| **GP4e** | **Dokumentation & samples** — [GAME.md](GAME.md) + `examples/game_*` shippable demos | ✅ subset (`examples/game_2d_smoke.kab`, `examples/game_3d_triangle.kab`) |

#### GP5 — Ship & plattformar

| Fas | Innehåll | Status |
|-----|----------|--------|
| **GP5a** | **Desktop ship** — en binär (`kabootar run` / kOS-app) med GPU | ✅ subset ([SHIP.md](SHIP.md) + `tests/ship_desktop_smoke.rs`) |
| **GP5b** | **WASM host** — `platform_use("host")` + WebGPU/WebGL present | ✅ subset (`import "game/host"`; native fallback `layer=host`; wasm32 web_sys) |
| **GP5c** | **Performance budgets i CI** — 60 FPS smoke (timing), max alloc/frame | ✅ subset (`tests/perf_gp5c_smoke.rs`: avg Δt < 50 ms + `os_mem_stats`) |
| **GP5d** | **Självständighet** — spel + editor kör utan extern Unity/Unreal/C#-toolchain | ✅ subset ([SHIP.md](SHIP.md) GP5d-checklista) |

**GP-ordning (rekommenderad):** GP0a–c → P0/P2 → GP1a–d → GP2a–b → GP4a → GP0f delete-gate → GP3 → GP5.

**Checkpoint GP:** textured 3D demo @ stabil frame-tid; `game` mall; hot reload smoke; docs uppdaterade. **Slutmått:** kan producera och shippa 2D/3D-spel i Kabootar snabbare än motsvarande C#/C++-pipeline, med GPU-prestanda i samma klass som native scriptade motorer.

### Våg SC — Science / AI (ta över Pythons roll) 🚧

**Mål:** `import "science"` (+ `science/nd` / `science/ml` / `science/data`) ska vara det **första valet** för forskning, dataanalys och AI — inte en “lite NumPy”. Kabootar är snabbare än Python; vi ska också vinna **ekosystemet och arbetsflödet** så att forskare och AI-team **inte behöver Python**.

**Slutmål (självständighet):** STEM/AI-stacken är **helt Kabootar** — algoritmer, träningsloopar, datasets, viz, notebooks och modeller i `.kab` / `.kbc`. Rust får **inte** växa som science-produktkod. Tillfällig native-hotpath (SIMD/GPU/FFI) är OK tills motsvarande går i Kab VM + thin host; därefter **delete-gate**. Kabootar blir **fri och självständig**: ingen Python, ingen NumPy/SciPy-pip, ingen PyTorch-runtime som beroende.

**Nuläge (bas):** komplex, fysik/kemi/ekonomi, statistik, `mat_*` / `num_*`, **ndarray** (`nd_*`), **ML** (`ml_*` / `ag_*`), signal (FFT), CSV/pretty, GPU-staging. Se [SCIENCE.md](SCIENCE.md).

#### Policy — bygg science i Kabootar, inte i Rust

| Regel | Innebörd |
|-------|----------|
| **Kab-first** | Ny API (`lib/science/*.kab`) + examples/tester i Kab **före** eller **istället för** ny Rust-yta |
| **Inga nya Rust-features** | Undantag: tillfällig hotpath (matmul/FFT/GPU) bakom stabil Kab-API — samma H6-regel som övrig plattform |
| **Port-plan** | Befintliga Rust-natives (`src/runtime/science/*`) → `lib/science/` när språk/VM räcker (SC5) |
| **Delete-gate** | CI-smoke för typisk research/AI-loop **utan** Python/NumPy/PyTorch |
| **Frihet** | Ingen runtime-beroende på CPython, pip-wheels eller proprietär AI-runtime |

#### Gap-analys — Kab vs NumPy / SciPy / Python-AI

Jämförelse mot det forskare faktiskt använder. ✅ = subset landad · 🟡 = delvis · ❌ = saknas (roadmap nedan).

| Område | Python-stack | Kab idag | Behövs för att konkurrera |
|--------|--------------|----------|---------------------------|
| **Ndarray-kärna** | NumPy: dtype, broadcast, slice/view, ufunc, stack/concat, fancy index | 🟡 contiguous f64, add/mul/sum/mean, F64 zero-copy | Full broadcast, slice/views, dtypes, `where`/`clip`, concat/stack, random |
| **Linalg** | `numpy.linalg` / SciPy: LU, QR, SVD, eig, Cholesky, lstsq, norms | 🟡 solve, 2×2 eig/SVD, matmul/dot | Allmän QR/SVD/eig, Cholesky, lstsq, condition, batched |
| **Numerik / SciPy** | `optimize`, `integrate`, `interpolate`, `special` | 🟡 trapz/simpson/newton/bisect/interp_linear | Minimize/least_squares, ODE, spline, erf/gamma/bessel subset |
| **Signal** | `scipy.signal`: FFT n-D, filter, spectrogram, resample | 🟡 1D FFT/IFFT, conv1d | 2D FFT, FIR/IIR, window, STFT |
| **Sparse** | `scipy.sparse` + sparse linalg | ❌ | CSR/COO + SpMV + sparse solve subset |
| **Stats** | `scipy.stats` / pandas describe | 🟡 mean/var + `table_describe` | Fördelningar, hypotest, corr/cov, quantiles, groupby |
| **Tabell / I/O** | pandas, CSV/Parquet | 🟡 CSV parse/load | Typed columns, join/groupby, Parquet/JSONL, `nd_save`/`nd_load` |
| **Viz** | Matplotlib / Plotly | 🟡 ASCII + pretty; canvas kvar | Canvas2d line/scatter/hist + notebook-inline |
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
| **SC0e** | **Broadcast + ufunc** — NumPy-style broadcasting, `where`/`clip`/`abs`/`exp`/`log` | ✅ subset (`nd_add`/`nd_mul`/`nd_sub`/`nd_div` broadcast; `nd_where`/`nd_clip`/`nd_abs`/`nd_exp`/`nd_log`/`nd_sqrt`) |
| **SC0f** | **Slice / view / stack** — ranges, `concat`/`stack`/`split` (copy-slice) | ✅ subset (`nd_slice`/`nd_concat`/`nd_stack`/`nd_split`; view-semantik kvar) |
| **SC0g** | **Dtypes** — f32/f64/i32/i64/bool (+ complex64 senare); cast | ✅ subset (`nd_dtype`/`nd_astype`; complex kvar) |
| **SC0h** | **Random** — seed, uniform/normal, shuffle (Kab-API) | ✅ subset (`nd_seed`/`nd_rand_uniform`/`nd_rand_normal`; shuffle via `ml_shuffle`) |
| **SC0i** | **I/O** — `nd_save` / `nd_load` (binär/VFS; npy-inspirerat) | ✅ subset (KND1 binary) |

#### SC1 — Linear algebra & numerik (SciPy-klass)

| Fas | Innehåll | Status |
|-----|----------|--------|
| **SC1a** | **matmul / tensordot subset** på ndarray (utöver `mat_mul`) | ✅ subset (`nd_matmul` / `nd_dot`) |
| **SC1b** | **Solve / LU** — `nd_solve` / `mat_solve` för Ax=b | ✅ subset (`nd_solve` Gauss+partial pivot) |
| **SC1c** | **Decomps subset** — QR/SVD/eig (start: 2×2; utöka) | ✅ subset (`mat_svd2` + `mat_eigen2`; allmän → **SC1e**) |
| **SC1d** | **FFT / signal subset** — 1D FFT + conv | ✅ subset (`num_fft` / `num_ifft` / `num_conv1d`) |
| **SC1e** | **Full linalg** — QR, tunn/full SVD, eig/sym, Cholesky, lstsq, `cond` | ✅ subset (`mat_qr`/`mat_svd`/`mat_eig`/`mat_cholesky`/`mat_lstsq`/`mat_cond`) |
| **SC1f** | **Optimize** — `minimize` (gradient/Nelder), `least_squares`, `root` | ✅ subset (`num_minimize` Nelder–Mead, `num_least_squares`, `num_root`) |
| **SC1g** | **Integrate / ODE** — quad + `odeint`/`rk4` för system | 📋 |
| **SC1h** | **Interpolate / special** — spline1d; `erf`/`gamma`/`bessel` subset | 📋 |
| **SC1i** | **Signal++** — 2D FFT, window, FIR/IIR, STFT/spectrogram | 📋 |
| **SC1j** | **Sparse** — CSR/COO, SpMV, sparse least-squares subset | 📋 |

#### SC2 — ML / AI (ersätt sklearn + PyTorch-subset)

| Fas | Innehåll | Status |
|-----|----------|--------|
| **SC2a** | **Aktiveringar + loss** — relu/sigmoid/softmax, mse/cross-entropy | ✅ subset (relu/sigmoid/softmax/mse; **CE kvar**) |
| **SC2b** | **Dense forward + SGD** — `ml_dense`, `ml_sgd_update` | ✅ subset (+ `ml_linreg_step`) |
| **SC2c** | **Autograd-lite** — tape för dense/relu/mse | ✅ subset (`ag_*`); **bredda ops → SC2f** |
| **SC2d** | **Dataset / batch** — shuffle, mini-batch, train/test split (**i Kab**) | ✅ subset (`ml_shuffle`/`ml_batch_slices`/`ml_train_test_split`) |
| **SC2e** | **Model I/O** — spara/ladda vikter (JSON/VFS/checkpoint) | ✅ subset (`ml_save_checkpoint`/`ml_load_checkpoint`) |
| **SC2f** | **Autograd++** — tape: matmul, conv, softmax, CE; `no_grad`; högre ordning senare | ✅ subset (`ag_matmul`/`ag_softmax`/`ag_ce`/`ag_add`/`ag_mul`/`ag_no_grad`; conv kvar) |
| **SC2g** | **Optimizers + metrics** — Adam/AdamW; accuracy/F1/ROC-AUC/confusion | ✅ subset (`ml_adam_update`/`ml_accuracy`/`ml_f1`/`ml_confusion`; AdamW/ROC kvar) |
| **SC2h** | **Klassisk ML** — PCA, k-means, logreg, decision stump/tree subset, pipeline | ✅ subset (`ml_pca`/`ml_kmeans`/`ml_logreg_*`; tree/pipeline kvar) |
| **SC2i** | **NN-lager** — Conv2d, MaxPool, Embedding, MultiheadAttention-lite | 📋 |
| **SC2j** | **Training DX** — `fit`-loop, early stop, schedulers, progress i REPL/notebook | 📋 |
| **SC2k** | **Tokenizer + transformer inference** — BPE/WordPiece subset + forward (train senare) | 📋 |
| **SC2l** | **AI delete-gate** — tränings-/inference-smoke **utan** Python/PyTorch i CI | 📋 |

#### SC3 — Data, viz, notebooks-DX

| Fas | Innehåll | Status |
|-----|----------|--------|
| **SC3a** | **CSV/JSON tabular** — `science/data` load/describe | ✅ subset (`csv_parse`/`csv_load`/`table_describe`) |
| **SC3b** | **Plot subset** — ASCII + pretty + **canvas2d** line/scatter/hist | ✅ subset (`ascii_plot`/`plot_line`/`plot_scatter`/`plot_hist`) |
| **SC3c** | **`kabootar mod init science-ai`** — mall + examples | ✅ subset |
| **SC3d** | **Docs & benches** — SCIENCE.md + CI vs Python-baslinjer | ✅ subset; 📋 utökade benches |
| **SC3e** | **Exploration DX** — REPL/notebook (se **Våg DX**) | ✅ DX0–DX5 subset |
| **SC3f** | **DataFrame-lite** — kolumntyper, select/filter, groupby/agg, join | ✅ subset (`df_from`/`df_select`/`df_filter`/`df_groupby`/`df_join`/`df_head`) |
| **SC3g** | **Stats++** — fördelningar, t-test/χ², corr/cov, quantiles | 📋 |
| **SC3h** | **Notebook rich display** — plot/table inline i `.knb` / WASM | ✅ subset (`rich_display` + `session_eval_rich` + notebook HTML) |

#### SC4 — Scale & hardware

| Fas | Innehåll | Status |
|-----|----------|--------|
| **SC4a** | **SIMD / BLAS-FFI** — matmul hotpath (kopplat P5) | ✅ subset (`sci_v*`; **BLAS kvar**) |
| **SC4b** | **GPU tensors** — wgpu compute för matmul/conv (kopplat GP0) | ✅ staging; 📋 riktig compute kernel |
| **SC4c** | **Workers** — parallell map över batch (kopplat P8) | ✅ sequential API; 📋 parallel |
| **SC4d** | **Delete-gate** — ML-smoke utan Python/NumPy i CI | ✅ subset |
| **SC4e** | **GPU train/infer path** — matmul/conv på device; host sync explicit | 📋 |
| **SC4f** | **Bench harness** — Kab vs NumPy/PyTorch timing i CI (dokumenterad, inte blocker) | 📋 |

#### SC5 — Science self-host (Kab-only, fri från Rust)

Målet: science-modulen blir **självständig** — skriven och underhållen i Kabootar.

| Fas | Innehåll | Status |
|-----|----------|--------|
| **SC5a** | **Kab-algoritmer** — linalg/optimize/stats/ML-träningsloopar i `lib/science/*.kab` (anropa bara tunna primitives) | 📋 |
| **SC5b** | **Port natives → Kab** — `nd_*`/`ml_*`/`num_*` logik som kan vara ren Kab flyttas; Rust krymper till buffer/SIMD/GPU | 📋 |
| **SC5c** | **Inga nya Rust-science-API** — CI/policy: nya exports bara via `lib/science` | 📋 |
| **SC5d** | **Hotpath-kontrakt** — dokumenterad lista: vilka ops får native (matmul, FFT, GPU); resten Kab | 📋 |
| **SC5e** | **Science bootstrap** — `import "science"` fungerar i kab-only/seed-läge utan att växa Rust-ytan | 📋 |
| **SC5f** | **Frihets-gate** — research+AI demo (data→train→plot→HTTP) 100 % Kab-toolchain; dokumentera “no Python required” | 📋 |

**SC-ordning:** SC2i–j → SC1g–j → SC3g → SC2k → SC4e → **SC5** (Kab-first parallellt).

**Checkpoint SC (nästa):** Conv2d/attention-lite + ODE/stats++ + GPU train path.  
**Checkpoint SC (landad 2026-08):** dtypes/I/O/autograd++/DataFrame; **PCA/k-means/logreg + optimize + rich notebook display**.  
**Checkpoint SC (research-parity):** ODE + stats++ + Conv2d.  
**Checkpoint SC (AI-parity):** GPU matmul + tokenizer/transformer-inference + SC2l.  
**Slutmått:** forskare och AI-utvecklare använder Kabootar **istället för Python** — från notebook till ship — med numerisk hotpath i NumPy/PyTorch-klass och **helt självständig** stack (SC5f).

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
| **DX7** | **DX self-host** — REPL/session-hjälpare i `.kab` där det går; thin host kvar | 📋 |

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
| **D9** ⏸ | **kOS desktop shell** — [G12](ROADMAP.md) / **K3** i `.kab` |

**Våg D totalt:** host-subset klart; resten via K/H

### Total kalender (en utvecklare, heltid) — omställd

```
Våg L (språk)  ████████░░░░░░░░  nu — L1 först
Våg S (host)   ░░░░████░░░░░░░░  efter L1–L3
Våg K (libs)   ░░░░░░██████████  kv8/os/kos i .kab
Våg H (thin)   ░░░░░░░░░░██████  frys Rust-yta
Våg P (perf)   ████████████████  P0–P9 subset (AOT-maskinkod / parallel workers kvar)
Våg GP (spel)  ████████████████  GP0–GP5 subset
Våg SC (STEM)  █████████████░░░  SC1f/SC2h/SC3h landad; Conv/ODE/SC5 kvar
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

Bygger på **Våg E** (fn-generics v1). Se [GENERICS.md](GENERICS.md#fas-2--g6-planering).

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
| **G7** | **kbrowser mobil** — viewport/touch/safe area + mobil shell-UI (`lib/kbrowser/mobile_chrome`) | ✅ subset |
| **G8** | **Compile-opt** — incremental self-host, [COMPILE.md](COMPILE.md) | ✅ subset (`.kbc` fingerprint + import mtimes) |
| **G9** | **Kv8 i Kabootar** — lexer/parser/eval Kv8-subset self-host | ✅ subset (`?.`/templates `${expr}`/ternary/`switch`/array/unary/`for*`/try/fn) |
| **G10** | **React/Next-lik** — Kv8 fiber + kDOM SSR (`import "kv8/react"`) | ✅ subset (`ntag`/`cnid*` multi nested + parent live sync/`onById`/`dispatchById`) |
| **G10b** | **Runtime MemBox** — opt-in `@manual` + `owned_*` / `import "os/mem"` (GC default orörd). Compile-time = **Våg O** | ✅ runtime; O1–O3 ✅ |
| **G11** | **kbrowser cross-platform** — `lib/kbrowser/` + `kb_sync_platform` object + native/kos/wasm smokes | ✅ subset |

**G7 — kbrowser mobil (planering):**

Kabootar Browser ska fungera på **mobiltelefoner** — Android och iPhone — med samma `kb_*`-API som desktop.

| Mål | Väg | Smoke |
|-----|-----|-------|
| **Android (web)** | Chrome / WebView + WASM + touch | `kb_viewport` + touch hit-test |
| **Android (app)** | Kabootar Shell (WebView) + PWA manifest | install + offline VFS |
| **iPhone (web)** | Mobile Safari + WASM | safe area + viewport |
| **iPhone (app)** | WKWebView Shell + PWA | App Store-ready wrapper |

Krav:

- [x] **Touch-input** — `kb_touch_at` + hit-test (`kb_poll_events`, fallback till `click`)
- [x] **Responsiv viewport** — `kb_viewport(w, h, dpr?, orientation?)` returnerar `{width,height,dpr,orientation}`
- [x] **iOS safe area** — `kb_safe_area(top?, right?, bottom?, left?)` stub
- [x] **Mobil shell-UI** — `lib/kbrowser/mobile_chrome.kab` (adressfält, tillbaka, flikar)
- [x] **PWA** — service worker + fetch events + manifest ([BROWSER_V2.md](BROWSER_V2.md)); “Lägg till hemskärm”
- [x] **Smokes** — `examples/kbrowser_mobile_smoke.kab`, `kbrowser_mobile_shell_smoke.kab`; device CI senare

Beror på: **G11** (kbrowser core), **Våg C** (layout, touch targets), **BROWSER_V2 PWA**.

**G11 — kbrowser på desktop-mål (planering):**

Kabootar Browser (`kbrowser`) ska inte bara köras i Chrome/WASM — den ska vara **första-klassens** på Kabootar OS (kOS) *och* på varje host där motorn byggs:

| Mål | Renderingsväg | Smoke |
|-----|---------------|-------|
| **kOS** | VFS (`kabootar://`), compositor, OS-fönster | `kb_navigate("kabootar://vfs/…")` + `kb_paint()` |
| **Windows** | Native shell / pixel-compositor | `file:///…`, `kb_host_sync()` |
| **Linux** | Native shell (X11/Wayland bridge) | samma API som Windows |
| **macOS** | Native shell (AppKit bridge) | samma API som Windows |
| **WASM** | `kabootar-shell.html` + host canvas | `wasm-pack` + `kb_run_ui()` |

Krav:

- [x] **`lib/kbrowser/`** — `core.kab` + `mobile_chrome.kab` + aggregator; Rust som host-bindning
- [x] **`kb_sync_platform()`** — returnerar `{mode,layer,host_os,schemes}`
- [x] **Enhetlig compositor-yta** — `kb_mount` → `kb_render` → `kb_paint` (kOS/native/wasm-klass)
- [x] **CI-smokes** — `kbrowser_native_smoke` / `kbrowser_kos_smoke` / `kbrowser_wasm_smoke` + host-tester
- [x] **Dokumentation** — matris i [BROWSER.md](BROWSER.md); native AppKit/X11-bridge senare

Mobil (Android, iPhone): se **G7** — samma `lib/kbrowser/`, touch + viewport + PWA/Shell.

Beror på: **G6–G10** (kDOM/Kv8/kss), **Våg C** (layout/canvas), **`lib/os/*`** (VFS, async, fönster).

| **G12** | **kOS desktop shell** — Windows-lik UX (taskbar, Start, fönster, Explorer) med modern stack (kDOM/KSS, GPU compositor, blur, animationer); se [OS.md#desktop--utseende](OS.md#desktop--utseende) | ✅ **subset** (G12.1–G12.5) |

**G12 — kOS utseende (planering):**

Målbild: användaren ska känna igen sig från Windows, men systemet ska **se och kännas 2020+-modernt** — inte Win32.

| Komponent | Innehåll | Teknik |
|-----------|----------|--------|
| **Shell** | Taskbar, Start/meny, systemfält, klocka | `lib/kos/shell.kab` |
| **Fönsterhanterare** | Titelfält, min/max/stäng, snap-zoner, Alt+Tab | `os_window_*` + compositor |
| **Explorer** | `kabootar://vfs`, mount, sökvägsfält | kbrowser + VFS |
| **Settings** | Kategorier (system, nät, skärm, integritet) | Kv8-app i VFS |
| **Tema** | Mörk/ljust, accentfärg, skal-transparens | KSS tokens + `kb_theme()` |
| **Rörelse** | Öppna/stäng, snap, hover | spring physics (`os_haptic_*`), vsync |

Milstolpar:

- [x] **G12.1** — Minimal shell: skrivbord + taskbar + ett fönster — ✅ **subset** via `lib/kos/shell` (`buildShell` / `listApps`)
- [x] **G12.2** — Start + app-lista från VFS (`/apps`) — ✅ **subset**: `openStart` / `isStartOpen` / `clickStart`; **Start → openWindow** via `lib/kos/launch` `launchApp` / `launchStartApp` / `drainKosEvents`; **Start click** via `wireStartApps` + `clickStartApp` (dispatch → drain) (`kos_launch_app_smoke`, `kos_start_click_smoke`, `kos_event_drain_smoke`); app-body från `os_read(/apps/…)` (`kos_app_body_smoke`)
- [x] **G12.3** — Explorer + filoperationer (`os_read`/`write`/`list`) — ✅ **subset**: `lib/kos/explorer` (`kos_g12_3_explorer_smoke`)
- [x] **G12.4** — Snap + multi-fönster + Alt+Tab-overlay — ✅ **subset**: `lib/kos/windows` (`openWindow` / `snapWindow` / `openAltTab`; `kos_g12_4_windows_smoke`)
- [x] **G12.5** — Visuell polish: blur-lager, rundning, animationer, ljust tema — ✅ **subset** (CSS polish via `lib/kos/theme` `applyKosTheme`; inte full GPU blur) (`kos_g12_5_theme_smoke`)

**Shell-integrering:** `bootKosDesktop()` i `lib/kos/shell` (build + theme + `kb_mount`/`kb_paint`). `kabootar shell` monterar Start + `/apps`, mappar vänsterklick → `kb_click` → `drainKosEvents` → remount/paint. CI: `kos_lib` gate i `self-host.yml`. ✅ **shell mount + input subset**. Exempel: `examples/kos_shell_mount_smoke.kab`, `examples/kos_host_click_smoke.kab`.

Beror på: **G11** (kbrowser), **Våg D5** (GPU compositor), **Våg C4** (layout).

**Tester:** `cargo test stdlib_wave`, `cargo test --test kabootar_js_parity`, [VSCODE_TESTS.md](VSCODE_TESTS.md).

---

## Bidra

Varje fas bygger på föregående. Se `src/` och öppna issues för diskussion om API-design.
