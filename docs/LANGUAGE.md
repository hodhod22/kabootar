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
| 10l | Generiskt klassarv | ✅ host; self-host emit; **kab-only** `Child<Number>().tag()` / `Child$Number` extends `Base$Number` / `Child().tag()` / `super.init` / `Child(42).val` / `super.count = 1` / `super.n += 2` / **`super.n ||= `** / `let m = super.tag; m()` / **`Child<T>` `let m = super.tag`** / `this.run(super.f)` / `(super.f)()` | `class Child<T> extends Base<T>` → `Child$Number` extends `Base$Number`; **`super.tag()`** / **`super.init(...)`** → `get_super_method`; **`let m = super.tag; m()`** även på **`Child<T>`** (`sh6_self_host_generic_super_bound_tag_ok`) |
| 10m | Explicit type-args + två specs | ✅ host; self-host emit; **kab-only** `h.echo(1)` + `h.echo("x")` / `echo$String` / `Box<String>("hi")` / `id<Number>(42)` / `id("hi")` / `id$String` / `id(id(42))` / `pair$Number_String` / `id(b)` / `id$Box` / `pair(x, s)` / `len(wrap(1))` | `id<Number>(42)`; `id(id(42))`; `pair$Number_String`; **`len(pair(x, s))`** / **`len(wrap(1))`** → `get_length`; `Box<String>("hi")`; `h.echo(1)` + `h.echo("x")`; `id(b)` → `id$Box` |
| 10n | Logical assign + `??` | ✅ host; self-host lexer/parse/emit; **kab-only default** | `a \|\|= 5`; `b &&= 9`; `c ??= 3`; **`o.x \|\|= 5`** / **`o.x &&= 9`** / **`o.x ??= 3`** / **`xs[0] \|\|= 5`** / **`xs[0] ??= 3`** / **`o.a.b ??= 4`** / **`o.items[0] \|\|= 5`** / **`xs[0].x ??= 3`** / **`xs[0][0] \|\|= 7`** / **`this.n \|\|= 5`** / **`o.items[0][0] \|\|= 8`** / **`xs[0][0].x ??= 6`** / **`super.n \|\|= 9`** / **`o.items[0][0].x ??= 4`** / **`xs[0][0][0] \|\|= 8`** / **`Child<T> super.n \|\|=`**; `null ?? 4` → `jump_if_not_nullish` |
| 10o | Optional chaining | ✅ host; self-host parse+emit; **kab-only default** | `o?.x` / `xs?.[0]` via `__opt_member` / `__opt_index`; `f?.()` via `jump_if_not_nullish` + `call` |
| 10p | Ternary `? :` | ✅ host; self-host parse+emit; **kab-only default** | `n > 3 ? 10 : 0`; nästlad `true ? false ? 1 : 2 : 3` |
| 10q | Result `?` | ✅ host; self-host parse+emit; **kab-only default** | `step()?` unwrap `Ok`; `bad()?` behåller `Err` (`match` → inner) → `result_question` |
| 10r | `switch` | ✅ host; self-host parse+emit + explicit **`fallthrough`**; **kab-only default** (match + default + fallthrough) | `switch (n) { case 2: { …; fallthrough } case 3: { … } }` |
| 10s | `do`/`while` | ✅ host; self-host parse+emit; **kab-only default** | `do { n = n + 1 } while (false)` kör kroppen minst en gång |
| 10t | Index/member `+=` | ✅ host; self-host parse+emit; **kab-only default** | `xs[0] += 3` via `iatmp`/`index_set`; **`o.x += 3`** / **`o.a.b += 4`** via `matmp`/`member_set`; **`o.items[0] +=`** / **`o.items[0][0] +=`** store-back via `member_set` after Index-kedja; **`xs[0].x +=`** / **`xs[0][0].x +=`** / **`o.items[0][0].x +=`**; **`xs[0][0] +=`** / **`xs[0][0][0] +=`** Index-kedja store-back (`iaWalk`); **`n &= 3`** / **`n <<= 1`** / **`n >>= 1`** / **`n >>>= 1`** / **`o.x <<= 1`** / **`o.x >>= 1`** / **`o.x >>>= 1`** / **`xs[0] <<= 1`** / **`xs[0] >>= 1`** / **`xs[0] >>>= 1`** / **`this.n <<= 1`** / **`this.n >>= 1`** / **`this.n >>>= 1`** / **`super.n <<= 1`** / **`super.n >>= 1`** / **`super.n >>>= 1`** / **`o.a.b <<= 1`** / **`o.a.b >>= 1`** / **`o.a.b >>>= 1`** / **`xs[0].x <<= 1`** / **`xs[0].x >>= 1`** / **`xs[0].x >>>= 1`** / **`o.items[0] <<= 1`** / **`o.items[0] >>= 1`** / **`o.items[0] >>>= 1`** / **`o.items[0][0] <<= 1`** / **`o.items[0][0] >>= 1`** / **`o.items[0][0] >>>= 1`** / **`xs[0][0].x <<= 1`** / **`xs[0][0].x >>= 1`** / **`xs[0][0].x >>>= 1`** / **`o.items[0][0].x <<= 1`** / **`o.items[0][0].x >>= 1`** / **`o.items[0][0].x >>>= 1`** / **`xs[0][0] <<= 1`** / **`xs[0][0] >>= 1`** / **`xs[0][0] >>>= 1`** / **`xs[0][0][0] <<= 1`** / **`xs[0][0][0] >>= 1`** / **`xs[0][0][0] >>>= 1`** / **`n **= 2`** / **`o.x **= 2`** / **`xs[0] **= 2`** / **`this.n **= 2`** / **`super.n **= 2`** / **`o.a.b **= 2`** / **`xs[0].x **= 2`** / **`o.items[0] **= 2`** / **`o.items[0][0] **= 2`** / **`xs[0][0].x **= 2`** / **`o.items[0][0].x **= 2`** / **`xs[0][0] **= 2`** / **`xs[0][0][0] **= 2`** / **`n %= 7`** / **`o.x %= 7`** / **`xs[0] %= 7`** / **`this.n %= 7`** / **`super.n %= 7`** / **`o.a.b %= 7`** / **`xs[0].x %= 7`** / **`o.items[0] %= 7`** / **`o.items[0][0] %= 7`** / **`xs[0][0].x %= 7`** / **`o.items[0][0].x %= 7`** / **`xs[0][0] %= 7`** / **`xs[0][0][0] %= 7`** / **`n -= 2`** / **`o.x -= 2`** / **`xs[0] -= 2`** / **`this.n -= 2`** / **`super.n -= 2`** / **`o.a.b -= 2`** / **`xs[0].x -= 2`** / **`o.items[0] -= 2`** / **`o.items[0][0] -= 2`** / **`xs[0][0].x -= 2`** / **`o.items[0][0].x -= 2`** / **`xs[0][0] -= 2`** / **`xs[0][0][0] -= 2`** / **`n *= 3`** / **`n |= 2`** / **`n ^= 3`** / **`o.x &= 3`** / **`xs[0] |= 2`** / **`o.x ^= 3`** / **`this.n &= 3`** / **`xs[0] ^= 3`** / **`super.n |= 2`** / **`o.x |= 2`** / **`xs[0] &= 3`** / **`this.n |= 2`** / **`this.n ^= 3`** / **`super.n &= 3`** / **`super.n ^= 3`** / **`o.a.b &= 3`** / **`o.a.b |= 2`** / **`o.a.b ^= 3`** / **`xs[0].x &= 3`** / **`xs[0].x |= 2`** / **`xs[0].x ^= 3`** / **`o.items[0] &= 3`** / **`o.items[0] |= 2`** / **`o.items[0] ^= 3`** / **`o.items[0][0] &= 3`** / **`o.items[0][0] |= 2`** / **`o.items[0][0] ^= 3`** / **`xs[0][0].x &= 3`** / **`xs[0][0].x |= 2`** / **`xs[0][0].x ^= 3`** / **`xs[0][0] &= 3`** / **`xs[0][0] |= 2`** / **`xs[0][0] ^= 3`** / **`o.items[0][0].x &= 3`** / **`o.items[0][0].x |= 2`** / **`o.items[0][0].x ^= 3`** / **`xs[0][0][0] &= 3`** / **`xs[0][0][0] |= 2`** / **`xs[0][0][0] ^= 3`**; **`o.x \|\|=`** / **`o.x &&=`** / **`o.x ??=`** / **`xs[0] \|\|=`** / **`xs[0] ??=`** / **`o.a.b ??=`** / **`o.items[0] \|\|=`** / **`xs[0].x ??=`** / **`xs[0][0] \|\|=`** / **`this.n \|\|=`** / **`o.items[0][0] \|\|=`** / **`xs[0][0].x ??=`** / **`super.n \|\|=`** / **`o.items[0][0].x ??=`** / **`xs[0][0][0] \|\|=`** / **`Child<T> super.n \|\|=`** |
| 10u | Template literals | ✅ host; self-host lexer/parse+emit; **kab-only default** | `` `n=${n}` `` desugaras till sträng-`+` |
| 10v | `is` / `instanceof` | ✅ host; self-host parse+emit; **kab-only default** | `is(obj, "Class")` → `instanceof` CALL; Kab-VM `vInstanceofS` på `vmC` + `extends` |
| 10w | Python-lån (`pass`/`raise`/`assert`/`not`) | ✅ host eval; self-host parse+emit; **kab-only default** | `pass`; `assert cond, msg`; `not x` → `!` / `OP_NOT`; `raise e` / `throw e` + `try`/`catch` (`fn_try_region`; densify-fix `bodyStart`) |
| 10x | `with` + `is`/`is not` | ✅ host eval; self-host parse+emit; **`is`/`is not` kab-only**; **`with` close på klass** samma instans-heap som `using` (inte eget SH6-test) | `a is b` → `object_is` CALL; `with rsrc as r { }` emit `emitDisposeName` (`close?.()`) |
| 10y | `using` | ✅ host eval; self-host parse+emit; **Kab-VM class `close()` writeback** | `using x = r` i `{ }` → `dispose`/`close` vid block-slut. rust-VM + Kab-VM: `sh6_self_host_using_class_close_writeback_ok` (`x.n` syns) |
| 10z | `import.meta` / `import()` | ✅ host eval; self-host parse+emit; **kab-only** `import.meta` + **`await import("math")`** | `import.meta.url` / `.path` → `import_meta()`; `import("math")` → `dynamic_import` + `await` (`sh6_self_host_dynamic_import_math_ok`) |
| 10aa | `delete` | ✅ host eval; self-host parse+emit; **kab-only default** | `delete o.z` → `object_delete_prop` + store-back på var; **`delete o.a.b`** / **`delete xs[0].x`** snapshot store-back (`member_set`/`index_set`); **`delete o[k]`** / **`delete o.items[0].x`** / **`delete xs[0][0].x`** / **`delete this.z`** / **`delete o.items[0][0].x`** / **`delete o.a.b.c`** / **`delete this.a.b`** / **`delete o[k].x`** / **`delete super.z`** / **`delete this[k]`** / **`delete super[k]`** / **`delete super.a.b`** / **`delete o[k][j]`** / **`delete super.a[k]`** / **`delete this[k].x`** / **`delete super[k].x`** / **`delete this.a[k]`** / **`delete o.a[k]`** / **`delete this[k][j]`** / **`delete super[k][j]`** / **`delete o.items[0][k]`** / **`delete this.a.b[k]`** / **`delete o.a.b[k]`** / **`delete xs[0][0][k]`** / **`delete super.a.b[k]`** / **`delete this.items[0][k]`** / **`delete super.items[0][k]`** / **`delete this.items[0][0][k]`** / **`delete o.items[0][0][k]`** / **`delete super.items[0][0][k]`** (rust `try_compile` vägrar `delete`) |
| 10ab | Klassisk `for` / `for-of` / `for-in` | ✅ host eval; self-host parse+emit; **kab-only default** | `for let i = 0; i < n; i = i + 1 { }` (även `for (let …)`); **`for x of xs`** via `iterator_begin` + `iterator_step_in_place`; **`for k in obj`** nycklar / **`for i in xs`** index via `JUMP_UNLESS_OBJECT` + `keys` |
| 10ac | `async fn` | ✅ host eval; self-host parse+emit `fn_async`; **kab-only** sync-kropp + `promise_resolve` | `async fn add(a, b) { return a + b }; await add(2, 3)` → **5** (`sh6_self_host_async_fn_ok`). Inte rust `schedule_bytecode_async` på Kab-VM. Inte `async =>`. |
| 10ad | `for await` | ✅ host eval; self-host parse+emit; **kab-only** array-iterable | `for await x of [1, 2, 3]` i `async fn` → `async_iterator_begin` + `async_iterator_step_in_place` + `await` (`sh6_self_host_for_await_array_ok`). |
| 10ae | `fn*` / `yield` | ✅ host eval; self-host parse+emit `fn_generator` + `yield`; **kab-only** `.next()` | `fn* gen() { yield 1; yield 2 }; g.next().value` (`sh6_self_host_generator_yield_ok`). Inte `async fn*` i samma vägg. |
| 10af | `yield*` | ✅ host eval; self-host desugar till `iterator_begin` + `yield`; **kab-only** array + `fn*` | `fn* gen() { yield* [1, 2] }` → **3** (`sh6_self_host_yield_star_array_ok`); `yield* inner()` → **3** (`sh6_self_host_yield_star_generator_ok`). |
| 10ag | `for x of gen()` | ✅ self-host + **kab-only** | `iterator_begin(vmGen)` identitet; `iterator_step_in_place` resume (`sh6_self_host_for_of_generator_ok`); **`break`/`return`/`throw`** → `iterator_close`; **`continue`** hoppar till nästa steg (`sh6_self_host_for_of_generator_break_ok`, `sh6_self_host_for_of_generator_return_ok`, `sh6_self_host_for_of_generator_throw_ok`, `sh6_self_host_for_of_generator_continue_ok`). Inte `async fn*`. |
| 10ah | `fn*` `return` | ✅ host eval; self-host + **kab-only** completion `.next()` | `yield 10; return 99` → **109** (`sh6_self_host_generator_return_ok`). |
| 10ai | `g.return(v)` | ✅ host eval; **kab-only** stäng utan resume | `g.next(); g.return(99)` → **99** (`sh6_self_host_generator_method_return_ok`). Inte `finally`. |
| 10aj | `g.throw(e)` | ✅ host eval; **kab-only** resume in i `catch` runt `yield` | `g.throw(99)` → **990** (`sh6_self_host_generator_throw_catch_ok`). Utan catch: close med värdet. Inte `finally`. Inte `async fn*`. |
| 10ak | `g.next(v)` send | ✅ host eval; **kab-only** yield-resultat | Första `.next()` ignorerar arg. Därefter `g.next(42)` → värdet av `let x = yield 1` (`sh6_self_host_generator_next_send_ok`). Inte `async fn*`. |
| 10al | `yield*` send | ✅ host eval; **kab-only** vidare till inner `fn*` | `yield* inner()` + `g.next(42)` → inner `let x = yield 10` (`sh6_self_host_yield_star_send_ok`). Inte `async fn*`. |
| 10am | `yield*` throw/return | ✅ host eval; **kab-only** vidare till inner `fn*` | `g.throw(77)` resume inner `catch` (`sh6_self_host_yield_star_throw_ok`); `g.return(42)` stänger inner (`sh6_self_host_yield_star_return_ok`). Inte `finally`. Inte `async fn*`. |
| 10an | `let x = yield*` | ✅ host eval; self-host emit completion-värde; **kab-only** | `let x = yield* inner()` efter inner `return 99` (`sh6_self_host_yield_star_expr_ok`). Inte `finally`. Inte `async fn*`. |
| 10ao | `yield*` throw → outer `catch` | ✅ **kab-only** | Inner utan `catch`: `g.throw(77)` resume:ar outer `catch` runt `yield*` (`sh6_self_host_yield_star_throw_outer_ok`). Inte `finally`. Inte `async fn*`. |
| 10ap | nästlad `yield*` throw | ✅ **kab-only** | `yield* mid()` + `yield* inner()` + outer `catch`: `g.throw(77)` → **77** (`sh6_self_host_yield_star_nested_throw_ok`). Inte `finally`. Inte `async fn*`. |
| 10aq | nästlad `yield*` send | ✅ **kab-only** | `yield* mid()` + `yield* inner()` + `g.next(42)` → inner `let x = yield 10` (`sh6_self_host_yield_star_nested_send_ok`). Inte `async fn*`. |
| 10ar | nästlad `yield*` return | ✅ **kab-only** | `g.return(42)` stänger inner och mid (`sh6_self_host_yield_star_nested_return_ok`). Inte `finally`. Inte `async fn*`. |
| 10as | nästlad `let x = yield*` | ✅ **kab-only** | `mid` `return` av `yield* inner()` `99`; outer `let x = yield* mid()` (`sh6_self_host_yield_star_nested_expr_ok`). Inte `finally`. Inte `async fn*`. |
| 10at | `yield*` array throw → outer `catch` | ✅ **kab-only** | Array-iterator saknar throw: `yield* [1, 2]` + `g.throw(77)` resume:ar outer `catch` (`sh6_self_host_yield_star_array_throw_ok`). Inte `finally`. Inte `async fn*`. |
| 10au | `yield*` array `g.return` | ✅ **kab-only** | `yield* [1, 2]` + `g.return(42)` stänger gen och array-iterator (`sh6_self_host_yield_star_array_return_ok`). Inte `finally`. Inte `async fn*`. |
| 10av | `yield*` `{ next() }` | ✅ **kab-only** | Custom iterator `next()` → `{ value: 3, done: false }` (`sh6_self_host_yield_star_custom_next_ok`). Inte `async fn*`. |
| 10aw | `yield*` custom `return` | ✅ **kab-only** | `g.return` anropar iteratorns `return` (`sh6_self_host_yield_star_custom_return_ok`). Inte `finally`. Inte `async fn*`. |
| 10ax | `yield*` custom `throw` | ✅ **kab-only** | `g.throw(77)` anropar iteratorns `throw` (`sh6_self_host_yield_star_custom_throw_ok`). Inte `finally`. Inte `async fn*`. |
| 10ay | `yield*` custom send | ✅ **kab-only** | `g.next(42)` anropar iteratorns `next(v)` (`sh6_self_host_yield_star_custom_send_ok`). Inte `finally`. Inte `async fn*`. |
| 10az | `yield*` custom `return` `done: true` | ✅ **kab-only** | `g.return` completion via `let x = yield* it; return x` (`sh6_self_host_yield_star_custom_return_done_ok` → **99**). Inte `finally`. Inte `async fn*`. |
| 10ba | `yield*` `Symbol.iterator` | ✅ **kab-only** | Objekt med `Symbol.iterator`-fabrik som returnerar `{ next }` (`sh6_self_host_yield_star_symbol_iterator_ok`). Inte `async fn*`. |
| 10bb | `for of` `Symbol.iterator` | ✅ **kab-only** | Fabrik returnerar array `[1, 2, 3]` → **6** (`sh6_self_host_for_of_symbol_iterator_ok`). Inte `async fn*`. |
| 10bc | `for of` `{ next }` | ✅ **kab-only** | `{ next: itNext }` utan `Symbol.iterator` → **7** (`sh6_self_host_for_of_custom_next_ok`). Inte `async fn*`. |
| 10bd | `o[Symbol.iterator]` | ✅ **kab-only** | Well-known symbol-nyckel, inte strängen `"Symbol.iterator"` (`sh6_self_host_for_of_symbol_iterator_wellknown_ok`). Inte `async fn*`. |
| 10be | `{ next() {} }` + `fn*` | ✅ **kab-only** | Metod-shorthand och generator i samma program (`sh6_self_host_obj_method_and_generator_ok`). Inte `async fn*`. |
| 10bf | `{ next() {} }` `this` | ✅ **kab-only** | Iterator-`next` läser `this` (`sh6_self_host_obj_method_iter_this_ok`). Inte `for of` writeback. Inte `async fn*`. |
| 10bg | `for of` `{ next() {} }` `this` | ✅ **kab-only** | `for of` + metod-`this` writeback (`sh6_self_host_for_of_obj_method_this_ok` → **5**). Inte nästlad metod i `fn`. Inte `async fn*`. |
| 10bh | nästlad `{ next() {} }` i `fn` | ✅ **kab-only** | Metod i `fn run` + yttre locals via caps + `this` (`sh6_self_host_for_of_nested_obj_method_ok` → **5**). Inte `async fn*`. |
| 10bi | nästlad `{ next() {} }` i `fn*` | ✅ **kab-only** | `yield*` av metod inuti generator (`sh6_self_host_yield_star_nested_obj_method_ok` → **5**). Inte `async fn*`. |
| 10bj | nästlad `"return"()` `this` | ✅ **kab-only** | `g.return` efter `yield*` anropar iterator-`return` med `this` (`sh6_self_host_yield_star_nested_return_this_ok` → **9**). Inte `finally`. Inte `async fn*`. |
| 10bk | nästlad `"throw"()` `this` | ✅ **kab-only** | `g.throw` efter `yield*` anropar iterator-`throw` med `this` (`sh6_self_host_yield_star_nested_throw_this_ok` → **9**). Inte `done: true` som `yield*`-completion. Inte `finally`. Inte `async fn*`. |
| 10bl | `throw` `done: true` | ✅ **kab-only** | Iterator-`throw` `{ done: true }` completion (`sh6_self_host_yield_star_throw_done_ok` → **9**). Inte `finally`. Inte `async fn*`. |
| 10bm | `throw` `done: true` + `yield` | ✅ **kab-only** | Efter `yield*` körs följande `yield 4` (`sh6_self_host_yield_star_throw_done_next_ok` → **4**). Inte `finally`. Inte `async fn*`. |
| 10bn | `return` `done: true` + `yield` | ✅ **kab-only** | Efter `yield*` körs följande `yield 4` (`sh6_self_host_yield_star_return_done_next_ok` → **4**). Inte `finally`. Inte `async fn*`. |
| 10bo | `try/finally` i `fn*` | ✅ **kab-only** | Efter `yield` i `try` körs `finally` (`sh6_self_host_generator_try_finally_ok` → **9**). Inte `async fn*`. |
| 10bp | `g.return` kör `finally` | ✅ **kab-only** | `g.return` efter `yield` i `try` kör `finally` (`sh6_self_host_generator_return_finally_ok` → **9**). Inte `async fn*`. |
| 10bq | `g.throw` kör `finally` | ✅ **kab-only** | `g.throw` efter `yield` i `try` kör `finally` (`sh6_self_host_generator_throw_finally_ok` → **9**). Inte `async fn*`. |
| 10br | `try/finally` utan `catch` | ✅ **kab-only** | `try { yield } finally { … }` (`sh6_self_host_generator_try_finally_no_catch_ok` → **9**; `return`/`throw` → **9**). Inte `async fn*`. |
| 10bs | `g.throw` kör `catch` sedan `finally` | ✅ **kab-only** | `acc = e` i `catch`, `acc = acc + 10` i `finally`, `return acc` (`sh6_self_host_generator_throw_catch_finally_ok` → **17**). Inte `async fn*`. |
| 10bt | `g.throw` `member_set` i `catch` + `finally` | ✅ **kab-only** | `h.c = e` i `catch`, `h.n = 9` i `finally`, `return h.c * 10 + h.n` (`sh6_self_host_generator_throw_catch_member_finally_ok` → **79**). Inte `async fn*`. |
| 10bu | `g.throw` `yield` i `catch` + `finally` | ✅ **kab-only** | `yield e` i `catch` pausar; `g.next()` kör `yield` i `finally` (`sh6_self_host_generator_throw_catch_yield_finally_ok` → **79**). Inte `async fn*`. |
| 10bv | `g.next(v)` send in i `yield` i `catch` | ✅ **kab-only** | `let x = yield e` efter `g.throw`; `g.next(9)` sen `return` med `finally` (`sh6_self_host_generator_throw_catch_yield_send_finally_ok` → **19**). Inte `async fn*`. |
| 10bw | `yield*` i `catch` + `finally` | ✅ **kab-only** | `g.throw` sen `yield* [e]` i `catch`; `g.next()` kör `yield` i `finally` (`sh6_self_host_generator_throw_catch_yield_star_finally_ok` → **79**). Inte `async fn*`. |
| 10bx | `yield* inner()` i `catch` + `finally` | ✅ **kab-only** | `g.throw` sen `yield* inner()` i `catch`; `g.next()` kör `yield` i `finally` (`sh6_self_host_generator_throw_catch_yield_star_inner_finally_ok` → **89**). Inte `async fn*`. |
| 10by | send genom `yield* inner()` i `catch` | ✅ **kab-only** | `g.throw` sen `let y = yield* inner()`; `g.next(3)` med `finally` (`sh6_self_host_generator_throw_catch_yield_star_inner_send_finally_ok` → **13**). Inte `async fn*`. |
| 10bz | nästlad `yield* mid()`/`inner()` i `catch` | ✅ **kab-only** | `g.throw` sen `yield* mid()` där `mid` gör `yield* inner()`; `g.next()` kör `yield` i `finally` (`sh6_self_host_generator_throw_catch_yield_star_nested_finally_ok` → **89**). Inte `async fn*`. |
| 10ca | send genom nästlad `yield*` i `catch` | ✅ **kab-only** | `g.throw` sen `let z = yield* mid()` / `inner()`; `g.next(3)` med `finally` (`sh6_self_host_generator_throw_catch_yield_star_nested_send_finally_ok` → **13**). Inte `async fn*`. |
| 10cb | `yield` i `finally` efter nästlad send | ✅ **kab-only** | `g.throw` sen `g.next(3)` genom `yield* mid()`/`inner()`; `yield 9` i `finally` sen `return acc` (`sh6_self_host_generator_throw_catch_yield_star_nested_send_yield_finally_ok` → **893**). Inte `async fn*`. |
| 10cc | `return` i `catch` kör `finally`-`yield` | ✅ **kab-only** | `return z + 10` i `catch` efter nästlad `yield*`-send; `g.next(3)` ger `yield 9` i `finally` (`sh6_self_host_generator_throw_catch_yield_star_nested_send_return_finally_ok` → **89**). Inte `async fn*`. |
| 10cd | completion efter `finally`-`yield` | ✅ **kab-only** | tredje `.next()` efter `return` i `catch` ger **13** (`sh6_self_host_generator_throw_catch_yield_star_nested_send_return_finally_done_ok`). Inte `async fn*`. |
| 10ce | completion `done: true` | ✅ **kab-only** | samma kedja; `r.done == true` (`sh6_self_host_generator_throw_catch_yield_star_nested_send_return_finally_done_flag_ok` → **1**). Inte `async fn*`. |
| 10cf | `.next()` efter completion | ✅ **kab-only** | ytterligare `.next()` förblir `done: true` (`sh6_self_host_generator_throw_catch_yield_star_nested_send_return_finally_done_again_ok` → **1**). Inte `async fn*`. |
| 10cg | extra `.next()` `value: null` | ✅ **kab-only** | `.next()` efter completion har `value: null` (`sh6_self_host_generator_throw_catch_yield_star_nested_send_return_finally_done_null_ok` → **1**). Inte `async fn*`. |
| 10ch | `g.throw` efter completion | ✅ **kab-only** | `g.throw` efter completion förblir `done: true` (`sh6_self_host_generator_throw_catch_yield_star_nested_send_return_finally_throw_done_ok` → **1**). Inte `async fn*`. |
| 10ci | `g.return` efter completion | ✅ **kab-only** | `g.return` efter completion förblir `done: true` (`sh6_self_host_generator_throw_catch_yield_star_nested_send_return_finally_method_return_done_ok` → **1**). Inte `async fn*`. |
| 10cj | `g.return` efter completion `value: null` | ✅ **kab-only** | `g.return` efter completion har `value: null` (`sh6_self_host_generator_throw_catch_yield_star_nested_send_return_finally_method_return_null_ok` → **1**). Inte `async fn*`. |
| 10ck | `g.throw` efter completion `value: null` | ✅ **kab-only** | `g.throw` efter completion har `value: null` (`sh6_self_host_generator_throw_catch_yield_star_nested_send_return_finally_throw_null_ok` → **1**). Inte `async fn*`. |
| 10cl | send in i `finally`-`yield` efter `return` | ✅ **kab-only** | `let f = yield 9` i `finally` efter nästlad `return` i `catch`; `g.next(4)` ger **13** (`sh6_self_host_generator_throw_catch_yield_star_nested_send_return_finally_send_ok` → **903**). Inte `async fn*`. |
| 10cm | `return` i `finally` ersätter `catch`-`return` | ✅ **kab-only** | `return f` i `finally` efter send till `yield 9`; completion **4** istället för **13** (`sh6_self_host_generator_throw_catch_yield_star_nested_send_return_finally_override_ok` → **894**). Inte `async fn*`. |
| 10cn | `throw` i `finally` ersätter `catch`-`return` | ✅ **kab-only** | `throw f` i `finally` efter send till `yield 9`; anroparens `catch` får **4** istället för completion **13** (`sh6_self_host_generator_throw_catch_yield_star_nested_send_return_finally_throw_override_ok` → **894**). Inte `async fn*`. |
| 10co | `g.throw` in i `finally`-`yield` | ✅ **kab-only** | `g.throw` medan generatorn är pausad på `yield` i `finally` efter nästlad `return` i `catch` (`sh6_self_host_generator_throw_catch_yield_star_nested_send_return_finally_throw_into_yield_ok` → **894**). Inte `async fn*`. |
| 10cp | nästlad `try` + `g.throw` in i inner `finally`-`yield` | ✅ **kab-only** | inner `try/finally` `yield 9`; `g.throw(4)` fångas av yttre `catch` (`sh6_self_host_generator_nested_try_throw_into_finally_yield_ok` → **194**). Inte `async fn*`. |
| 10cq | nästlad `try` + `g.throw` in i inner `try` | ✅ **kab-only** | `g.throw` vid inner `yield 1` kör inner `finally` `yield 9`, sedan yttre `catch` (`sh6_self_host_generator_nested_try_throw_into_try_ok` → **194**). Inte `async fn*`. |
| 10cr | nästlad `try` + `g.return` in i inner `try` | ✅ **kab-only** | `g.return(4)` vid inner `yield 1` kör inner `finally` `yield 9`, sedan completion **4** (`sh6_self_host_generator_nested_try_return_into_try_ok` → **194**). Inte `async fn*`. |
| 10cs | nästlad `try` + `g.return` in i inner `finally`-`yield` | ✅ **kab-only** | `g.next(); g.next(); g.return(4)` vid inner `yield 9` → completion **4** (`sh6_self_host_generator_nested_try_return_into_finally_yield_ok` → **194**). Inte `async fn*`. |
| 10ct | nästlad `try` + yttre `finally` + `g.throw` | ✅ **kab-only** | `g.throw(4)` vid inner `yield 1` → inner `yield 9`, yttre `finally` `yield 8`, sedan **4** (`sh6_self_host_generator_nested_try_outer_finally_throw_ok` → **1984**). Inte `async fn*`. |
| 10cu | nästlad `try` + yttre `finally` + `g.return` | ✅ **kab-only** | `g.return(4)` vid inner `yield 1` → inner `yield 9`, yttre `finally` `yield 8`, sedan **4** (`sh6_self_host_generator_nested_try_outer_finally_return_ok` → **1984**). Inte `async fn*`. |
| 10cv | `g.return` vid yttre `finally`-`yield` | ✅ **kab-only** | inner redan kört; `g.next`×3 sedan `g.return(4)` vid `yield 8` (`sh6_self_host_generator_nested_try_return_into_outer_finally_yield_ok` → **1984**). Inte `async fn*`. |
| 10cw | `g.throw` vid yttre `finally`-`yield` | ✅ **kab-only** | inner redan kört; `g.throw(4)` vid `yield 8` är okastad (inte den `try`:ns `catch`) (`sh6_self_host_generator_nested_try_throw_into_outer_finally_yield_ok` → **1984**). Inte `async fn*`. |
| 10cx | `g.throw` vid inner `finally`-`yield` + yttre `finally` | ✅ **kab-only** | `g.next`×2 sedan `g.throw(4)` vid inner `yield 9` → yttre `catch` + `finally` `yield 8` (`sh6_self_host_generator_nested_try_throw_inner_finally_outer_finally_ok` → **1984**). Inte `async fn*`. |
| 10cy | `g.return` vid inner `finally`-`yield` + yttre `finally` | ✅ **kab-only** | `g.next`×2 sedan `g.return(4)` vid inner `yield 9` → yttre `finally` `yield 8` (`sh6_self_host_generator_nested_try_return_inner_finally_outer_finally_ok` → **1984**). Inte `async fn*`. |
| 10cz | `g.next(v)` send in i yttre `finally`-`yield` | ✅ **kab-only** | inner redan kört; `let x = yield 8; return x` + `g.next(4)` (`sh6_self_host_generator_nested_try_outer_finally_send_ok` → **1984**). Inte `async fn*`. |
| 10da | `g.next(v)` send in i inner `finally`-`yield` | ✅ **kab-only** | `let x = yield 9; return x` + yttre `finally` `yield 8` (`sh6_self_host_generator_nested_try_inner_finally_send_ok` → **1984**). Inte `async fn*`. |
| 10db | `g.next(v)` send in i inner `try`-`yield` | ✅ **kab-only** | `let x = yield 1; return x` kör inner + yttre `finally`-`yield` (`sh6_self_host_generator_nested_try_inner_try_send_ok` → **1984**). Inte `async fn*`. |
| 10dc | `yield*` i inner `try` + yttre `finally` | ✅ **kab-only** | `yield* inner()` sedan inner `yield 9`, yttre `yield 8`, `return 4` (`sh6_self_host_generator_nested_try_yield_star_outer_finally_ok` → **1984**). Inte `async fn*`. |
| 10dd | `g.throw` genom `yield*` i inner `try` + yttre `finally` | ✅ **kab-only** | `yield* inner()` (yield 1), `g.throw(4)` → inner `yield 9`, yttre `catch` + `yield 8`, completion **4** (`sh6_self_host_generator_nested_try_yield_star_throw_outer_finally_ok` → **1984**). Inte `async fn*`. |
| 10de | `g.return` genom `yield*` i inner `try` + yttre `finally` | ✅ **kab-only** | `yield* inner()` (yield 1), `g.return(4)` → inner `yield 9`, yttre `yield 8`, completion **4** (`sh6_self_host_generator_nested_try_yield_star_return_outer_finally_ok` → **1984**). Inte `async fn*`. |
| 10df | `g.next(v)` send genom `yield*` i inner `try` + yttre `finally` | ✅ **kab-only** | `let x = yield* inner()` + `return x`; `g.next(4)` → inner `yield 9`, yttre `yield 8`, completion **4** (`sh6_self_host_generator_nested_try_yield_star_send_outer_finally_ok` → **1984**). Inte `async fn*`. |
| 10dg | send genom `yield*` när inner-`fn*` har `try/finally` | ✅ **kab-only** | inner `let x = yield 1; return x` + `finally yield 7`; yttre `yield 9`/`8`; `g.next(4)` (`sh6_self_host_generator_nested_try_yield_star_inner_try_send_outer_finally_ok` → **17984**). Inte `async fn*`. |
| 10dh | `g.throw` genom `yield*` när inner-`fn*` har `try/finally` | ✅ **kab-only** | inner `finally yield 7` sedan yttre `yield 9`/`8`, completion **4** (`sh6_self_host_generator_nested_try_yield_star_inner_try_throw_outer_finally_ok` → **17984**). Inte `async fn*`. |
| 10di | `g.return` genom `yield*` när inner-`fn*` har `try/finally` | ✅ **kab-only** | inner `finally yield 7` sedan yttre `yield 9`/`8`, completion **4** (`sh6_self_host_generator_nested_try_yield_star_inner_try_return_outer_finally_ok` → **17984**). Inte `async fn*`. |
| 10dj | send in i inner-`fn*`:s `finally`-`yield` via `yield*` | ✅ **kab-only** | `let x = yield 7; return x` i inner `finally`; `g.next`×2 sedan `g.next(4)` (`sh6_self_host_generator_nested_try_yield_star_inner_finally_send_outer_finally_ok` → **17984**). Inte `async fn*`. |
| 10dk | `g.throw` in i inner-`fn*`:s `finally`-`yield` via `yield*` | ✅ **kab-only** | `g.next`×2 sedan `g.throw(4)` vid inner `yield 7` → yttre `yield 9`/`8`, completion **4** (`sh6_self_host_generator_nested_try_yield_star_inner_finally_throw_outer_finally_ok` → **17984**). Inte `async fn*`. |
| 10dl | `g.return` in i inner-`fn*`:s `finally`-`yield` via `yield*` | ✅ **kab-only** | `g.next`×2 sedan `g.return(4)` vid inner `yield 7` → yttre `yield 9`/`8`, completion **4** (`sh6_self_host_generator_nested_try_yield_star_inner_finally_return_outer_finally_ok` → **17984**). Inte `async fn*`. |
| 10dm | `g.next(v)` send in i yttre `finally`-`yield` efter `yield*` | ✅ **kab-only** | inner redan kört; `let x = yield 8; return x` + `g.next(4)` (`sh6_self_host_generator_nested_try_yield_star_outer_finally_send_ok` → **1984**). Inte `async fn*`. |
| 10dn | `g.throw` vid yttre `finally`-`yield` efter `yield*` | ✅ **kab-only** | inner redan kört; `g.throw(4)` vid `yield 8` är okastad (inte den `try`:ns `catch`) (`sh6_self_host_generator_nested_try_yield_star_throw_into_outer_finally_yield_ok` → **1984**). Inte `async fn*`. |
| 10do | `g.return` vid yttre `finally`-`yield` efter `yield*` | ✅ **kab-only** | inner redan kört; `g.next`×3 sedan `g.return(4)` vid `yield 8` (`sh6_self_host_generator_nested_try_yield_star_return_into_outer_finally_yield_ok` → **1984**). Inte `async fn*`. |
| 10dp | nästlad `yield* mid()`/`inner()` + yttre `finally` | ✅ **kab-only** | `yield* mid()` → inner yield 1, sedan `yield 9`/`8`, `return 4` (`sh6_self_host_generator_nested_try_yield_star_nested_outer_finally_ok` → **1984**). Inte `async fn*`. |
| 10dq | `g.throw` genom nästlad `yield* mid()`/`inner()` + yttre `finally` | ✅ **kab-only** | `g.next` → 1, `g.throw(4)` → `yield 9`/`8`, completion **4** (`sh6_self_host_generator_nested_try_yield_star_nested_throw_outer_finally_ok` → **1984**). Inte `async fn*`. |
| 10dr | `g.return` genom nästlad `yield* mid()`/`inner()` + yttre `finally` | ✅ **kab-only** | `g.next` → 1, `g.return(4)` → `yield 9`/`8`, completion **4** (`sh6_self_host_generator_nested_try_yield_star_nested_return_outer_finally_ok` → **1984**). Inte `async fn*`. |
| 10ds | `g.next(v)` send genom nästlad `yield* mid()`/`inner()` + yttre `finally` | ✅ **kab-only** | `let z = yield* mid()` + `return z`; `g.next(4)` → `yield 9`/`8`, completion **4** (`sh6_self_host_generator_nested_try_yield_star_nested_send_outer_finally_ok` → **1984**). Inte `async fn*`. |
| 10dt | `yield*` av array i inner `try` + yttre `finally` | ✅ **kab-only** | `yield* [1]` sedan inner `yield 9`, yttre `yield 8`, `return 4` (`sh6_self_host_generator_nested_try_yield_star_array_outer_finally_ok` → **1984**). Inte `async fn*`. |
| 10du | `g.throw` genom `yield*` av array i inner `try` + yttre `finally` | ✅ **kab-only** | `yield* [1]`, `g.throw(4)` → inner `yield 9`, yttre `catch` + `yield 8`, completion **4** (`sh6_self_host_generator_nested_try_yield_star_array_throw_outer_finally_ok` → **1984**). Inte `async fn*`. |
| 10dv | `g.return` genom `yield*` av array i inner `try` + yttre `finally` | ✅ **kab-only** | `yield* [1]`, `g.return(4)` → inner `yield 9`, yttre `yield 8`, completion **4** (`sh6_self_host_generator_nested_try_yield_star_array_return_outer_finally_ok` → **1984**). Inte `async fn*`. |
| 10dw | `g.next(v)` send in i yttre `finally`-`yield` efter `yield*` av array | ✅ **kab-only** | `yield* [1]` redan kört; `let x = yield 8; return x` + `g.next(4)` (`sh6_self_host_generator_nested_try_yield_star_array_outer_finally_send_ok` → **1984**). Inte `async fn*`. |
| 10dx | `g.throw` vid yttre `finally`-`yield` efter `yield*` av array | ✅ **kab-only** | inner redan kört; `g.throw(4)` vid `yield 8` är okastad (inte den `try`:ns `catch`) (`sh6_self_host_generator_nested_try_yield_star_array_throw_into_outer_finally_yield_ok` → **1984**). Inte `async fn*`. |
| 10dy | `g.return` vid yttre `finally`-`yield` efter `yield*` av array | ✅ **kab-only** | inner redan kört; `g.next`×3 sedan `g.return(4)` vid `yield 8` (`sh6_self_host_generator_nested_try_yield_star_array_return_into_outer_finally_yield_ok` → **1984**). Inte `async fn*`. |
| 10dz | `yield*` av `{ next() }` i inner `try` + yttre `finally` | ✅ **kab-only** | custom iterator yield 1 sedan inner `yield 9`, yttre `yield 8`, `return 4` (`sh6_self_host_generator_nested_try_yield_star_custom_outer_finally_ok` → **1984**). Inte `async fn*`. |
| 10ea | `g.throw` genom `yield*` av `{ next() }` (utan `throw`) + yttre `finally` | ✅ **kab-only** | `g.throw(4)` vid yield* → inner `yield 9`, yttre `catch` + `yield 8`, completion **4** (`sh6_self_host_generator_nested_try_yield_star_custom_throw_outer_finally_ok` → **1984**). Inte `async fn*`. |
| 10eb | `g.return` genom `yield*` av `{ next() }` (utan `return`) + yttre `finally` | ✅ **kab-only** | `g.return(4)` vid yield* → inner `yield 9`, yttre `yield 8`, completion **4** (`sh6_self_host_generator_nested_try_yield_star_custom_return_outer_finally_ok` → **1984**). Inte `async fn*`. |
| 10ec | `g.next(v)` send in i yttre `finally`-`yield` efter `yield*` av `{ next() }` | ✅ **kab-only** | custom iterator redan kört; `let x = yield 8; return x` + `g.next(4)` (`sh6_self_host_generator_nested_try_yield_star_custom_outer_finally_send_ok` → **1984**). Inte `async fn*`. |
| 10ed | `g.throw` vid yttre `finally`-`yield` efter `yield*` av `{ next() }` | ✅ **kab-only** | iterator redan kört; `g.throw(4)` vid `yield 8` är okastad (inte den `try`:ns `catch`) (`sh6_self_host_generator_nested_try_yield_star_custom_throw_into_outer_finally_yield_ok` → **1984**). Inte `async fn*`. |
| 10ee | `g.return` vid yttre `finally`-`yield` efter `yield*` av `{ next() }` | ✅ **kab-only** | iterator redan kört; `g.next`×3 sedan `g.return(4)` vid `yield 8` (`sh6_self_host_generator_nested_try_yield_star_custom_return_into_outer_finally_yield_ok` → **1984**). Inte `async fn*`. |
| 10ef | `g.throw` genom `yield*` av `{ next(), throw() }` + yttre `finally` | ✅ **kab-only** | `throw()` yieldar **4** (`done: false`); `catch` körs inte; sedan `yield 9`/`8` (`sh6_self_host_generator_nested_try_yield_star_custom_throw_method_outer_finally_ok` → **1498**). Inte `async fn*`. |
| 10eg | `g.return` genom `yield*` av `{ next(), return() }` + yttre `finally` | ✅ **kab-only** | `return()` yieldar **4** (`done: false`); sedan inner `yield 9`, yttre `yield 8` (`sh6_self_host_generator_nested_try_yield_star_custom_return_method_outer_finally_ok` → **1498**). Inte `async fn*`. |
| 10eh | `g.next(v)` send genom `yield*` av `{ next() }` + yttre `finally` | ✅ **kab-only** | `let x = yield* it` + `return x`; andra `next(4)` är `done: true` (`sh6_self_host_generator_nested_try_yield_star_custom_send_outer_finally_ok` → **1984**). Inte `async fn*`. |
| 10ei | `yield*` av `Symbol.iterator` i inner `try` + yttre `finally` | ✅ **kab-only** | factory `mkIter` yield 1 sedan inner `yield 9`, yttre `yield 8`, `return 4` (`sh6_self_host_generator_nested_try_yield_star_symbol_iterator_outer_finally_ok` → **1984**). Inte `async fn*`. |
| 10ej | `g.throw` genom `yield*` av `Symbol.iterator` (utan `throw`) + yttre `finally` | ✅ **kab-only** | `g.throw(4)` vid yield* → inner `yield 9`, yttre `catch` + `yield 8`, completion **4** (`sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_outer_finally_ok` → **1984**). Inte `async fn*`. |
| 10ek | `g.return` genom `yield*` av `Symbol.iterator` (utan `return`) + yttre `finally` | ✅ **kab-only** | `g.return(4)` vid yield* → inner `yield 9`, yttre `yield 8`, completion **4** (`sh6_self_host_generator_nested_try_yield_star_symbol_iterator_return_outer_finally_ok` → **1984**). Inte `async fn*`. |
| 10el | `g.next(v)` send genom `yield*` av `Symbol.iterator` + yttre `finally` | ✅ **kab-only** | `let x = yield* o` + `return x`; andra `next(4)` är `done: true` (`sh6_self_host_generator_nested_try_yield_star_symbol_iterator_send_outer_finally_ok` → **1984**). Inte `async fn*`. |
| 10em | `g.next(v)` send in i yttre `finally`-`yield` efter `yield*` av `Symbol.iterator` | ✅ **kab-only** | iterator redan kört; `let x = yield 8; return x` + `g.next(4)` (`sh6_self_host_generator_nested_try_yield_star_symbol_iterator_outer_finally_send_ok` → **1984**). Inte `async fn*`. |
| 10en | `g.throw` vid yttre `finally`-`yield` efter `yield*` av `Symbol.iterator` | ✅ **kab-only** | iterator redan kört; `g.throw(4)` vid `yield 8` är okastad (inte den `try`:ns `catch`) (`sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_into_outer_finally_yield_ok` → **1984**). Inte `async fn*`. |
| 10eo | `g.return` vid yttre `finally`-`yield` efter `yield*` av `Symbol.iterator` | ✅ **kab-only** | iterator redan kört; `g.next`×3 sedan `g.return(4)` vid `yield 8` (`sh6_self_host_generator_nested_try_yield_star_symbol_iterator_return_into_outer_finally_yield_ok` → **1984**). Inte `async fn*`. |
| 10ep | `g.throw` genom `yield*` av `Symbol.iterator` med `throw()` + yttre `finally` | ✅ **kab-only** | iterator-`throw()` yieldar **4** (`done: false`); `catch` körs inte; sedan `yield 9`/`8` (`sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_method_outer_finally_ok` → **1498**). Inte `async fn*`. |
| 10eq | `g.return` genom `yield*` av `Symbol.iterator` med `return()` + yttre `finally` | ✅ **kab-only** | iterator-`return()` yieldar **4** (`done: false`); sedan inner `yield 9`, yttre `yield 8` (`sh6_self_host_generator_nested_try_yield_star_symbol_iterator_return_method_outer_finally_ok` → **1498**). Inte `async fn*`. |
| 10er | `g.throw` genom `yield*` av `{ next(), throw() }` där `throw()` kastar vidare | ✅ **kab-only** | `throw e` i iterator-`throw` → inner `yield 9`, yttre `catch` + `yield 8`, completion **4** (`sh6_self_host_generator_nested_try_yield_star_custom_throw_rethrows_outer_finally_ok` → **1984**). Inte `async fn*`. |
| 10es | `g.throw` genom `yield*` av `Symbol.iterator` vars `throw()` kastar vidare | ✅ **kab-only** | factory-iterator `throw e` → inner `yield 9`, yttre `catch` + `yield 8`, completion **4** (`sh6_self_host_generator_nested_try_yield_star_symbol_iterator_throw_rethrows_outer_finally_ok` → **1984**). Inte `async fn*`. |
| 10et | `g.return` genom `yield*` av `{ next(), return() }` där `return()` kastar | ✅ **kab-only** | `throw v` i iterator-`return` → inner `yield 9`, yttre `catch` + `yield 8`, completion **4** (`sh6_self_host_generator_nested_try_yield_star_custom_return_throws_outer_finally_ok` → **1984**). Inte `async fn*`. |
| 10eu | `g.return` genom `yield*` av `Symbol.iterator` vars `return()` kastar | ✅ **kab-only** | factory-iterator `throw v` i `return()` → inner `yield 9`, yttre `catch` + `yield 8`, completion **4** (`sh6_self_host_generator_nested_try_yield_star_symbol_iterator_return_throws_outer_finally_ok` → **1984**). Inte `async fn*`. |
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