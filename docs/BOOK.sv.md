# Språket Kabootar — En komplett referens

## Förord

Kabootar är ett fullstack-språk för system och applikationer med JavaScript-liknande syntax, explicit typbeteende och en stor runtime som täcker allt från HTTP/SQL till workers, krypto och självhostning. Denna bok beskriver språket som det finns i implementeringen `nova-interpreter`.

Språket är avsiktligt inte JavaScript: det tar bort implicit typkonvertering, `var`-hoisting, prototyparv och `eval`. I gengäld erbjuder det förutsägbara typer, Rust-liknande `match`, C#-inspirerade klasser, moduler, async/generatorer och en självhostad kompileringspipeline.

Denna referens är uppdelad i en kärnhandledning i språket (kapitel 1–10), en runtime-guide (kapitel 11–13) och en kompilator-/självhostningsguide (kapitel 14).

---

## Kapitel 1 — Introduktion

### 1.1 Vad är Kabootar?

Kabootar är ett generellt språk som kompilerar till ett internt bytecode-format (`.kbc`) och körs på en Rust-baserad VM. Det har tre huvudmål:

1. **Välbekant syntax** — C/Rust/JavaScript-liknande token och block.
2. **Förutsägbara typer** — ingen tyst konvertering; `null`, `undefined`, `NaN` och `Result` är explicita.
3. **Runtime med batterier medföljer** — moduler för SQL, HTTP, OS-åtkomst, krypto, vetenskap, DOM-rendering med mera.

### 1.2 Hello world

```kabootar
println("Hello, Kabootar!")
```

Kör med:

```bash
kabootar hello.kab
```

### 1.3 Filändelse och modulingång

Källfiler använder `.kab`. Kompilatorn/tolkaren laddar `main.kab` som standard eller fältet `entry` från `kabootar.toml`.

---

## Kapitel 2 — Lexikal struktur och typer

### 2.1 Token och whitespace

- Identifierare: ASCII-bokstäver, siffror och `_`, inte inledande siffra.
- Nyckelord: `fn`, `let`, `const`, `if`, `else`, `while`, `for`, `in`, `of`, `return`, `break`, `continue`, `throw`, `raise`, `try`, `catch`, `finally`, `pass`, `assert`, `with`, `using`, `match`, `switch`, `case`, `default`, `fallthrough`, `do`, `async`, `await`, `yield`, `fn*`, `class`, `extends`, `interface`, `implements`, `enum`, `import`, `pub`, `this`, `super`, `true`, `false`, `null`, `undefined`, `NaN`, `Some`, `None`, `Ok`, `Err`, `is`, `not`, `delete`.
- Operatorer: `+ - * / % **`, `== != < <= > >=`, `&& || ??`, `! ~`, `& | ^ << >> >>>`, `? :`, `=>`, `= += -= *= /= %= **=`.
- Kommentarer: `// radkommentar` och `/* blockkommentar */`.
- Semikolon är valfria mellan satser; radbrytningar separerar satser i block.

### 2.2 Primitiva typer

| Typ | Literal | Notering |
|-----|---------|----------|
| Heltal | `42`, `-7` | 64-bitars signed. |
| Flyttal | `3.14`, `NaN` | 64-bitars IEEE 754. |
| BigInt | `123n`, `BigInt("99")` | Godtyckligt stort heltal. Ingen blandning med `number`. |
| Sträng | `"text"`, `` `template ${x}` `` | UTF-8. Mallar stöder `${expr}`-interpolation. |
| Boolean | `true`, `false` | |
| Null | `null` | Explicit "inget värde". |
| Undefined | `undefined` | Oinitierad binding eller saknad nyckel/index. |

### 2.3 Sammansatta typer

- **Array**: `[1, 2, 3]`.
- **Objekt**: `{ "a": 1, b: 2 }`.
- **Map/Set**: Skapas via `map_new` / `set_new`.
- **Klassinstanser**: Skapas från `class`-definitioner.
- **Result**: `Ok(v)`, `Err(e)`.
- **Option**: `Some(v)`, `None`.

### 2.4 Sanning

Falska värden: `null`, `undefined`, `false`, `0`, `""`, `NaN`. Allt annat är sant.

### 2.5 `null` kontra `undefined`

- `null` = medveten frånvaro.
- `undefined` = oinitierad eller saknad.
- `null == undefined` är `false` i Kabootar.

---

## Kapitel 3 — Variabler och konstanter

### 3.1 `let` och `const`

```kabootar
let x = 10
x = 20               // ok

const PI = 3.14
PI = 3               // runtime-fel

let y                // undefined
println(y)           // skriver "undefined"
```

Det finns ingen `var`.

**Exempel: byta värden**

```kabootar
let a = 1
let b = 2
let tmp = a
a = b
b = tmp
println(a)           // 2
println(b)           // 1
```

### 3.2 Destrukturering

```kabootar
let [a, b, ...rest] = [1, 2, 3, 4]
println(a)           // 1
println(rest)        // [3, 4]

let { name, age } = { name: "Ada", age: 36 }
println(name)        // "Ada"

let { name: n, ...restObj } = { name: "Ada", age: 36, city: "Stockholm" }
println(n)           // "Ada"
println(restObj)     // { age: 36, city: "Stockholm" }
```

### 3.3 Spread

```kabootar
let base = { x: 1, y: 2 }
let merged = { ...base, extra: 1 }
println(merged)      // { x: 1, y: 2, extra: 1 }

let xs = [1, 2]
let all = [...xs, 99]
println(all)         // [1, 2, 99]

fn f(a, ...rest) {
    return len(rest)
}
println(f(1, 2, 3))  // 2
```

### 3.4 Scope

Block-scope likt JavaScript `let`. Att läsa en odeklarerad variabel är ett runtime-fel, inte `undefined`.

```kabootar
let outer = 10
if true {
    let inner = 20
    println(outer)   // 10
}
// println(inner)    // runtime-fel: odeklarerad variabel
```

---

## Kapitel 4 — Uttryck och operatorer

### 4.1 Aritmetik

```kabootar
println(1 + 2)     // 3
println(3 - 4)     // -1
println(5 * 6)     // 30
println(7 / 8)     // 0.875 (flyttal)
println(9 % 2)     // 1
println(2 ** 3 ** 2)  // 512
```

Heltalsdivision med noll är ett fel. Flyttals-`NaN` är explicit via literalen `NaN` eller flyttalsmatematik.

**Exempel: cirkelns area**

```kabootar
const PI = 3.14159
let r = 5
let area = PI * r * r
println(area)      // 78.53975
```

### 4.2 Jämförelse

```kabootar
println(1 == 1)                  // true
println(1 != 2)                  // true
println(3 < 5)                   // true
println("5" == 5)                // false (strikt per typ)
println(null == undefined)       // false
```

### 4.3 Logik

```kabootar
let a = true
let b = false
println(a && b)          // false
println(a || b)          // true
println(!a)              // false
println(0 ?? 9)          // 0
println(null ?? 9)       // 9
```

### 4.4 Bitvis

```kabootar
println(5 & 3)           // 1   (0101 & 0011)
println(5 | 3)           // 7   (0101 | 0011)
println(5 ^ 3)           // 6   (0101 ^ 0011)
println(~5)              // -6
println(1 << 3)          // 8
println(8 >> 2)          // 2
println(-8 >>> 2)        // 1073741822
```

Tillämpas med 32-bitars heltalssemantik (`ToInt32` / `ToUint32`).

### 4.5 Ternär

```kabootar
let n = -5
let sign = n > 0 ? "positive" : "non-positive"
println(sign)            // "non-positive"

let age = 20
let label = age >= 18 ? "adult" : "minor"
println(label)           // "adult"
```

### 4.6 `?` på Result

```kabootar
fn may_fail(x) {
    if x > 0 {
        return Ok(x * 2)
    }
    return Err("x must be positive")
}

fn load() {
    let value = may_fail(3)?
    println(value)       // 6
    return value
}
```

`?` packar upp `Ok(v)` eller returnerar `Err(e)` från den aktuella funktionen.

### 4.7 `in`

```kabootar
let obj = { a: 1, b: 2 }
let arr = [10, 20, 30]
println("a" in obj)      // true
println("c" in obj)      // false
println(1 in arr)        // true
println(5 in arr)        // false
println("x" in "xyz")    // true
```

### 4.8 `is` / `is not`

```kabootar
let x = null
println(x is null)           // true
println(x is not undefined)  // true

let y = 0
println(y is false)          // false
```

### 4.9 Medlems- och indexåtkomst

```kabootar
let obj = { name: "Kabootar", version: 2 }
println(obj.name)        // "Kabootar"
println(obj["version"])  // 2

let arr = [10, 20, 30]
println(arr[0])          // 10

let maybe = null
println(maybe?.field)    // undefined

fn greet() { return "hi" }
println(greet?.())       // "hi"
```

### 4.10 Tilldelningsuttryck

```kabootar
let x = 1
x += 4
println(x)               // 5

let arr = [0, 0, 0]
arr[1] = 42
println(arr)             // [0, 42, 0]

let obj = {}
obj.field = "x"
println(obj.field)       // "x"

let pair = [1, 2]
let [a, b] = pair
println(a)               // 1
```

### 4.11 `delete`

```kabootar
delete obj.key
```

---

## Kapitel 5 — Kontrollflöde

### 5.1 `if` / `else`

Parenteser runt villkoret är valfria:

```kabootar
if x < 0 {
    println("negative")
} else if x == 0 {
    println("zero")
} else {
    println("positive")
}
```

### 5.2 `while`

```kabootar
let i = 0
while i < 10 {
    println(i)
    i = i + 1
}
```

`break` och `continue` stöds.

### 5.3 `do … while`

```kabootar
do {
    i = i + 1
} while i < 10
```

### 5.4 `for`-loopar

```kabootar
// C-stil
for let i = 0; i < len(xs); i = i + 1 {
    println(xs[i])
}

// värden
for x of xs {
    println(x)
}

// index/nycklar
for i in xs {
    println(xs[i])
}

for key in obj {
    println(key)
}
```

`for const x of xs` och `for let x of xs` styr loopvariabelns muterbarhet.

### 5.5 `match`

```kabootar
match n {
    0 => "zero",
    x if x > 0 => "positive",
    _ => "other"
}
```

Mönster inkluderar: literal, variabel, wildcard `_`, `Some`/`None`, `Ok`/`Err`, enum-varianter, arrayer, objekt och garder (`if expr =>`).

### 5.6 `if let` / `while let`

```kabootar
if let Some(x) = opt {
    println(x)
}

while let Ok(v) = r {
    println(v)
}
```

### 5.7 `switch`

```kabootar
switch (x) {
    case 1: {
        println("one")
    }
    case 2: case 3: {
        println("two or three")
    }
    default: {
        println("other")
    }
}
```

Ingen implicit fall-through; använd `fallthrough` explicit.

### 5.8 `try` / `catch` / `finally`

```kabootar
try {
    risky()
} catch (e) {
    println(e)
} finally {
    cleanup()
}
```

`try`/`catch` arbetar på `Result`-värden: `Ok(v)` packas upp till `v`, `Err(e)` går in i catch-blocket.

### 5.9 `pass`, `assert`, `with`, `using`

```kabootar
pass                            // no-op
assert(x > 0, "must be positive")

with resource as r {            // binder resource till r, disposer vid blockslut
    use(r)
}

using x = expr;                 // explicit dispose via Symbol.dispose / dispose() / close()
```

---

## Kapitel 6 — Funktioner

### 6.1 Namngivna funktioner

```kabootar
fn add(a, b) {
    return a + b
}

pub fn exported(a) {            // exporteras från modul
    return a
}
```

### 6.2 Pilfunktioner

```kabootar
let double = (x) => x * 2
let sum = (a, b) => {
    return a + b
}
```

### 6.3 Parametrar

```kabootar
fn greet(name = "world") { }
fn sum(...xs) { }               // rest-parameter
fn f(a, b = 2, ...rest) { }
```

### 6.4 Asynkrona funktioner och `await`

```kabootar
async fn fetch() {
    return 42
}

async fn main() {
    let n = await fetch()
    return n
}
```

`await` fungerar endast inuti `async fn` eller asynkrona pilar. Det tömmer microtask-kön FIFO.

### 6.5 Generatorer

```kabootar
fn* counter() {
    yield 1
    yield 2
    return 99
}

let it = counter()
let a = it.next().value   // 1
let b = it.next().value   // 2
let c = it.next().value   // 99
```

`yield*` delegerar till en annan iterable/generator. Asynkrona generatorer använder `async fn*`.

### 6.6 Funktioner av högre ordning

Funktioner är förstaklassiga och kan skickas, returneras och lagras i variabler.

### 6.7 `return`

`return` är valfritt i slutet av en funktion; det sista uttryckets värde returneras automatiskt om det inte finns explicit `return`.

---

## Kapitel 7 — Objekt, arrayer och klasser

### 7.1 Arrayer

```kabootar
let xs = [1, 2, 3]
xs.push(4)                     // muterar
let ys = map(xs, (x) => x * 2)
let n = len(xs)
```

Array-hjälpare inkluderar `map`, `filter`, `reduce`, `find`, `slice`, `sort`, `reverse`, `join`, `includes`, `some`, `every`, `index_of`, `flat`, `flat_map`, `at`, `fill`, `to_spliced`, `to_reversed`, `to_sorted`, `shift`, `unshift`, `splice`, `concat`.

### 7.2 Objekt

```kabootar
let u = { name: "Ada", age: 36 }
u.name
u["name"]
```

Objekthjälpare inkluderar `assign`, `has_key`, `delete_prop`, `keys`, `values`, `entries`, `from_entries`, `clone_shallow`, `group_by`.

### 7.3 Klasser

Kabootar-klasser är C#-stil, inte JavaScript-prototyper.
Receiver i metoder är **`this`**. Nyckelordet **`self`** är reserverat för framtida `struct` (Rust-stil).

```kabootar
class Point {
    x: number;
    y: number;

    fn init(a, b) {
        this.x = a
        this.y = b
    }

    fn sum() {
        return this.x + this.y
    }
}

let p = Point(3, 4)
p.sum()                     // 7
```

### 7.4 Arv

```kabootar
class Dog extends Animal {
    breed: string;

    fn init(n, b) {
        super.init(n)
        this.breed = b
    }
}
```

### 7.5 Interface

```kabootar
interface Greeter {
    fn greet();
}

class Person implements Greeter {
    name: string;
    fn greet() {
        return "hi " + this.name
    }
}

is_impl(p, "Greeter")       // true
```

### 7.6 Enums

```kabootar
enum Color { Red, Green }
enum Msg { Move(x, y), Stop }

match c {
    Color.Red => "red",
    _ => "other"
}
```

### 7.7 Privata fält

```kabootar
class C {
    #n: number = 0;
    fn get() { return this.#n }
}
```

---

## Kapitel 8 — Mönstermatchning

### 8.1 Mönster

Ett mönster kan vara:

- Literal: `1`, `"x"`, `true`, `null`, `undefined`, `NaN`
- Variabel: `x`
- Wildcard: `_`
- `Some(p)`, `None`
- `Ok(p)`, `Err(p)`
- Enum-variant: `Color.Red`, `Msg.Move(x, y)`
- Array: `[a, b, ...rest]`
- Objekt: `{ name, age: a }`

### 8.2 Match-armar med garder

```kabootar
match n {
    x if x > 0 => "positive",
    0 => "zero",
    _ => "negative"
}
```

### 8.3 `if let` / `while let`

Syntaktiskt socker över `match`.

---

## Kapitel 9 — Moduler

### 9.1 Import-syntax

```kabootar
import "math"
add(1, 2)

import "greet"
greet("Kabootar")
```

Importer binder namn in i den aktuella miljön.

### 9.2 Inbyggda moduler

| Modul | Innehåll |
|-------|----------|
| `std` | JSON, typ-hjälpare |
| `json` | `parse`, `dump` |
| `collections` | Map/Set-hjälpare |
| `strings` | strängverktyg |
| `math` | grundläggande aritmetikhjälpare |
| `http` | HTTP-routing och hämtning |
| `crypto` | kryptografiska funktioner |
| `science` | fysik, kemi, statistik |
| `docai` | dokumentations-AI-hjälpare |
| `codai` | kod-AI-hjälpare |
| `sql` | in-process SQL-databas |
| `os` | sandboxat filsystem |

### 9.3 Filmoduler

Skapa `lib/greet.kab`:

```kabootar
pub fn greet(name) {
    return "Hello, " + name
}

fn secret() { }             // privat
```

Endast `pub fn`, `pub let` och `pub const` exporteras.

### 9.4 Projektstruktur

```toml
# kabootar.toml
version = "0.1.0"
entry = "main.kab"
port = 8080

[dependencies]
greet = "1.0.0"
```

### 9.5 Versionsstyrda importer

```kabootar
import "greet@1.0"
```

### 9.6 Lokalt paketregister

```bash
kabootar publish lib/greet.kab
kabootar install greet@1.0
kabootar install
```

### 9.7 `import.meta` och dynamisk import

```kabootar
let url = import.meta.url
let mod = await import("math")   // Promise med modul-namespace
```

---

## Kapitel 10 — Felhantering

### 10.1 `Result` och `Option`

```kabootar
let r = Ok(42)
let e = Err("failed")
let opt = Some(99)
let empty = None
```

### 10.2 `?`-operator

```kabootar
fn step() {
    return may_fail()?
}
```

### 10.3 `throw` / `raise`

```kabootar
throw "bad input"
raise Error("bad input")
```

Fångas med `try`/`catch`.

### 10.4 `Error`-hjälpare

```kabootar
error_new("msg", { cause: err })
error_stack(e)
error_cause(e)
type_error("expected number")
reference_error("missing")
range_error("out of range")
```

---

## Kapitel 11 — Asynkron programmering och generatorer

### 11.1 Promises

```kabootar
let p = promise_new((resolve, reject) => {
    resolve(42)
})

promise_then(p, (v) => v * 2)
promise_all([p1, p2])
await_all([p1, p2])             // returnerar array synkront
```

### 11.2 Asynkrona iteratorer

```kabootar
async fn* stream() {
    yield 1
    yield 2
}

async fn consume() {
    for await x of stream() {
        println(x)
    }
}
```

### 11.3 Iterator-hjälpare

Lata iteratoradapter:

```kabootar
let it = iterator_map(gen(), (n) => n * 2)
iterator_filter(it, (n) => n > 0)
iterator_take(it, 5)
iterator_skip(it, 2)
iterator_chain(a, b)
iterator_zip(a, b)
iterator_enumerate(it)
iterator_flat_map(it, fn, depth)
iterator_drop_while(it, fn)
iterator_accumulate(it, fn, initial)
iterator_pairwise(it)
```

Konsumeras med `for … of`, `.toArray()`, `.reduce()`, `.find()` med flera.

---

## Kapitel 12 — Standardbibliotek

### 12.1 Matematik

`floor`, `ceil`, `round`, `abs`, `min`, `max`, `sqrt`, `pow`, `random`, `sign`, `trunc`, `clamp`, `pi`, `e`, `log`, `log2`, `log10`, `exp`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `hypot`, `cbrt`, `imul`, `clz32`, `fround`, `fmod`, `log1p`, `expm1`.

### 12.2 Talformatering

`parse_int`, `parse_float`, `is_finite`, `is_integer`, `is_safe_integer`, `to_fixed`, `to_exponential`, `to_precision`, `number_to_string`.

### 12.3 Sträng

`len`, `trim`, `trim_start`, `trim_end`, `split`, `replace`, `replace_all`, `repeat`, `pad_start`, `pad_end`, `char_at`, `char_code_at`, `from_char_code`, `code_point_at`, `from_code_point`, `starts_with`, `str_includes`, `str_last_index_of`, `string_split`, `string_to_lower`, `string_to_upper`, `string_replace`, `string_replace_all`.

### 12.4 JSON

`json_parse`, `json_stringify(v, indent)`.

### 12.5 RegExp

```kabootar
let re = /a.b/s
regex_test(re, text)
regex_match(re, text)
regex_replace(re, text, "x")
regex_replace_all(re, text, "x")
regexp_new("a.b", "s")
regex_escape("a.b")
```

### 12.6 Datum

`date_now`, `date_parse`, `date_format`, `date_iso`, `date_new`, `date_get_time`, `date_get_full_year`, `date_to_iso_string`.

### 12.7 Timers

`sleep_ms`, `sleep_ticks`, `set_timeout`, `clear_timeout`, `set_interval`, `clear_interval`, `performance.now()`.

### 12.8 URI-kodning

`encode_uri`, `decode_uri`, `encode_uri_component`, `decode_uri_component`.

### 12.9 Kollektioner

`map_new`, `map_get`, `map_set`, `map_has`, `map_delete`, `map_clear`, `map_keys`, `map_values`, `map_entries`, `map_for_each`, `map_from_entries`, `map_group_by`.

`set_new`, `set_add`, `set_has`, `set_delete`, `set_clear`, `set_values`, `set_for_each`, `set_union`, `set_intersection`, `set_difference`, `set_symmetric_difference`, `set_is_subset`, `set_is_superset`, `set_is_disjoint`.

`weak_map_new`, `weak_set_new`.

### 12.10 Typade arrayer och bufferar

`array_buffer_new`, `float64_array_new/get/set`, `data_view_new`, `uint8_array_new/get/set`, `int32_array_new/get/set`, `sab_new`, `sab_byte_length`, `sab_transfer`.

### 12.11 Atomics

`atomics_load`, `atomics_store`, `atomics_add`, `atomics_sub`, `atomics_and`, `atomics_or`, `atomics_xor`, `atomics_exchange`, `atomics_compare_exchange`, `atomics_wait`, `atomics_notify`.

### 12.12 Textkodning

`text_encode`, `text_decode`, `btoa`, `atob`.

### 12.13 Krypto

`crypto.getRandomValues(array)`.

### 12.14 Miljö

`env_get`, `env_set`, `env_has`, `env_delete`, `env_to_object`, `cwd`, `globalThis()`.

### 12.15 Typkontroller

`typeof()`, `is_null`, `is_undefined`, `is_nan`, `is_array`, `is_promise`, `is_error`, `is_regexp`, `is_proxy`, `is_weakmap`, `is_weakset`, `type_assert`.

---

## Kapitel 13 — Runtime-miljön

### 13.1 CLI-kommandon

```bash
kabootar run script.kab
kabootar compile main.kab
kabootar serve --watch main.kab
kabootar mod init api
kabootar publish lib/greet.kab
kabootar install greet@1.0
```

### 13.2 HTTP

```kabootar
import "http"

http_route("GET", "/", fn() { return ok("Kabootar") })

async fn fetch() {
    let res = await http_fetch_async("GET", "https://example.com/", "")
    return http_body(res)
}
```

TLS: `tls_add_ca`, `tls_ca_only`, `tls_pin`, `tls_cert_sha256`, `tls_reset`.

### 13.3 SQL

```kabootar
import "sql"

sql("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)")
let rows = sql("SELECT * FROM users")
```

`sql_async`, KV-lager via `open_kv`, `kv_get`, `kv_set`, `kv_list`, `kv_atomic`, `kv_watch`, `kv_enqueue`, `kv_dequeue`.

### 13.4 OS och filsystem

```kabootar
import "os"

os_mkdir("/data")
os_write("/data/log.txt", "hello")
let text = os_read("/data/log.txt")
```

Asynkrona varianter: `os_read_async`, `os_write_async`.

### 13.5 Workers

```kabootar
let w = worker_new()
worker_start(w)
worker_post_message(w, "hello")
let msg = worker_recv(w)
worker_terminate(w)
```

I worker-kontext: `onmessage(handler)`, `postMessage(msg)`, `worker_run_message_loop()`.

### 13.6 Strömmar

`stream_new`, `stream_from_array`, `stream_read`, `stream_read_all`, `stream_pipe_to`, `stream_tee`, `stream_transfer`, `byte_stream_new`, `transform_stream_new`, `reader_read`, `writer_write`.

### 13.7 WebSocket

`ws_channel_pair`, `ws_link`, `ws_send`, `ws_recv`, `ws_connect`, `tcp_listen`, `tcp_connect`, `tcp_accept`, `udp_bind`, `udp_send`, `udp_recv`.

### 13.8 DOM och rendering

Globalen `kdom` och makrot `html! { }` för Kv8 UI. Canvas 2D-API via `canvas_*`-natives. KML-parsning via `kml("...")`.

### 13.9 FFI och npm

```kabootar
let lib = ffi_load("mylib.dll")
ffi_call(lib, "add", [1, 2])

npm_install("lodash")
npm_import("lodash")
```

TypeScript-stripping: `ts_strip_types`, `ts_transpile`, `ts_compile`.

### 13.10 Node.js-kompatibilitetsmoduler

`import "node:fs"`, `"node:path"`, `"node:process"`, `"node:os"`, `"node:url"`, `"node:crypto"`, `"node:buffer"`.

### 13.11 `Intl` och `Temporal`

```kabootar
Intl.NumberFormat("sv-SE", { style: "currency", currency: "SEK" }).format(1234.5)
Temporal.Now.plainDateISO()
Temporal.PlainDate.from({ year: 2026, month: 7, day: 11 })
```

---

## Kapitel 14 — Kompilator och självhostning

### 14.1 Pipeline

```
källtext
    → lexer.kab        → token[]
    → parser.kab         → AST
    → emit.kab           → opcode IR
    → serialize.kab      → .kbc-text
    → compile.kab        → full pipeline
```

Rust-implementeringen speglar dessa faser i `src/lexer.rs`, `src/parser.rs`, `src/bytecode/compiler.rs` och `src/bytecode/types.rs`.

### 14.2 Bytecode-cache

`.kabootar/cache/*.kbc` lagrar kompilerad bytecode. Cacheposter ogiltigförklaras när källfilen är nyare.

### 14.3 Självhostningsstatus

- `lexer.kab` kompilerar och körs via självhost.
- `parser.kab` kompilerar och körs via självhost.
- `emit.kab` kompilerar och körs via självhost (tungt).
- Kommande milstolpar: `serialize.kab` full självhost, därefter sann bootstrap (`compile.kab` kompilerad av självhostad `compile()`).

### 14.4 Designregler för självhostning

- Modul-global state används för scratch-slots; Kabootars funktionslokaler är inte re-entranta över rekursiva anrop.
- `push` returnerar en ny array: tilldela alltid `arr = push(arr, item)`.
- Spara AST-fält före rekursivt nedstigande.
- Använd bracket-åtkomst för AST-nycklar för att undvika krockar.
- Håll toppnivåfunktioner per modul låga (~4–7) för att undvika stack overflow på Windows.
- Nästlade if/while kräver explicita stackar för jump-patchning.

---

## Appendix A — Grammatikreferens

### Satser

```
stmt        := let_stmt | const_stmt | pub_stmt | import_stmt
             | fn_stmt | class_stmt | enum_stmt | interface_stmt
             | return_stmt | if_stmt | while_stmt | do_while_stmt
             | for_stmt | switch_stmt | try_stmt | using_stmt
             | break | continue | pass | assert | expr_stmt

let_stmt    := "let" binding_pattern ("=" expr)?
const_stmt  := "const" binding_pattern "=" expr
pub_stmt    := "pub" (let_stmt | const_stmt | fn_stmt)
fn_stmt     := "async"? "fn" identifier params block
class_stmt  := "class" identifier ("extends" identifier)?
               ("implements" identifier ("," identifier)*)? class_body
enum_stmt   := "enum" identifier "{" enum_variant* "}"
interface_stmt := "interface" identifier "{" fn_signature* "}"
```

### Uttryck

```
expr        := assign_expr
assign_expr := logical_or ("=" | "+=" | "-=" | "*=" | "/=" | "%=" | "**=") assign_expr
             | logical_or
logical_or  := logical_and ("||" logical_and)*
logical_and := nullish ("&&" nullish)*
nullish     := equality ("??" equality)*
equality    := compare (("==" | "!=") compare)*
compare     := bitwise (("<" | ">" | "<=" | ">=") bitwise)*
bitwise     := shift (("&" | "|" | "^") shift)*
shift       := additive (("<<" | ">>" | ">>>") additive)*
additive    := multiplicative (("+" | "-") multiplicative)*
multiplicative := unary (("*" | "/" | "%" | "**") unary)*
unary       := ("!" | "-" | "~" | "delete" | "throw" | "raise") unary | postfix
postfix     := primary ("(" args? ")" | "[" expr "]" | "." identifier | "?." identifier | "?.[" expr | "?.")*
primary     := number | float | bigint | string | template | boolean
             | null | undefined | NaN | identifier
             | "(" expr ")" | array_literal | object_literal
             | fn* block | "this" | "super" | "import" "." "meta"
```

### Mönster

```
pattern     := literal | identifier | "_" | "Some" "(" pattern ")"
             | "None" | "Ok" "(" pattern ")" | "Err" "(" pattern ")"
             | enum_name "." variant ("(" pattern* ")")?
             | "[" pattern_piece* "]" | "{" object_pattern_field* "}"
```

---

## Appendix B — Sammanfattning av inbyggda och native-funktioner

### Konsol

- `println(...)`, `log(...)`, `console_log(...)`, `console_warn(...)`, `console_error(...)`

### Matematik

Se kapitel 12.1.

### Typkontroller

Se kapitel 12.15.

### Array/Objekt

Se kapitel 7.

### Runtime-introspektion

- `lang_info()`, `ecosystem_info()`, `modules_catalog()`

### Register

- `registry_publish(path)`, `registry_install(name, version)`, `registry_list()`, `registry_search(query)`, `registry_seed()`, `registry_uninstall(name, version)`

---

## Appendix C — Skillnader mot JavaScript

| JavaScript | Kabootar |
|------------|----------|
| `var` | borttaget; endast `let`/`const` |
| `===` | `==` är redan strikt per typ |
| implicit konvertering (`"1" + 2`) | runtime-fel |
| `null == undefined` | `false` |
| prototyparv | C#-stil `class` med `this` |
| `function` | `fn` |
| pil `=>` | samma syntax, plus blockkropp |
| `constructor` | `fn init(...)` |
| `for…in` på objekt | `for key in obj` |
| `for…of` på arrayer | `for x of xs` |
| `with`-sats | borttagen |
| `eval()` | borttagen |
| `Infinity`/`-Infinity` | heltalsdivision är fel; flyttal använder `NaN` |

---

*Slut på Språket Kabootar — En komplett referens (svenska).*
