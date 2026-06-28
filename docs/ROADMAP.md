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

## Master fetch-plan (2026–2027)

Fyra vågor i ordning — inget hämtningsbart ska lämnas mellan vågarna utan medvetet beslut.

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
| **B1** ✅/🚧 | `Deno.serve` async + HTTP/2, WebSocket framing polish — **async bakgrundstråd** ✅, HTTP/2 kvar |
| **B2** ✅ | Full WHATWG Streams (backpressure, cancel, tee edge cases) — grund ✅ |
| **B3** ✅ | `Deno.permissions` / capability prompts |
| **B4** ✅ | `Deno.test` / `Deno.bench` inbyggt |
| **B5** ✅/🚧 | npm/JSR: fler paket, native addons policy, lockfile — **integrity från cache** ✅ |
| **B6** ✅ | `Deno.cwd`/`chdir` + `Deno.realPath`/`Deno.symlink`/`Deno.link` |
| **B7** ✅ | `Deno.listenTls` / ALPN / cert reload — TLS server ✅, ALPN kvar |
| **B8** ✅ | Worker: SharedWorker, `postMessage` transferables full lista — in-process ✅ |

**Våg B totalt:** ~1–2 månader

### Våg C — DOM / kDOM / Kv8 (~60–70 % av browser-ytan) 🚧

**Status (2026-06):** C1 påbörjad — `kdom_query_selector`, `kdom_query_selector_all`, `kdom_mutation_records`. **C2 påbörjad:** React 19 via esbuild ESM (ingen UMD), Kv8 modern JS (`for…of`, templates, `??`, `?.`, `import`/`export`), fast/slow CI (`scripts/kv8-test-*.sh`). React bundle **parsar** (~0,1 s); full **eval** i Kv8-tolkaren tar fortfarande minuter — nästa milestone: render/fiber-paritet + tolk-prestanda.

| Fas | Innehåll |
|-----|----------|
| **C1** 🚧 | **kDOM** — full DOM Level 2: Events, MutationObserver, `querySelector` alla selectors |
| **C2** 🚧 | **Kv8** — modern JS + ESM/esbuild React 19; full bundle eval/render parity; `class`/`async` i parsern |
| **C3** | **Kv8 JIT** — fler hot paths, delad bytecode med huvudspråket |
| **C4** | **Layout** — flexbox/grid (subset), box model polish |
| **C5** | **Canvas 2D** — full HTML5 canvas API ([CANVAS.md](CANVAS.md) luckor) |
| **C6** | **WebGL** — textures, FBO, shaders från GLSL-filer |
| **C7** | **WebRTC** — stub → produktion (ICE, DTLS-SRTP) |
| **C8** | **PWA/Extensions** — service worker fetch events, extension permissions |
| **C9** | **DevTools** — network panel, profiler, live edit |

**Våg C totalt:** ~4–6 månader

### Våg D — OS (stub → native, ~40–50 %)

| Fas | Innehåll |
|-----|----------|
| **D1** | Ring 0: scheduler/CFS produktion, riktig preemption |
| **D2** | MMU: page faults, COW, mmap |
| **D3** | FS: ext4-lik journal, permissions, ACL |
| **D4** | Netstack: riktig NIC-driver (`--features hw`) |
| **D5** | GPU compositor: multi-monitor, vsync |
| **D6** | `os_compat_run`: Wine-lik / container (inte 99 % stub) |
| **D7** | Boot: BIOS/UEFI chain eller bare-metal target |
| **D8** | Sauce-strategier: haptic, seamless, energy — hardware där möjligt |

**Våg D totalt:** ~6–12 månader

### Total kalender (en utvecklare, heltid)

```
Våg A (JS)     ████████████░░░░  mån 1–4
Våg B (Deno)   ░░░░████░░░░░░░░  mån 4–5
Våg C (DOM)    ░░░░░░██████████  mån 5–10
Våg D (OS)     ░░░░░░░░░░██████  mån 10–18+
```

**Checkpoint efter varje våg:** `cargo test` full suite + uppdatera [FEATURES.md](FEATURES.md).

---

## Bidra

Varje fas bygger på föregående. Se `src/` och öppna issues för diskussion om API-design.
