# För dig som kan JavaScript

> **Hoppa över grundsyntax.** Om du redan kan `let`, `if`, `while`, funktioner och arrayer behöver du inte läsa [LANGUAGE.md](LANGUAGE.md) från början. Det här dokumentet listar **bara det som skiljer sig** från JavaScript.

Kabootar är **inte** Node eller en webbläsare — det är ett eget språk med JS-liknande syntax, inbyggd SQL, HTTP, moduler och fullstack-runtime.

---

## 30 sekunder — mental modell

| Du tänker (JS) | I Kabootar |
|----------------|------------|
| `function` / `=>` | `fn` + **`(a, b) => a + b`** |
| `const` / `var` | **`const`** + `let` (ingen `var`) |
| `===` | `==` (redan strikt per typ) |
| `import x from "./x"` | `import "x"` eller fil i `lib/` |
| `module.exports` | `pub fn` / `pub let` i `.kab`-filer |
| `class` + prototyper | `class` + **`this`** (C#-stil). Dataobjekt: **Parent** (`getParent`/`setParent`), inte prototyper |
| `npm` + Express | `http_route`, `sql()` inbyggt |
| `undefined` överallt | bara oinitierade `let`; odekvarerat = **fel** |
| `"1" + 2` → `"12"` | **fel** — ingen implicit typkonvertering |

---

## Syntax — bara skillnaderna

### Funktioner

```javascript
// JavaScript
function add(a, b) { return a + b; }
const add = (a, b) => a + b;
```

```kabootar
// Kabootar
fn add(a, b) {
    return a + b
}
let double = (x) => x * 2
async fn fetch() {
    return 42
}
let n = await fetch()
```

- `fn` för namngivna funktioner; **`(a, b) => expr`** eller block-kropp för pilar.
- **`async fn`** och **`async (n) => ...`** returnerar Promise; **`await`** kör microtask-kön (FIFO).
- `return` utan semikolon fungerar (semikolon är ofta valfritt mellan satser).

### Variabler

```kabootar
let x = 1       // omtilldelningsbar
const PI = 3    // immutable — fel vid PI = 4
let y           // undefined tills tilldelad
// var z = 1     — finns inte (medvetet)
```

| Situation | JavaScript | Kabootar |
|-----------|------------|----------|
| `let x;` sedan läs `x` | `undefined` | `undefined` |
| Läs `foo` utan deklaration | `undefined` (sloppy) / ReferenceError (strict) | **Runtime-fel** |
| `null == undefined` | `true` (`==`) | **`false`** |

### `if` / `while`

Parenteser runt villkor är **valfria** (JS kräver dem):

```javascript
// JS
if (x < 10) { ... }
```

```kabootar
if x < 10 {
    ...
}
```

### Jämförelser och logik

- **`==` / `!=`** jämför **samma typ** — ingen `"5" == 5`-coercion.
- Ingen `===` — du behöver den inte.
- `&&` och `||` finns; använd dem som i JS, men operanderna tvingas inte till boolean via konstig konvertering i aritmetik.
- **`**`** — exponent (högerassociativ): `2 ** 3 ** 2` → `512`.
- **`??`** — nullish coalesce: bara `null`/`undefined` triggar höger sida (`0 ?? 9` → `0`).
- **`&` `|` `^` `~` `<<` `>>` `>>>`** — bitwise på 32-bit heltal (JS ToInt32/ToUint32-semantik).
- **`switch`** — utan implicit fall-through; använd **`fallthrough`** i slutet av ett `case`-block om nästa gren ska köras.
- **`sleep_ms` / `set_timeout` / `set_interval`** — riktiga millisekunder (wall-clock). **`sleep_ticks`** använder scheduler-ticks.
- **`do { } while (cond)`** — kroppen körs minst en gång.
- **`for x of xs`** — itererar **värden** (som C# `foreach`, Rust `for x in &vec`). Loopen streamar via iteratorprotokollet (ingen materialisering).
- **`for await x of xs`** — async iteration; endast inuti **`async fn`**. Streamar **`Promise`-baserade** `{ value, done }`-steg.
- **`fn* name() { yield v }`** — generatorfunktion; anrop **`name()`** ger iterator. **`yield* iterable`** delegerar till annan iterable/generator; delegatens **`return`**-värde blir resultatet av **`yield*`**-uttrycket. **`return expr`** efter sista **`yield`** blir **`value`** på sista **`.next()`** med **`done: true`**.
- **`async fn* name() { yield v }`** — async generator; **`yield* asyncIterable`** delegerar över async/sync källor; **`await`** i kroppen; **`.next()` / `.return()` / `.throw()`** returnerar **`Promise<{ value, done }>`**; under aktiv **`yield*`** vidarebefordras **`.return()`** och **`.throw()`** till delegaten; konsumeras med **`for await…of`**.
- **`Iterator`** / **`AsyncIterator`** classes (not prototypes) — static **`from`**, **`fromAsync`**, **`zip`**, **`enumerate`**, **`chain`**, **`map`**, **`filter`**, **`take`**, **`skip`**, **`flatMap`**, **`dropWhile`**, **`takeWhile`**, **`pairwise`**, **`accumulate`**; iterator objects expose instance **`.map`**, **`.filter`**, **`.take`**, **`.skip`**, **`.flatMap(fn, depth?)`**, **`.dropWhile`**, **`.takeWhile`**, **`.pairwise()`**, **`.accumulate(fn, initial?)`**, **`.zip(iterable)`**, **`.enumerate()`**, **`.chain(iterable, ...)`**. **`AsyncIterator`** instance/static adapters are **native async lazy** (true **`for await…of`** streaming).
- **`iterator_map` / `iterator_filter` / `iterator_take` / `iterator_skip` / `iterator_chain` / `iterator_zip` / `iterator_enumerate` / `iterator_flat_map(it, fn, depth?)` / `iterator_drop_while` / `iterator_take_while` / `iterator_pairwise` / `iterator_accumulate(it, fn, initial?)`** — returnerar **lazy iterator-objekt** (inte array). **`flatMap`** optional **`depth`** (default **`1`**) plattar array-resultat. **`takeWhile(fn)`** yieldar tills predikatet blir falskt. **`pairwise()`** yieldar `[prev, curr]` för på varandra följande element. **`accumulate(fn, initial?)`** yieldar löpande totaler (som Python **`itertools.accumulate`**).
- **`iterator_from_async(asyncIterable)`** / **`Iterator.fromAsync(asyncIterable)`** — returnerar async iterator (som JS **`Iterator.fromAsync`**).
- **`array_from_async(iterable)`** — materialiserar async iterable till array (Promise), som JS **`Array.fromAsync`**.
- **`iterator_to_array(it)`** / **`.toArray()`** — materialiserar en iterator till array.
- **`iterator_reduce(it, fn, initial?)`** / **`.reduce(fn, initial?)`** — fold över iterator.
- **`iterator_for_each(it, fn)`** / **`.forEach(fn)`** — kör callback för varje element.
- **`iterator_find(it, fn)`** / **`.find(fn)`** — första matchande värde, annars `undefined`.
- **`iterator_find_index(it, fn)`** / **`.findIndex(fn)`** — index för första match, annars `-1`.
- **`iterator_includes(it, value)`** / **`.includes(value)`** — `true` om något element är lika med `value`.
- **`it.return(value?)`** / **`it.throw(reason?)`** — avslutar iterator tidigt; generator **`.throw(e)`** resume:ar in i **`catch`** runt **`yield`** om det finns, annars stängs generatorn. Under aktiv **`yield*`** vidarebefordras **`.return()`** / **`.throw()`** till delegaten. **`break`** / **`return`** / **`raise`** i **`for…of`** / **`for await…of`** anropar **`iterator_close`**. **`fn* gen()`** / **`async fn* gen()`**: **`gen[Symbol.iterator]()`** / **`gen[Symbol.asyncIterator]()`** returnerar generatorn själv; lazy iterator-objekt samma (**`it[Symbol.iterator]()`** → **`it`**).
- **`{ value, done }`** från **`next()`** / generatorer: **`null`** när iterationen är klar (`done: true`), inte **`undefined`** — se [TYPES.md](TYPES.md).
- **`for i in xs`** — itererar **index** (array/sträng) eller **nycklar** (objekt) — inte JS:s förvirrande `for…in`.
- **`for const x of xs`** / **`for let x of xs`** — loop-variabel med immutabilitet som `const`/`let`.

### Strängar och sammanslagning

- `"hello" + " " + name` fungerar som i JS.
- **Template literals** med interpolation: `` `Hej ${name}!` ``

### Arrayer och objekt

```kabootar
let xs = [1, 2, 3];
let first = xs[0];
xs.push(4);                    // muterar array
let ys = map(xs, (x) => x * 2)   // pilfunktion som callback
let len = xs.length;           // eller len(xs)
sort(xs); join(xs, ", ");      // array-API
parse_int("42"); floor(3.9);   // tal + Math
to_fixed(1.5, 2);              // "1.50"
"key" in u;                    // medlemskap (objekt/array/sträng)
at(xs, -1); fill(xs, 0);       // array helpers (immutabla returvärden)
str_slice("hi", 0, 1); string_includes(s, "x");
object_assign(a, b); object_has_own(o, "k");
date_iso(0);                   // ISO-8601 UTC-sträng
regex_search("foo", text);     // index eller -1

let u = { name: "Ada", age: 36 };
u.name;
u["name"];

for key in u { ... }           // nycklar på objekt
for x of xs { ... }            // värden i array (inte JS for-in)
for x of iterator_map(gen(), (n) => n * 2) { ... }  // lazy map over generator
for await x of gen() { ... }   // async fn body only
fn* counter() { yield 1; yield 2 }
async fn* stream() { yield 1 }
for i in xs { sum = sum + xs[i] }  // index 0..len-1
```

Saknas ännu: (inga planerade TLS-funktioner i nästa steg).

### TLS trust och cert pinning (v2.11)

```kabootar
// Extra CA (behåller Mozilla roots)
tls_add_ca(pem_string)

// Endast egen CA (t.ex. self-signed / intern PKI)
tls_ca_only(pem_string)

// SHA-256-pinning av leaf-cert (64 hex tecken)
let pin = tls_cert_sha256(pem_string)
tls_pin("api.example.com", pin)

tls_reset()  // tillbaka till standard trust

async fn load() {
    return await http_fetch_async("GET", "https://api.example.com/", "")
}
```

### HTTP headers (v2.12–v2.13)

```kabootar
async fn api() {
    let res = await http_fetch_async(
        "POST",
        "https://api.example.com/items",
        "{\"name\":\"Ada\"}",
        { "Content-Type": "application/json", Authorization: "Bearer tok" }
    )
    let ct = http_header(res, "content-type")
    return http_body(res)
}
```

Response headers har **lowercase**-nycklar. `http_header(res, "Content-Type")` är case-insensitive. Redirects följs automatiskt (v2.13).

### Destructuring, spread, for, try/catch (v2.3)

```kabootar
let [a, b, ...rest] = [1, 2, 3, 4]
let { name, age } = { name: "Ada", age: 36 }
let merged = { ...base, extra: 1 }
let all = [...xs, 99]

for let i = 0; i < len(xs); i = i + 1 {
    // ...
}

try {
    risky()
} catch (e) {
    e
}
```

`try`/`catch` fångar **`Result::Err`** — inte JS-undantag. `Ok(v)` unwrapas till `v`.

### `?`-operator, match-guards, `is` (v2.14)

```kabootar
fn load() {
    return step()?
}

match n {
    x if x > 0 => "positive",
    _ => "other"
}

class Dog extends Animal { }
instanceof(dog, "Animal")   // true — klass + arv
is_impl(dog, "Greeter")  // interface-check (v2.7)
```

`?` unwrapar `Ok(v)` eller returnerar `Err(e)` som `Result`. Ternary `a ? b : c` fungerar fortfarande.

Komplett lista: [FEATURES.md](FEATURES.md).

---

## Typer — det JS-utvecklare brukar snubbla på

Se [TYPES.md](TYPES.md) för detaljer. Kortversion:

### `null` vs `undefined`

Båda finns i språket — **ersätt inte** den ena med den andra.

```kabootar
let a = null;        // medvetet "inget"
let b;               // undefined — inte tilldelad än
is_null(a)           // true
is_undefined(b)      // true
null == undefined    // false  ← inte som JS ==
```

| | `null` | `undefined` |
|---|--------|-------------|
| Oinitierad `let` | nej | ja |
| Saknad objektnyckel | nej | ja |
| `==` mellan dem | `false` | `false` |

Fullständig guide: [TYPES.md](TYPES.md).

### Ingen implicit typkonvertering

```kabootar
"1" + 2    // fel (eller fel typ i +), inte "12"
"5" == 5   // false
```

### `NaN` och division

```kabootar
1 / 0      // fel: Integer division by zero
is_nan(NaN)  // true — testa flyttals-NaN explicit
```

Heltalsdivision ger heltal; `NaN` är en **flyttals**-literal, inte något som smyger in i `1 + 2`.

### Truthiness

Liknar JS: `null`, `undefined`, `false`, `0`, `""`, `NaN` är falska. **Men** du kan inte lita på implicit `"0"` → 0 i uttryck — se ovan.

### `Option` / `Result` (inte JS)

Rust-inspirerat — för explicit "kanske inget" / "kanske fel":

```kabootar
Some(42)
None
Ok("ok")
Err("failed")
```

Använd detta istället för att överallt returnera `null`/`undefined` som felkoder.

---

## Klasser — C#, inte prototyper

```javascript
// JS
class Point {
  constructor(x, y) { this.x = x; this.y = y; }
}
```

```kabootar
class Animal {
    name: string;
    fn init(n) { this.name = n }
    fn label() { return this.name }
}

class Dog extends Animal {
    breed: string;

    fn init(n, b) {
        super.init(n)
        this.breed = b
    }

    fn greet() {
        return super.greet() + "!"
    }
}
let d = Dog("Rex", "lab")
```

| | JavaScript | Kabootar |
|---|------------|----------|
| Modell | Prototyper | Klassdefinition |
| Instans | `this` | **`this`** (samma roll, annat nyckelord — undviker JS `this`) |
| Konstruktor | `constructor` | **`fn init(...)`** (konvention) |
| Arv | `extends` + prototypkedja | **`class Dog extends Animal`** (fält + metoder ärvs) |
| Super | `super()` / `super.method()` | **`super.init(...)`** / **`super.method()`** |
| Nya fält på flykt | `obj.z = 1` | deklarera fält i klassen |

Mer: [CLASSES.md](CLASSES.md).

---

## Moduler — inte ES modules / CommonJS

```javascript
// JS (ESM)
import { add } from "./math.js";
export function add(a, b) { return a + b; }
```

```kabootar
import "math";
add(1, 2);

// I lib/math.kab eller inbyggd modul:
pub fn add(a, b) {
    return a + b
}
```

| | JavaScript | Kabootar |
|---|------------|----------|
| Import | `import … from "path"` | **`import "namn";`** (sträng) |
| Export | `export` / `module.exports` | **`pub fn`** / **`pub let`** |
| Fil | `.js` / `package.json` | **`.kab`** + valfritt `kabootar.toml` |
| Version | `package.json` semver | `@version "1.0"` + `import "mod@1.0"` |

Inbyggda moduler (ingen fil behövs): `math`, `http`, `crypto`, `science`, `docai`, `codai` — se [MODULES.md](MODULES.md).

```kabootar
import "science";
import "codai";
code_util("http-route-get");
```

---

## Fullstack — inbyggt (inget npm för detta)

Som JS-utvecklare är du van vid att lägga till Express, pg, fs via npm. I Kabootar finns mycket **i språket**:

| Behov | JavaScript-värld | Kabootar |
|-------|------------------|----------|
| HTTP API | Express / Fastify | `http_fetch_async`, **`http_header`**, **`http_headers`**, **`tls_*`** — [HTTP.md](HTTP.md) |
| SQL | PostgreSQL + driver | `sql()`, **`sql_async()`** in-process — [SQL.md](SQL.md) |
| Filer (virtuellt) | `fs` | `os_read`, `os_write`, **`os_read_async`**, **`await_all`** — [OS.md](OS.md) |
| Markup | React / HTML | KML + `kdom` — [KML.md](KML.md) |

```kabootar
sql("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)");
http_route("GET", "/api/users", list_users);

pub fn list_users() {
    return http_response(200, sql("SELECT id, name FROM users"))
}
```

CLI: `kabootar serve --watch main.kab` — motsvarar ungefär `node server.js` med hot reload ([PROJECT.md](PROJECT.md)).

---

## `match` — inte `switch`

```kabootar
match x {
    1 => "one",
    [] => "empty",
    [a, b] => a + b,
    [head, ...rest] => head,
    { name: n } => n,
    { name, age } => age,
    n if n > 0 => "positive",
    _ => "other"
}
```

Rust-liknande mönstermatchning — inte JS `switch` med fall-through. Array- och objekt-mönster (v2.15).

---

## Saknas (leta inte efter detta ännu)

| JS-feature | Status i Kabootar |
|------------|-------------------|
| `async` / `await` / `Promise` | ✅ `async`/`await`, **`promise_new`**, **`promise_resolve`/`reject`**, **`promise_then`/`catch`/`finally`**, **`promise_all`/`race`/`any`/`allSettled`** |
| Pilfunktioner `=>` | ✅ v2.4 |
| Klass-arv `extends` | ✅ v2.4 |
| `super.init` / `super.method` | ✅ v2.5 |
| `var` | Medvetet borttaget |
| Destructuring | ✅ v2.3 |
| Spread `...arr` | ✅ v2.3 |
| `for (let i=0; …)` | ✅ `for let i = 0; …` (v2.3) |
| `try` / `catch` | ✅ på `Result`, inte JS-undantag (v2.3) |
| `JSON.parse` / `JSON.stringify` | ✅ `json_parse` / `json_stringify` eller `import "std"` |
| `Math.*` | ✅ `floor`, `ceil`, `round`, `abs`, `min`, `max`, `sqrt`, `pow`, `random`, `sin`/`cos`/`tan`, `asin`/`acos`/`atan`/`atan2`, `log`, `hypot`, `cbrt`, `fmod`, `imul`, `clz32`, `fround`, … |
| `parseInt` / `parseFloat` | ✅ `parse_int`, `parse_float`, `number_to_string` |
| `Array.sort` / `join` / `splice` | ✅ `sort`, `to_sorted`, `join`, `splice`, `reverse`, `to_reversed`, `reduce_right`, `shift`, `unshift` |
| `Set` union/intersection | ✅ `set_union`, `set_intersection`, `set_difference`, `set_is_subset`, `set_is_superset`, `set_is_disjoint` |
| `Array.flat` / alias | ✅ `flat`, `array_flat`, `array_flat_map`, `array_includes`, … |
| `Object.is` / `hasOwn` | ✅ `object_is`, `object_has`, `object_delete`, `object_keys`, … |
| `Object.assign` | ✅ `assign` |
| `"key" in obj` | ✅ `in`-operator + `has_key` |
| `Promise.all` / `race` / `any` / `allSettled` | ✅ `promise_all`, `promise_race`, `promise_any`, `promise_all_settled`, `promise_resolve`, `promise_then` |
| `console.log` | ✅ `log` / `println` |
| `fetch` | ✅ **`fetch`**, **`response_text`**, **`response_json`**, **`response_ok`** (eller `http_fetch_async`) |
| `queueMicrotask` | ✅ **`queue_microtask`** |
| `?.` optional chaining | ✅ **`obj?.x`**, **`obj?.[i]`**, **`fn?.()`** |
| `delete obj.key` | ✅ **`delete o.x`** |
| Prototyparv / `Object.assign` | Använd `class` eller `{ key: value }` |
| `eval()` | Medvetet borttaget |

Se [FEATURES.md](FEATURES.md) för full matris (✅ / 🚧 / ❌).

---

## Verktyg för JS-utvecklare

| Verktyg | Beskrivning |
|---------|-------------|
| [IDE.md](IDE.md) | VS Code & Cursor — samma extension, LSP, CodAI |
| [CODAI.md](CODAI.md) | Kodsnippets som Tailwind — `code_util("http-route-get")` |
| [DOCAI.md](DOCAI.md) | Fråga dokumentationen — som att fråga MDN, men för Kabootar |

```kabootar
import "docai";
doc_ask("skillnad null undefined");
```

---

## När du *ska* läsa annan dokumentation

| Du vill… | Läs |
|----------|-----|
| Bara JS-skillnader | **Detta dokument** |
| Full syntaxreferens | [LANGUAGE.md](LANGUAGE.md) |
| `null` / `undefined` / `NaN` | [TYPES.md](TYPES.md) |
| Projekt, CLI, hot reload | [PROJECT.md](PROJECT.md) |
| WASM från riktig JS | Avsnittet nedan |

---

## WASM — anropa Kabootar från JavaScript

Det enda stället där du faktiskt kör Kabootar *inuti* JS:

```javascript
import init, { evaluate } from './pkg/kabootar.js';
await init();
evaluate('fn add(a,b){ return a+b }; add(2,3)'); // "5"
```

Kabootar-källan i `.kab` är **inte** JS — kompilera/transpilera till WASM separat.

---

## Snabb checklista vid porting från JS

1. Byt `function` → `fn`; `const` fungerar, ta bort `var`.
2. Byt `import/export` → `import "mod"` + `pub fn`.
3. Ta bort `===`; behåll `==`.
4. Ersätt `express` + `pg` med `http_route` + `sql()` där det räcker.
5. Förvänta dig **fel** vid odekvarerade variabler och typmix i `+`.
6. Kör i VS Code/Cursor med LSP — [IDE.md](IDE.md).

*(Klasser använder `this` för instansen — inte JavaScript `this` eller prototypkedja.)*

---

*Övrig dokumentation: [README.md](README.md)*
