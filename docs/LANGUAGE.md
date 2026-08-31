# Kabootar — språket

> **Nolltolerans:** produkten ska vara **bara `.kab`** (även JIT, GC, stdlib, OS, CLI). Plan: [ROADMAP.md — egna fötter](ROADMAP.md#kabootar-på-egna-fötter--noll-rust). Fart: [Våg FT](ROADMAP.md#våg-ft--fart-alla-tekniker-i-kab). Test: [kabtest](../lib/kabtest/README.md).

> **Kan du redan JavaScript?** Läs [JAVASCRIPT.md](JAVASCRIPT.md) — där står **bara skillnaderna**. Du behöver inte gå igenom grundsyntaxen här nedan.

## Grundsyntax

Kabootar har C/Rust/JavaScript-liknande syntax:

```kabootar
let x = 42;
let name = "Kabootar";

if x < 100 {
    println(x)
} else {
    println(name)
}

while x > 0 {
    x = x - 1
}
```

## Variabler

```kabootar
let a = 10;        // initierad
let b;             // undefined (binding finns, inget värde än)
b = 5;             // tilldelning
```

**Skillnad mot JavaScript:** `let b;` ger `undefined` i variabeln, inte `null`. Att läsa en **odeklarerad** variabel är ett fel.

Se [TYPES.md](TYPES.md) för när du ska använda `null` respektive `undefined` — båda är förstklassiga literalvärden i språket.

## Literaler

| Literal | Betydelse |
|---------|-----------|
| `42` | heltal |
| `3.14` | flyttal |
| `"text"` | sträng |
| `true` / `false` | booleskt |
| `null` | explicit tomhet |
| `undefined` | oinitierad binding |
| `NaN` | endast flyttal (se TYPES.md) |
| `[1, 2, 3]` | array (v1.9) |

## Array-literaler (v1.9)

```kabootar
let nums = [1, 2, 3];
let mixed = [1, 2.5, "x"];
```

Returnerar `Array` — samma typ som `cplx()` och SQL-resultat.

`let [a, ...rest] = nums` (self-host + Kab-VM `array_slice_rest`).
`let { a, ...rest } = obj` (self-host + Kab-VM `object_rest`).
Nested: `let { x: [a, b] } = o` (self-host + Kab-VM `get_member`/`index_get`).
Array-spread: `[1, ...xs]` (self-host + Kab-VM `concat_array`).
Objekt-spread: `{ a: 1, ...obj }` (self-host + Kab-VM `merge_object`).
Objekt-shorthand: `{ a }` → `{ a: a }` (self-host + Kab-VM `make_object`).
Objekt-metod: `{ add(a, b) { return a + b } }` (self-host + Kab-VM `make_arrow_fn`).
Computed nyckel: `{ [k]: v }` (self-host + Kab-VM `index_set`).

## Kontrollflöde

- `if` / `else`
- `while` / `break` / `continue`
- `for … in` / `for … of`
- `match` (Rust-liknande)
- Python-lån: `pass`/`assert`/`not`/`raise` (self-host + Kab-VM), `with x as name { }`, `is` / `is not` (self-host + Kab-VM `object_is`)

## Funktioner

```kabootar
fn add(a, b) {
    return a + b
}

fn greet(name, hello = "hi") {
    return hello
}
```

Spread i anrop: `f(...xs)` och `Pair(...args)` (self-host parse/emit + Kab-VM `call_from_array` / `new_instance_from_array`).

Rest-parametrar: `fn f(a, ...xs)`, `(a, ...xs) =>`, klassmetod `fn rest(a, ...xs)`, objekt-metod `{ rest(a, ...xs) {} }` och trait default-metod `fn rest(a, ...xs)` (self-host + Kab-VM packar extra args till en array; rust `try_compile` vägrar rest).

Default-parametrar: `fn f(a, b = 3)`, `(a, b = 3) =>`, klassmetod `fn add(a, b = 3)`, objekt-metod `{ add(a, b = 3) {} }` och trait default-metod `fn add(a, b = 3)` (self-host + Kab-VM via `jump_if_not_nullish`; rust bytecode-compile vägrar defaults). Missing args är `undefined` och får default. Rust `try_compile` är inte produktväg.

Parametriska funktioner (`fn id<T>(x: T)`) är **planerade** — se [GENERICS.md](GENERICS.md). Inte tillgängligt i motorn än.

## Inbyggda anrop

| Funktion | Beskrivning |
|----------|-------------|
| `println(...)` | Skriv till konsol |
| `is_null(x)` | Testa `null` |
| `is_undefined(x)` | Testa `undefined` |
| `is_nan(x)` | Testa flyttals-NaN |
| `sql("…")` | Kör SQL mot inbyggd DB (stödjer `$1`-parametrar) |
| `kml("…")` | Parsa KML till Kabootar DOM |
| `kdom_render(node)` | Rendera DOM-nod till HTML |
| `os_info()` | Kernel-info |
| `os_caps()` | Aktiva kernel-kapabiliteter |
| `os_mkdir(path)` | Skapa katalog |
| `os_stat(path)` | `[typ, storlek]` — `file` eller `dir` |
| `os_read(path)` | Läs fil från virtuellt FS |
| `os_write(path, content)` | Skriv fil |
| `os_exists(path)` | Kontrollera om fil finns |
| `os_list(dir)` | Lista filer i katalog |
| `os_delete(path)` | Ta bort fil |
| `http_route(method, path, fn)` | Registrera HTTP-route |
| `http_request(method, path)` | Simulera HTTP-request |
| `http_response(status, body)` | Skapa HTTP-response |

## Moduler

```kabootar
import "math";
add(1, 2);
```

Se [MODULES.md](MODULES.md).

## Globala runtime-objekt

| Namn | Beskrivning |
|------|-------------|
| `document` | Värd-webbläsarens DOM (WASM) |
| `kdom` | Kabootars egen DOM |
| `os` | OS-handle |
| `db` | Databasanslutning |

## WASM API

```javascript
import init, { evaluate } from './pkg/kabootar.js';
await init();
evaluate('let i = 0; while i < 5 { i = i + 1 }; i'); // "5"
```

## 20 utmärkande språkfunktioner

Kabootar som systemspråk — vad som finns idag vs. roadmap. Kör `lang_info()` för live-status.

| # | Funktion | Status | API |
|---|----------|--------|-----|
| 1 | Zero-FFI OS | ✅ | `os_syscall`, `lang_syscalls` |
| 2 | Comptime 3.0 | ✅ | `comptime { }` foldas till literal vid compile; `comptime_assert` |
| 3 | Aktörer | 🔶 | `actor Name { }`, `actor_spawn` |
| 4 | Hot reload | 🔶 | `kabootar serve --watch` + kbc-invalidate |
| 5 | Auto-SIMD | 🔶 | `@simd` (dokumentation) |
| 6 | Valfri minne | ✅ O1–O3 | Default **GC** (web). Systems: `@manual` + compile-time Owned/`&`/`&mut` — [OWNERSHIP.md](OWNERSHIP.md) |
| 7 | Web-native | 🔶 | `html! { }` → Kv8 |
| 8 | Verktygskedja | 🔶 | `compile`, `fmt`, `registry_*` |
| 9 | Statisk binär | 🔶 | `cargo build --release` |
| 10 | Match + guards | ✅ host; self-host array/objekt/`n @ pat`/or/range/enum; **kab-only** const/`_` / `[x, y]` / `{ p, q }` / `1..=5` / `n @ 1..=5` / `1 | 2 | 3` / `(1 | 2)` / `..5` / `5..` / `[h, ...t]` / `{ k, ...s }` / `[h, ...mid, last]` / `if n != 3` / `Color.Red` / `Msg.Move(p)` / `xs @ [p, q]` / `wrap @ { k, ...s }` / `{ k: n @ 1..=5 }` / `[n @ 1, ...r]` / `Ok(n @ 1..=5)` / `Some(n @ 1..=5)` / `Option.Some(n)` / `Option.Some("x")` / `Option<Number>.None` / `1.0..=2.0` / `Result.Ok(n)` / `Result<Number, String>.Err` / `n @ 1 | 2` / `v @ Msg.Move(x)` | `match 1 { 1 => 2, _ => 0 }` → `jump_unless_const_eq`; **`match [1, 2] { [x, y] => x + y }`** → `jump_unless_array`; **`match { p, q }`** → `jump_unless_has_member`; **`match 5 { 1..=5 => 9 }`** / **`match 1.5 { 1.0..=2.0 => 9 }`** → `ge`/`le`; **`n @ 1..=5`** → DUP + `store_local` + range; **`match 2 { 1 | 2 | 3 => 9 }`** / **`(1 | 2)`** / **`1 | (2 | 3)`** / **`n @ 1 | 2`** → `emitMatchPatOr`; **`v @ Msg.Move(x)`** → DUP + `unpack_enum_fields`; **`match 3 { ..5 => 1 }`** / **`5..`** → open `hasLo`/`hasHi`; **`[h, ...t]`** → `array_slice_rest`; **`{ k, ...s }`** → `object_rest`; **`[h, ...mid, last]`** / **`[h, ..., last]`** → `index_peek_from_end`; **`n @ 1..=5 if n != 3`** → `ne` + `jump_if_false`; **`Color.Red`** → `jump_unless_enum_variant`; **`Msg.Move(p)`** → `unpack_enum_fields`; **`xs @ [p, q]`** / **`xs @ [h, ...t]`** / **`wrap @ { k, ...s }`** → DUP + `store_local` + inner; **`{ k: n @ 1..=5 }`** → field + at + range; **`[n @ 1, ...r]`** → piece + at + rest; **`Ok(n @ 1..=5)`** / **`Some(n @ 1..=5)`** / **`Err(n @ 1..=5)`** → unwrap + at + range; **`Option.Some(n)`** / **`Option.Some("x")`** / **`Option<Number>.None`** → `jump_unless_enum_variant` (`Option$Number` / `Option$String` prefix); **`Result.Ok(n)`** / **`Result<Number, String>.Err`** → `Result$Number_String` ([ROADMAP L6](ROADMAP.md#våg-l--language-systems-ready--subset)) |
| 10b | User `enum` | ✅ host + self-host unit/payload; **kab-only** `match Color.Red` / `Msg.Move(p)` / `v @ Msg.Move(x)` / `Option.Some(n)` / `Option.Some("x")` / `Result.Ok(n)` | `enum Color { Red, Green }`, `Color.Red`, `Msg.Move(x)` → `jump_unless_enum_variant` / `unpack_enum_fields`; **`enum Option<T>`** → `Option$Number` / `Option$String`; **`enum Result<T, E>`** → `Result$Number_String` |
| 10c | `if let` / `while let` | ✅ host; self-host socker över `match`; **kab-only** `Some`/`Ok` / `1 | 2` / `n @ Some(x)` / **`1..`** / **`while let 1..`** / **`..5`** / **`while let ..5`** | `if let Some(x) = Some(3)` → `jump_unless_option_some`; `while let Ok(v) = r` → `jump_unless_result_ok` + `break` på miss; **`if let 1 | 2 = x`** / **`while let 1 | 2 = r`** → `emitMatchPatOr`; **`if let n @ Some(x) = Some(6)`** → DUP + `store_local` + Some unwrap; **`if let 1.. = x`** / **`while let 1.. = r`** → `emitMatchPatRange` `ge`; **`if let ..5 = x`** / **`while let ..5 = r`** → `emitMatchPatRange` `lt` |
| 10d | Class field types | ✅ | `age: number` valideras vid tilldelning |
| 10e | `struct` + `&self` | ✅ host; self-host parse+emit + **`struct Box<T>`** med fälttyp `T`; **kab-only** `Box(42)` / `Box("x")` / `Box$Number` / `Box$String` / **`struct WBox<T> where T: Show`** / `WBox$Shown` / **`WBox<Nope>`** reject | `fn get(&self)`; `let b = Box(42)` → `Box$Number`; `Box("hi")` → `Box$String` ([ROADMAP R3](ROADMAP.md)/[R4](ROADMAP.md#våg-r--struct-rust-inspirerat-)) |
| 10f | Trait default-metoder | ✅ host; self-host emit+inject; **kab-only** `trait Show<T>` / `implements Show<Number>` / `Show$Number` / **`Thing().id()`** / `iface_method_default` / **`id() { return 42 }`** / **`Show<T> default`** / **`Show<T> default override`** | `trait HasId { fn id() { return 1 } }` + `class C implements HasId`; klass `fn id()` vinner över default; `trait Show<T>` + `implements Show<Number>`; `trait Show<T> { fn show() { return 42 } }` + `implements Show<Number>` inject; Point `fn show()` vinner över Show<T>-default ([ROADMAP T5](ROADMAP.md)/[GENERICS.md T2](GENERICS.md)) |
| 10g | Associated types på klass | ✅ host; self-host parse+emit; **kab-only** `type Item = Number` / `class_assoc_types` | `type Item = Number` i `implements Iter` ([ROADMAP T3](ROADMAP.md)) |
| 10h | `where T: Trait` | ✅ host fn/metod; self-host emit på generiska fn, metoder **och klasser**; **kab-only** `class Box<T> where T: Show` / `Box<Shown>()` / `Box$Shown` / `fn show_it<T> where T: Show` / `show_it<Shown>` / `show_it$Shown` / `Box().show_it<Shown>` / **`show_it<Nope>`** reject / **`Box().show_it<Nope>`** reject / **`Box<Nope>`** reject / **`where T: Show, T: Named`** / `both_it$Shown` / **`both_it<OnlyShow>`** reject / **`where A: Show, B: Named`** / `pair_it$Shown_Labeled` / **`pair_it<Shown, Nope>`** reject / **`PairBox<Shown, Labeled>`** / `PairBox$Shown_Labeled` / **`PairBox<Shown, Nope>`** reject / **`Box().join_ab<Shown, Labeled>`** / `join_ab$Shown_Labeled` / **`Box().join_ab<Shown, Nope>`** reject / **`Box().both_it<Shown>`** / **`Box().both_it<OnlyShow>`** reject / **`BothBox<Shown>`** / `BothBox$Shown` / **`BothBox<OnlyShow>`** reject / **`WBox<Shown>`** / `WBox$Shown` / **`WBox<Nope>`** reject | `class Box<T> where T: Show` ([ROADMAP T1](ROADMAP.md)); `show_it<Nope>` / `Box().show_it<Nope>` / `Box<Nope>()` → `does not implement`; `both_it<Shown>` med två bounds; `both_it<OnlyShow>` saknar `Named`; `pair_it<Shown, Labeled>`; `pair_it<Shown, Nope>` / `PairBox<Shown, Nope>` / `Box().join_ab<Shown, Nope>` saknar `Named` på B; `class PairBox<A, B> where A: Show, B: Named`; `Box().join_ab<Shown, Labeled>`; **`Box().both_it<Shown>`** / **`Box().both_it<OnlyShow>`** med två bounds på metod; `class BothBox<T> where T: Show, T: Named` → `BothBox$Shown`; `BothBox<OnlyShow>` saknar `Named`; `struct WBox<T> where T: Show` → `WBox$Shown`; `WBox<Nope>` → `does not implement` |
| 10j | Generisk metod på `Box<T>` | ✅ host; self-host emit; **kab-only** `b.echo(1)` / `h.echo("x")` / `echo$Number` / `echo$String` | `let b = Box(42); b.echo(1)` → `echo$Number`; `h.echo("x")` → `echo$String` ([GENERICS.md G8.1](GENERICS.md)) |
| 10k | Generisk enum `Option<T>` / `Result<T,E>` | ✅ host Option; self-host ctor, **två typparametrar**, `match`; **kab-only** `Option.Some(n)` / `Option.Some("x")` / `Option<Number>.None` / `Result.Ok(n)` / `Result<Number, String>.Err` | `Option.Some(42)` → `Option$Number`; `Option.Some("x")` → `Option$String`; `Result<Number, String>.Ok(42)` → `Result$Number_String` ([GENERICS.md G9](GENERICS.md)) |
| 10l | Generiskt klassarv | ✅ host; self-host emit; **kab-only** `Child<Number>().tag()` / `Child$Number` extends `Base$Number` / `Child().tag()` / `super.init` / `Child(42).val` / `super.count = 1` / `super.n += 2` / **`super.n ||= `** / `let m = super.tag; m()` / `this.run(super.f)` / `(super.f)()` | `class Child<T> extends Base<T>` → `Child$Number` extends `Base$Number`; **`super.tag()`** / **`super.init(...)`** → `get_super_method`; **`super.count = 1`** / **`super.n += 2`**; **`let m = super.tag; m()`**; **`this.run(super.f)`**; **`(super.f)(4)`** |
| 10m | Explicit type-args + två specs | ✅ host; self-host emit; **kab-only** `h.echo(1)` + `h.echo("x")` / `echo$String` / `Box<String>("hi")` / `id<Number>(42)` / `id("hi")` / `id$String` / `id(id(42))` / `pair$Number_String` / `id(b)` / `id$Box` / `pair(x, s)` / `len(wrap(1))` | `id<Number>(42)`; `id(id(42))`; `pair$Number_String`; **`len(pair(x, s))`** / **`len(wrap(1))`** → `get_length`; `Box<String>("hi")`; `h.echo(1)` + `h.echo("x")`; `id(b)` → `id$Box` |
| 10n | Logical assign + `??` | ✅ host; self-host lexer/parse/emit; **kab-only default** | `a \|\|= 5`; `b &&= 9`; `c ??= 3`; **`o.x \|\|= 5`** / **`o.x &&= 9`** / **`o.x ??= 3`** / **`xs[0] \|\|= 5`** / **`xs[0] ??= 3`** / **`o.a.b ??= 4`** / **`o.items[0] \|\|= 5`** / **`xs[0].x ??= 3`** / **`xs[0][0] \|\|= 7`** / **`this.n \|\|= 5`** / **`o.items[0][0] \|\|= 8`** / **`xs[0][0].x ??= 6`** / **`super.n \|\|= 9`** / **`o.items[0][0].x ??= 4`** / **`xs[0][0][0] \|\|= 8`** / **`Child<T> super.n \|\|=`**; `null ?? 4` → `jump_if_not_nullish` |
| 10o | Optional chaining | ✅ host; self-host parse+emit; **kab-only default** | `o?.x` / `xs?.[0]` via `__opt_member` / `__opt_index`; `f?.()` via `jump_if_not_nullish` + `call` |
| 10p | Ternary `? :` | ✅ host; self-host parse+emit; **kab-only default** | `n > 3 ? 10 : 0`; nästlad `true ? false ? 1 : 2 : 3` |
| 10q | Result `?` | ✅ host; self-host parse+emit; **kab-only default** | `step()?` unwrap `Ok`; `bad()?` behåller `Err` (`match` → inner) → `result_question` |
| 10r | `switch` | ✅ host; self-host parse+emit + explicit **`fallthrough`**; **kab-only default** (match + default + fallthrough) | `switch (n) { case 2: { …; fallthrough } case 3: { … } }` |
| 10s | `do`/`while` | ✅ host; self-host parse+emit; **kab-only default** | `do { n = n + 1 } while (false)` kör kroppen minst en gång |
| 10t | Index/member `+=` | ✅ host; self-host parse+emit; **kab-only default** | `xs[0] += 3` via `iatmp`/`index_set`; **`o.x += 3`** / **`o.a.b += 4`** via `matmp`/`member_set`; **`o.items[0] +=`** / **`o.items[0][0] +=`** store-back via `member_set` after Index-kedja; **`xs[0].x +=`** / **`xs[0][0].x +=`** / **`o.items[0][0].x +=`**; **`xs[0][0] +=`** / **`xs[0][0][0] +=`** Index-kedja store-back (`iaWalk`); **`n &= 3`** / **`n |= 2`** / **`n ^= 3`** / **`o.x &= 3`** / **`xs[0] |= 2`** / **`o.x ^= 3`** / **`this.n &= 3`** / **`xs[0] ^= 3`** / **`super.n |= 2`** / **`o.x |= 2`** / **`xs[0] &= 3`** / **`this.n |= 2`** / **`this.n ^= 3`** / **`super.n &= 3`** / **`super.n ^= 3`** / **`o.a.b &= 3`** / **`o.a.b |= 2`** / **`o.a.b ^= 3`** / **`xs[0].x &= 3`** / **`xs[0].x |= 2`** / **`xs[0].x ^= 3`** / **`o.items[0] &= 3`** / **`o.items[0] |= 2`** / **`o.items[0] ^= 3`** / **`o.items[0][0] &= 3`** / **`o.items[0][0] |= 2`** / **`o.items[0][0] ^= 3`** / **`xs[0][0].x &= 3`** / **`xs[0][0].x |= 2`** / **`xs[0][0].x ^= 3`** / **`xs[0][0] &= 3`** / **`xs[0][0] |= 2`** / **`xs[0][0] ^= 3`** / **`o.items[0][0].x &= 3`** / **`o.items[0][0].x |= 2`**; **`o.x \|\|=`** / **`o.x &&=`** / **`o.x ??=`** / **`xs[0] \|\|=`** / **`xs[0] ??=`** / **`o.a.b ??=`** / **`o.items[0] \|\|=`** / **`xs[0].x ??=`** / **`xs[0][0] \|\|=`** / **`this.n \|\|=`** / **`o.items[0][0] \|\|=`** / **`xs[0][0].x ??=`** / **`super.n \|\|=`** / **`o.items[0][0].x ??=`** / **`xs[0][0][0] \|\|=`** / **`Child<T> super.n \|\|=`** |
| 10u | Template literals | ✅ host; self-host lexer/parse+emit; **kab-only default** | `` `n=${n}` `` desugaras till sträng-`+` |
| 10v | `is` / `instanceof` | ✅ host; self-host parse+emit; **kab-only default** | `is(obj, "Class")` → `instanceof` CALL; Kab-VM `vInstanceofS` på `vmC` + `extends` |
| 10w | Python-lån (`pass`/`raise`/`assert`/`not`) | ✅ host eval; self-host parse+emit; **kab-only default** | `pass`; `assert cond, msg`; `not x` → `!` / `OP_NOT`; `raise e` / `throw e` + `try`/`catch` (`fn_try_region`; densify-fix `bodyStart`) |
| 10x | `with` + `is`/`is not` | ✅ host eval; self-host parse+emit; **`is`/`is not` kab-only**; **`with` inte Kab-VM-proven** | `a is b` → `object_is` CALL; `with rsrc as r { }` emit `emitDisposeName` (`close?.()`). Kab-VM: `close` anropas men `this.n` / `store_global` syns inte i anroparen (lastThis/trampolin-COW) |
| 10y | `using` | ✅ host eval; self-host parse+emit | `using x = r` i `{ }` (pEnterBody-block, inte tom pStmts-push) → `dispose`/`close` vid block-slut. rust-VM proven. Kab-VM: samma vägg som `with` — anrop ja, sidoeffekt nej (inte ny trampolin/writeback/native) |
| 10z | `import.meta` / `import()` | ✅ host eval; self-host parse+emit; **kab-only default** för **`import.meta`** | `import.meta.url` / `.path` → `import_meta()`; `import("math")` → `dynamic_import` (inte Kab-VM-proven) |
| 10aa | `delete` | ✅ host eval; self-host parse+emit; **kab-only default** | `delete o.z` → `object_delete_prop` + store-back på var; **`delete o.a.b`** / **`delete xs[0].x`** snapshot store-back (`member_set`/`index_set`); **`delete o[k]`** / **`delete o.items[0].x`** / **`delete xs[0][0].x`** / **`delete this.z`** / **`delete o.items[0][0].x`** / **`delete o.a.b.c`** / **`delete this.a.b`** / **`delete o[k].x`** / **`delete super.z`** / **`delete this[k]`** / **`delete super[k]`** / **`delete super.a.b`** / **`delete o[k][j]`** / **`delete super.a[k]`** / **`delete this[k].x`** / **`delete super[k].x`** / **`delete this.a[k]`** / **`delete o.a[k]`** / **`delete this[k][j]`** / **`delete super[k][j]`** / **`delete o.items[0][k]`** / **`delete this.a.b[k]`** / **`delete o.a.b[k]`** / **`delete xs[0][0][k]`** / **`delete super.a.b[k]`** / **`delete this.items[0][k]`** / **`delete super.items[0][k]`** / **`delete this.items[0][0][k]`** / **`delete o.items[0][0][k]`** / **`delete super.items[0][0][k]`** (rust `try_compile` vägrar `delete`) |
| 10ab | Klassisk `for` / `for-of` / `for-in` | ✅ host eval; self-host parse+emit; **kab-only default** | `for let i = 0; i < n; i = i + 1 { }` (även `for (let …)`); **`for x of xs`** via `iterator_begin` + `iterator_step_in_place`; **`for k in obj`** nycklar / **`for i in xs`** index via `JUMP_UNLESS_OBJECT` + `keys` |
| 11 | Effect system | 🔶 | `@pure` `@io` `@disk` (strippas) |
| 12 | Benchmark | 🔶 | `lang_benchmark`, `@benchmark` |
| 13 | Doc-exempel | 🔶 | `@example` planerat |
| 14 | Kanaler | ✅ | `channel_new/send/recv` |
| 15 | Cache-layout | 🔶 | `@packed` (dokumentation) |
| 16 | Post-quantum | 🔶 | `crypto_kyber_encapsulate`, `crypto_dilithium_sign` |
| 17 | Persistens | 🔶 | `@persist`, `persist_save/load` |
| 18 | GPU/shader | 🔶 | `shader_compile`, `webgl_*` |
| 19 | Resumable fel | 🔶 | `try/catch` returnerar resume-värde |
| 20 | Självhostande | 🔶 | Produktkompilator i `self_host/` (`.kab`); Rust-host skuld tills SH28 |

```kabootar
lang_info();                                    // alla 20
html! { <main>Hello Kabootar</main> };          // Kv8 UI
let ch = channel_new(8); channel_send(ch, 1);   // kanaler
actor Service { };                              // aktör + mailbox
comptime_assert(true, "arch ok");               // comptime-check
lang_benchmark("work", 1000, my_fn);            // inbyggd bench
persist_save("/data/cfg.json", { port: 8080 }); // VFS-persist
shader_compile("frag", "void main(){}");        // SPIR-V-stub
```

### Systems memory (`@manual`) — Våg O

Default modules stay **GC** (web/Kv8/UI). System/kOS files opt in with `@manual` and get **compile-time ownership** plus runtime MemBox.

```kabootar
@version "1.0.0"
@manual

fn peek(buf: &Owned) {
    return owned_read(buf, 0, 4)
}

fn take(buf: Owned) {
    drop(buf)
}

let buf = owned_alloc(64, "frame")
owned_write(buf, 0, [1, 2, 3])
let bytes = peek(&buf)       // shared borrow — buf lives
let other = owned_move(buf)  // move — buf dead after this
// owned_read(buf, …)        // compile error: use after move
take(other)
```

| Regel | Betydelse |
|-------|-----------|
| **GC default** | Ingen ownership-check; `owned_*` förbjudet utan `@manual` |
| **Affine Owned** | `let y = x`, call-arg, `owned_move`/`move`/`drop` flyttar; use-after-move = compile error |
| **Peek-API** | `owned_read` / `owned_write` (och `kos/mem` read/write) flyttar inte |
| **`&` / `&mut`** | Shared vs exclusive borrow; borrow lever under call-uttrycket |
| **Runtime** | `Value::Owned` + use-after-move som säkerhetsnät |

Se [OWNERSHIP.md](OWNERSHIP.md). **Inte** Rust-lifetimes.

- kOS helper: `import "kos/display_buf"` — `@manual` framebuffer over `kos/mem`.