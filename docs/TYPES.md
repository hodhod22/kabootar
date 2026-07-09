# Kabootar — typer: null, undefined och NaN

**Status:** Både `null` och `undefined` är **officiella, separata** värden i lexer, parser, bytecode och runtime. Ersätt inte `undefined` med `null` i ny kod — de betyder olika saker och `null == undefined` är **`false`**.

## Problemet med JavaScript

I JavaScript blandas ofta:

- `null` och `undefined` i jämförelser (`==`)
- `undefined` från saknade properties och oinitierade variabler
- `NaN` som tyst sprids genom all aritmetik

Kabootar löser detta med **tydliga, separata värden** och **explicit felhantering**.

## `null`

**Betydelse:** Medvetet "inget värde" — som SQL `NULL` eller att du valt att representera tomhet.

```kabootar
let user = null;
is_null(user)   // true
```

- `null == null` → `true`
- `null == undefined` → **`false`** (till skillnad från JS `==`)

## `undefined`

**Betydelse:** Värdet saknas — bindingen finns men är oinitierad, eller en nyckel/index finns inte.

```kabootar
let x;
is_undefined(x)  // true
x = 5;
is_undefined(x)  // false

let m = { "a": 1 };
is_undefined(m["b"])   // true — saknad nyckel
is_undefined(m["a"])   // false
```

Att läsa en **odeklarerad** variabel (`y` utan `let y`) ger **runtime-fel**, inte `undefined`.

## När ska jag använda vilket?

| Situation | Använd |
|-----------|--------|
| Medvetet “inget värde” / SQL-liknande tomhet | `null` |
| `let x;` utan init | `undefined` (automatiskt) |
| Saknad objektnyckel / array-index | `undefined` (automatiskt) |
| Valfritt fält i AST/objekt (self-host) | jämför med `undefined`, inte `null` |
| API som uttryckligen returnerar tomhet | `null` om kontraktet säger det |

**Praktisk regel:** om du testar “finns fältet?” → `x == undefined` eller `x != undefined`. Om du sätter “tomt med flit” → `null`.

## Sanning (truthiness)

| Värde | Sanning |
|-------|---------|
| `null` | falskt |
| `undefined` | falskt |
| `false` | falskt |
| `0` | falskt |
| `""` | falskt |
| `NaN` | falskt |
| övrigt | sant |

`if` och `while` använder `is_truthy()` — ingen implicit sträng→tal-konvertering.

## `NaN` (Rust-inspirerat)

| Regel | Kabootar |
|-------|----------|
| Heltalsdivision med 0 | **Fel** (`Integer division by zero`) |
| Heltal + heltal | Alltid heltal, aldrig `NaN` |
| `NaN` literal | Endast `Float`-typ |
| Jämförelse med `NaN` | Använd `is_nan(x)` |

```kabootar
is_nan(NaN)     // true
1 / 0           // fel, inte Infinity/NaN
```

Flyttalsliteraler (`3.14`) och `NaN` representeras som `Value::Float`.

```kabootar
let pi = 3.14;
is_nan(NaN)     // true
is_nan(3.14)    // false
1 / 0           // fel, inte Infinity/NaN
```

## `Option` och `Result`

Rust-liknande konstruktioner finns kvar för explicit hantering:

```kabootar
Some(42)
None
Ok(1)
Err("failed")
```

Använd `Option`/`Result` när operationer kan misslyckas — inte `null` som universal-felvärde.
