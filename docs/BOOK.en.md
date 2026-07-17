# The Kabootar Language — A Complete Reference

## Preface

Kabootar is a fullstack systems and application language with JavaScript-like syntax, explicit typing behavior, and a large runtime covering everything from HTTP/SQL to workers, crypto, and self-hosting. This book describes the language as it exists in the `nova-interpreter` implementation.

The language is intentionally not JavaScript: it removes implicit coercion, `var` hoisting, prototype inheritance, and `eval`. In return it offers predictable types, Rust-style `match`, C#-style classes, modules, async/generators, and a self-hosted compiler pipeline.

This reference is organized into a core language tutorial (Chapters 1–10), a runtime guide (Chapters 11–13), and a compiler/self-hosting guide (Chapter 14).

---

## Chapter 1 — Introduction

### 1.1 What is Kabootar?

Kabootar is a general-purpose language that compiles to an internal bytecode format (`.kbc`) and runs on a Rust-based VM. It has three major design goals:

1. **Familiar syntax** — C/Rust/JavaScript-like tokens and blocks.
2. **Predictable types** — no silent coercion; `null`, `undefined`, `NaN`, and `Result` are explicit.
3. **Batteries-included runtime** — modules for SQL, HTTP, OS access, crypto, science, DOM rendering, and more.

### 1.2 Hello world

```kabootar
println("Hello, Kabootar!")
```

Run with:

```bash
kabootar hello.kab
```

### 1.3 File extension and module entry

Source files use `.kab`. The compiler/interpreter loads `main.kab` by default or the `entry` field from `kabootar.toml`.

---

## Chapter 2 — Lexical Structure and Types

### 2.1 Tokens and whitespace

- Identifiers: ASCII letters, digits, and `_`, not starting with a digit.
- Keywords: `fn`, `let`, `const`, `if`, `else`, `while`, `for`, `in`, `of`, `return`, `break`, `continue`, `throw`, `raise`, `try`, `catch`, `finally`, `pass`, `assert`, `with`, `using`, `match`, `switch`, `case`, `default`, `fallthrough`, `do`, `async`, `await`, `yield`, `fn*`, `class`, `extends`, `interface`, `implements`, `enum`, `import`, `pub`, `this`, `super`, `true`, `false`, `null`, `undefined`, `NaN`, `Some`, `None`, `Ok`, `Err`, `is`, `not`, `delete`.
- Operators: `+ - * / % **`, `== != < <= > >=`, `&& || ??`, `! ~`, `& | ^ << >> >>>`, `? :`, `=>`, `= += -= *= /= %= **=`.
- Comments: `// line comment` and `/* block comment */`.
- Semicolons are optional between statements; newlines separate statements inside blocks.

### 2.2 Primitive types

| Type | Literal | Notes |
|------|---------|-------|
| Integer | `42`, `-7` | 64-bit signed. |
| Float | `3.14`, `NaN` | 64-bit IEEE 754. |
| BigInt | `123n`, `BigInt("99")` | Arbitrary precision integer. No arithmetic mix with `number`. |
| String | `"text"`, `` `template ${x}` `` | UTF-8. Templates support `${expr}` interpolation. |
| Boolean | `true`, `false` | |
| Null | `null` | Explicit "no value". |
| Undefined | `undefined` | Uninitialized binding or missing key/index. |

### 2.3 Composite types

- **Array**: `[1, 2, 3]`.
- **Object**: `{ "a": 1, b: 2 }`.
- **Map/Set**: Created via `map_new` / `set_new`.
- **Class instances**: Created from `class` definitions.
- **Result**: `Ok(v)`, `Err(e)`.
- **Option**: `Some(v)`, `None`.

### 2.4 Truthiness

Falsy values: `null`, `undefined`, `false`, `0`, `""`, `NaN`. Everything else is truthy.

### 2.5 `null` vs `undefined`

- `null` = deliberate absence.
- `undefined` = uninitialized or missing.
- `null == undefined` is `false` in Kabootar.

---

## Chapter 3 — Variables and Constants

### 3.1 `let` and `const`

```kabootar
let x = 10
x = 20               // ok

const PI = 3.14
PI = 3               // runtime error

let y                // undefined
println(y)           // prints "undefined"
```

There is no `var`.

**Example: swapping values**

```kabootar
let a = 1
let b = 2
let tmp = a
a = b
b = tmp
println(a)           // 2
println(b)           // 1
```

### 3.2 Destructuring

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

Block-scoped like JavaScript `let`. Reading an undeclared variable is a runtime error, not `undefined`.

```kabootar
let outer = 10
if true {
    let inner = 20
    println(outer)   // 10
}
// println(inner)    // runtime error: undefined variable
```

---

## Chapter 4 — Expressions and Operators

### 4.1 Arithmetic

```kabootar
println(1 + 2)          // 3
println(3 - 4)          // -1
println(5 * 6)          // 30
println(7 / 8)          // 0.875 (float division)
println(9 % 2)          // 1
println(2 ** 3 ** 2)    // 512 (right-associative)
```

Integer division by zero is an error. Float `NaN` is explicit via the `NaN` literal or float math.

**Example: area of a circle**

```kabootar
const PI = 3.14159
let r = 5
let area = PI * r * r
println(area)           // 78.53975
```

### 4.2 Comparison

```kabootar
println(1 == 1)                  // true
println(1 != 2)                  // true
println(3 < 5)                   // true
println(5 <= 5)                  // true
println("5" == 5)                // false (strict per type, no coercion)
println(null == undefined)       // false
```

### 4.3 Logical

```kabootar
let a = true
let b = false
println(a && b)          // false
println(a || b)          // true
println(!a)              // false
println(0 ?? 9)          // 0
println(null ?? 9)       // 9
println(undefined ?? 5)  // 5
```

### 4.4 Bitwise

```kabootar
println(5 & 3)           // 1  (0101 & 0011)
println(5 | 3)           // 7  (0101 | 0011)
println(5 ^ 3)           // 6  (0101 ^ 0011)
println(~5)              // -6
println(1 << 3)          // 8
println(8 >> 2)          // 2
println(-8 >>> 2)        // 1073741822
```

Applied with 32-bit integer semantics (`ToInt32` / `ToUint32`).

### 4.5 Ternary

```kabootar
let n = -5
let sign = n > 0 ? "positive" : "non-positive"
println(sign)            // "non-positive"

let age = 20
let label = age >= 18 ? "adult" : "minor"
println(label)           // "adult"
```

### 4.6 `?` on Result

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

`?` unwraps `Ok(v)` or returns `Err(e)` from the current function.

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

### 4.9 Member and index access

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

### 4.10 Assignment expressions

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
let obj = { a: 1, b: 2 }
delete obj.a
println("a" in obj)      // false
println("b" in obj)      // true
```

---

## Chapter 5 — Control Flow

### 5.1 `if` / `else`

Parentheses around the condition are optional:

```kabootar
let x = -5
if x < 0 {
    println("negative")
} else if x == 0 {
    println("zero")
} else {
    println("positive")
}
// prints "negative"
```

**Example: grade classifier**

```kabootar
let score = 85
let grade
if score >= 90 {
    grade = "A"
} else if score >= 80 {
    grade = "B"
} else if score >= 70 {
    grade = "C"
} else {
    grade = "F"
}
println(grade)   // "B"
```

### 5.2 `while`

```kabootar
let i = 0
while i < 5 {
    println(i)   // 0, 1, 2, 3, 4
    i = i + 1
}

// sum 1..10
let sum = 0
let n = 1
while n <= 10 {
    sum = sum + n
    n = n + 1
}
println(sum)     // 55
```

`break` and `continue` are supported.

### 5.3 `do … While`

```kabootar
let i = 0
let sum = 0
do {
    sum = sum + i
    i = i + 1
} while i < 5
println(sum)     // 10 (0+1+2+3+4)
```

### 5.4 `for` loops

```kabootar
let xs = ["a", "b", "c"]

// C-style
for let i = 0; i < len(xs); i = i + 1 {
    println(xs[i])   // a, b, c
}

// values
for x of xs {
    println(x)       // a, b, c
}

// indices/keys
for i in xs {
    println(i)       // 0, 1, 2
}

let obj = { name: "Ada", age: 36 }
for key in obj {
    println(key)     // "name", "age"
}
```

`for const x of xs` and `for let x of xs` control loop-variable mutability.

### 5.5 `match`

```kabootar
let n = 3
let desc = match n {
    0 => "zero",
    x if x > 0 => "positive",
    _ => "negative"
}
println(desc)        // "positive"
```

**Example: matching Option**

```kabootar
let opt = Some(42)
let value = match opt {
    Some(x) => x,
    None => 0
}
println(value)       // 42
```

**Example: matching arrays**

```kabootar
let pair = [1, 2]
match pair {
    [a, b] => println(a + b),   // 3
    _ => println("other")
}
```

Patterns include: literal, variable, wildcard `_`, `Some`/`None`, `Ok`/`Err`, enum variants, arrays, objects, and guards (`if expr =>`).

### 5.6 `if let` / `while let`

```kabootar
let opt = Some(7)
if let Some(x) = opt {
    println(x)       // 7
}

let r = Ok(1)
if let Ok(v) = r {
    println(v)       // 1
}

let results = [Ok(1), Ok(2), Err("done")]
let i = 0
while let Ok(v) = results[i] {
    println(v)
    i = i + 1
}
// prints 1, 2
```

### 5.7 `switch`

```kabootar
let x = 2
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
// prints "two or three"
```

No implicit fall-through; use `fallthrough` explicitly.

### 5.8 `try` / `catch` / `finally`

```kabootar
fn divide(a, b) {
    if b == 0 {
        return Err("division by zero")
    }
    return Ok(a / b)
}

try {
    let result = divide(10, 0)?
    println(result)
} catch (e) {
    println("caught: " + e)   // "caught: division by zero"
} finally {
    println("cleaning up")
}
```

`try`/`catch` operates on `Result` values: `Ok(v)` unwraps to `v`, `Err(e)` enters the catch block.

### 5.9 `pass`, `assert`, `with`, `using`

```kabootar
pass                            // no-op

assert(1 + 1 == 2, "math works")

// with runs cleanup when the block ends
with open_resource() as r {
    use(r)
}

// using disposes a value explicitly
using file = open_file("log.txt");
// file is disposed here
```

---

## Chapter 6 — Functions

### 6.1 Named functions

```kabootar
fn add(a, b) {
    return a + b
}
println(add(2, 3))       // 5

pub fn exported(a) {            // exported from module
    return a
}
```

### 6.2 Arrow functions

```kabootar
let double = (x) => x * 2
println(double(4))        // 8

let sum = (a, b) => {
    return a + b
}
println(sum(1, 2))        // 3
```

### 6.3 Parameters

```kabootar
fn greet(name = "world") {
    return "Hello, " + name
}
println(greet())          // "Hello, world"
println(greet("Kabootar")) // "Hello, Kabootar"

fn sum(...xs) {
    let total = 0
    for x of xs {
        total = total + x
    }
    return total
}
println(sum(1, 2, 3, 4))  // 10

fn f(a, b = 2, ...rest) {
    return a + b + len(rest)
}
println(f(1))             // 3
println(f(1, 5, "x", "y")) // 8
```

### 6.4 Async functions and `await`

```kabootar
async fn fetch() {
    return 42
}

async fn main() {
    let n = await fetch()
    println(n)            // 42
    return n
}
```

`await` only works inside `async fn` or async arrows. It drains the microtask queue FIFO.

### 6.5 Generators

```kabootar
fn* counter() {
    yield 1
    yield 2
    return 99
}

let it = counter()
println(it.next().value)  // 1
println(it.next().value)  // 2
println(it.next().value)  // 99

fn* flatten() {
    yield 1
    yield* [2, 3]
    yield 4
}
let f = flatten()
for x of f {
    println(x)            // 1, 2, 3, 4
}
```

`yield*` delegates to another iterable/generator. Async generators use `async fn*`.

### 6.6 Higher-order functions

Functions are first-class and can be passed, returned, and stored in variables.

```kabootar
fn apply(f, x) {
    return f(x)
}
let inc = (n) => n + 1
println(apply(inc, 5))    // 6

fn makeMultiplier(k) {
    return (x) => x * k
}
let triple = makeMultiplier(3)
println(triple(4))          // 12
```

### 6.7 `return`

`return` is optional at the end of a function; the last expression value is returned automatically if there is no explicit `return`.

```kabootar
fn square(x) {
    x * x    // implicit return
}
println(square(3))        // 9
```
---

## Chapter 7 — Objects, Arrays, and Classes

### 7.1 Arrays

```kabootar
let xs = [1, 2, 3]
xs.push(4)                     // mutates
println(xs)                    // [1, 2, 3, 4]

let ys = map(xs, (x) => x * 2)
println(ys)                    // [2, 4, 6, 8]

let evens = filter(xs, (x) => x % 2 == 0)
println(evens)                 // [2, 4]

let sum = reduce(xs, (acc, x) => acc + x, 0)
println(sum)                   // 10

println(len(xs))               // 4
println(includes(xs, 3))       // true
println(at(xs, -1))            // 4
```

Array helpers include `map`, `filter`, `reduce`, `find`, `slice`, `sort`, `reverse`, `join`, `includes`, `some`, `every`, `index_of`, `flat`, `flat_map`, `at`, `fill`, `to_spliced`, `to_reversed`, `to_sorted`, `shift`, `unshift`, `splice`, `concat`.

### 7.2 Objects

```kabootar
let u = { name: "Ada", age: 36 }
println(u.name)                // "Ada"
println(u["name"])             // "Ada"

let keys = object_keys(u)
println(keys)                  // ["name", "age"]

let values = object_values(u)
println(values)                // ["Ada", 36]

let copy = object_assign({}, u)
copy.city = "Stockholm"
println(copy)                  // { name: "Ada", age: 36, city: "Stockholm" }
```

Object helpers include `assign`, `has_key`, `delete_prop`, `keys`, `values`, `entries`, `from_entries`, `clone_shallow`, `group_by`.

### 7.3 Classes

Kabootar classes are C#-style, not JavaScript prototypes.
Method receiver is **`this`**. The keyword **`self`** is reserved for a future `struct` (Rust-style).

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

### 7.4 Inheritance

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

    fn label() {
        return super.label() + " (" + this.breed + ")"
    }
}

let d = Dog("Rex", "lab")
println(d.label())             // "Rex (lab)"
```

### 7.5 Interfaces

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

let p = Person()
p.name = "Ada"
println(p.greet())             // "hi Ada"
println(is_impl(p, "Greeter")) // true
```

### 7.6 Enums

```kabootar
enum Color { Red, Green }
enum Msg { Move(x, y), Stop }

let c = Color.Red
let description = match c {
    Color.Red => "red",
    Color.Green => "green",
    _ => "other"
}
println(description)           // "red"

let m = Msg.Move(3, 4)
match m {
    Msg.Move(x, y) => println(x + y),   // 7
    Msg.Stop => println("stopped")
}
```

### 7.7 Private fields

```kabootar
class Counter {
    #n: number = 0;

    fn inc() {
        this.#n = this.#n + 1
    }

    fn get() {
        return this.#n
    }
}

let c = Counter()
c.inc()
c.inc()
println(c.get())               // 2
// println(c.#n)               // error: private field
```

---

## Chapter 8 — Pattern Matching

### 8.1 Patterns

A pattern can be:

- Literal: `1`, `"x"`, `true`, `null`, `undefined`, `NaN`
- Variable: `x`
- Wildcard: `_`
- `Some(p)`, `None`
- `Ok(p)`, `Err(p)`
- Enum variant: `Color.Red`, `Msg.Move(x, y)`
- Array: `[a, b, ...rest]`
- Object: `{ name, age: a }`

**Example: matching on Option**

```kabootar
let opt = Some(7)
match opt {
    Some(x) => println(x),       // 7
    None => println("none")
}
```

**Example: matching arrays and objects**

```kabootar
let pair = [1, 2]
match pair {
    [a, b] => println(a + b),    // 3
    _ => println("other")
}

let user = { name: "Ada", age: 36 }
match user {
    { name: n, age: a } => println(n + " is " + a),
    _ => println("other")
}
```

### 8.2 Match arms with guards

```kabootar
let n = 5
let desc = match n {
    0 => "zero",
    x if x > 0 => "positive",   // guard
    _ => "negative"
}
println(desc)                  // "positive"
```

### 8.3 `if let` / `while let`

```kabootar
let opt = Some(7)
if let Some(x) = opt {
    println(x)                   // 7
}

let results = [Ok(1), Ok(2), Err("done")]
let i = 0
while let Ok(v) = results[i] {
    println(v)                   // 1, 2
    i = i + 1
}
```

---

## Chapter 9 — Modules

### 9.1 Import syntax

```kabootar
import "math"
println(add(1, 2))             // 3

import "greet"
greet("Kabootar")
```

Imports bind names into the current environment.

### 9.2 Built-in modules

| Module | Contents |
|--------|----------|
| `std` | JSON, type helpers |
| `json` | `parse`, `dump` |
| `collections` | Map/Set helpers |
| `strings` | string utilities |
| `math` | basic arithmetic helpers |
| `http` | HTTP routing and fetching |
| `crypto` | cryptographic functions |
| `science` | physics, chemistry, statistics |
| `docai` | documentation AI helpers |
| `codai` | code AI helpers |
| `sql` | in-process SQL database |
| `os` | sandboxed filesystem |

**Example: using built-in modules**

```kabootar
import "json"
import "math"

let obj = json_parse('{"x":3,"y":4}')
println(obj.x)                 // 3
println(add(obj.x, obj.y))     // 7
```

### 9.3 File modules

Create `lib/greet.kab`:

```kabootar
pub fn greet(name) {
    return "Hello, " + name
}

fn secret() { }             // private
```

Only `pub fn`, `pub let`, and `pub const` are exported.

### 9.4 Project structure

```toml
# kabootar.toml
version = "0.1.0"
entry = "main.kab"
port = 8080

[dependencies]
greet = "1.0.0"
```

**Example: `main.kab`**

```kabootar
import "greet"

fn main() {
    println(greet("Kabootar"))
}

main()
```

### 9.5 Versioned imports

```kabootar
import "greet@1.0"
```

### 9.6 Local package registry

```bash
kabootar publish lib/greet.kab
kabootar install greet@1.0
kabootar install
```

Programmatic access:

```kabootar
registry_publish("lib/greet.kab")
registry_install("greet", "1.0")
let mods = registry_list()
println(mods)                  // [{ name: "greet", version: "1.0.0" }, ...]
```

### 9.7 `import.meta` and dynamic import

```kabootar
println(import.meta.url)       // current module URL

async fn loadMath() {
    let math = await import("math")
    println(math.add(2, 3))    // 5
}
```

---

## Chapter 10 — Error Handling

### 10.1 `Result` and `Option`

```kabootar
let r = Ok(42)
println(r)                     // Ok(42)

let e = Err("failed")
println(e)                     // Err("failed")

let opt = Some(99)
println(opt)                   // Some(99)

let empty = None
println(empty)                 // None
```

### 10.2 `?` operator

```kabootar
fn read_int(s) {
    let n = parse_int(s)
    if is_nan(n) {
        return Err("not a number")
    }
    return Ok(n)
}

fn double_parse(s) {
    let n = read_int(s)?
    return n * 2
}

println(double_parse("21"))    // 42
// double_parse("x") returns Err("not a number")
```

### 10.3 `throw` / `raise`

```kabootar
fn validate(x) {
    if x < 0 {
        throw "negative value"
    }
    return x
}

try {
    validate(-1)
} catch (e) {
    println(e)                 // "negative value"
}
```

Caught with `try`/`catch`.

### 10.4 `Error` helpers

```kabootar
let err = error_new("parse failed", { cause: reference_error("missing field") })
println(error_stack(err))
println(error_cause(err))

fn require_number(x) {
    if typeof(x) != "number" {
        return type_error("expected number")
    }
    return Ok(x)
}
```

---

## Chapter 11 — Async Programming and Generators

### 11.1 Promises

```kabootar
let p = promise_new((resolve, reject) => {
    resolve(42)
})

promise_then(p, (v) => v * 2)
promise_all([p1, p2])
await_all([p1, p2])             // returns array synchronously
```

**Example: promise chain**

```kabootar
let p = promise_new((resolve, reject) => {
    set_timeout(() => resolve(10), 10)
})

let q = promise_then(p, (v) => v * 2)
let r = promise_then(q, (v) => println(v))   // 20
```

**Example: awaiting multiple promises**

```kabootar
async fn load() {
    let a = promise_resolve(1)
    let b = promise_resolve(2)
    let values = await_all([a, b])
    println(values)              // [1, 2]
}
```

### 11.2 Async iterators

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

### 11.3 Iterator helpers

Lazy iterator adapters:

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

**Example: lazy pipeline**

```kabootar
fn* numbers() {
    let i = 1
    while i <= 100 {
        yield i
        i = i + 1
    }
}

let evens = iterator_filter(numbers(), (n) => n % 2 == 0)
let firstFive = iterator_take(evens, 5)
println(firstFive.toArray())   // [2, 4, 6, 8, 10]
```

Consume with `for … of`, `.toArray()`, `.reduce()`, `.find()`, etc.

---

## Chapter 12 — Standard Library

### 12.1 Math

```kabootar
println(floor(3.9))        // 3
println(ceil(3.1))         // 4
println(round(3.5))        // 4
println(sqrt(16))          // 4
println(pow(2, 10))        // 1024
println(pi())                // 3.14159...
println(sin(pi() / 2))      // 1
println(hypot(3, 4))         // 5
println(clamp(10, 0, 5))     // 5
```

`floor`, `ceil`, `round`, `abs`, `min`, `max`, `sqrt`, `pow`, `random`, `sign`, `trunc`, `clamp`, `pi`, `e`, `log`, `log2`, `log10`, `exp`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `hypot`, `cbrt`, `imul`, `clz32`, `fround`, `fmod`, `log1p`, `expm1`.

### 12.2 Number formatting

```kabootar
println(parse_int("42"))         // 42
println(parse_float("3.14"))       // 3.14
println(is_nan(NaN))              // true
println(is_integer(4.0))            // true
println(to_fixed(1.5, 2))           // "1.50"
println(to_exponential(1234, 2))  // "1.23e+3"
println(number_to_string(255, 16)) // "ff"
```

`parse_int`, `parse_float`, `is_finite`, `is_integer`, `is_safe_integer`, `to_fixed`, `to_exponential`, `to_precision`, `number_to_string`.

### 12.3 String

```kabootar
let s = "  hello world  "
println(trim(s))                  // "hello world"
println(split(s, " "))             // ["", "", "hello", "world", "", ""]
println(replace("a-b-c", "-", ":")) // "a:b-c"
println(repeat("x", 3))            // "xxx"
println(pad_start("7", 3, "0"))    // "007"
println(char_at("abc", 1))         // "b"
println(starts_with("hello", "he")) // true
```

`len`, `trim`, `trim_start`, `trim_end`, `split`, `replace`, `replace_all`, `repeat`, `pad_start`, `pad_end`, `char_at`, `char_code_at`, `from_char_code`, `code_point_at`, `from_code_point`, `starts_with`, `str_includes`, `str_last_index_of`, `string_split`, `string_to_lower`, `string_to_upper`, `string_replace`, `string_replace_all`.

### 12.4 JSON

```kabootar
let obj = json_parse('{"name":"Ada","age":36}')
println(obj.name)                  // "Ada"

let text = json_stringify(obj, 2)
println(text)
// {
//   "name": "Ada",
//   "age": 36
// }
```

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

### 12.6 Date

```kabootar
let now = date_now()
println(now)                       // timestamp in ms

let d = date_new()
println(date_get_full_year(d))     // e.g. 2026
println(date_to_iso_string(d))     // e.g. "2026-07-11T19:29:00.000Z"
```

`date_now`, `date_parse`, `date_format`, `date_iso`, `date_new`, `date_get_time`, `date_get_full_year`, `date_to_iso_string`.

### 12.7 Timers

```kabootar
set_timeout(() => println("later"), 100)

let id = set_interval(() => println("tick"), 1000)
// clear_timeout(id) or clear_interval(id) to stop

let start = performance.now()
sleep_ms(10)
let elapsed = performance.now() - start
println(elapsed)                   // ~10
```

`sleep_ms`, `sleep_ticks`, `set_timeout`, `clear_timeout`, `set_interval`, `clear_interval`, `performance.now()`.

### 12.8 URI encoding

```kabootar
let url = "https://example.com/?q=hello world"
println(encode_uri_component("hello world")) // "hello%20world"
println(decode_uri_component("hello%20world")) // "hello world"
```

`encode_uri`, `decode_uri`, `encode_uri_component`, `decode_uri_component`.

### 12.9 Collections

```kabootar
let m = map_new()
map_set(m, "key", 42)
println(map_get(m, "key"))         // 42
println(map_has(m, "missing"))     // false

let s = set_new()
set_add(s, 1)
set_add(s, 2)
set_add(s, 1)                      // ignored
println(set_has(s, 2))             // true
println(set_values(s))              // [1, 2]

let a = [1, 2, 3, 4]
let groups = map_group_by(a, (n) => n % 2 == 0 ? "even" : "odd")
println(groups)                    // { odd: [1, 3], even: [2, 4] }
```

`map_new`, `map_get`, `map_set`, `map_has`, `map_delete`, `map_clear`, `map_keys`, `map_values`, `map_entries`, `map_for_each`, `map_from_entries`, `map_group_by`.

`set_new`, `set_add`, `set_has`, `set_delete`, `set_clear`, `set_values`, `set_for_each`, `set_union`, `set_intersection`, `set_difference`, `set_symmetric_difference`, `set_is_subset`, `set_is_superset`, `set_is_disjoint`.

`weak_map_new`, `weak_set_new`.

### 12.10 Typed arrays and buffers

```kabootar
let buf = array_buffer_new(8)
let f64 = float64_array_new(buf)
float64_array_set(f64, 0, 3.14)
println(float64_array_get(f64, 0)) // 3.14

let u8 = uint8_array_new(buf)
println(uint8_array_get(u8, 0))      // byte value
```

`array_buffer_new`, `float64_array_new/get/set`, `data_view_new`, `uint8_array_new/get/set`, `int32_array_new/get/set`, `sab_new`, `sab_byte_length`, `sab_transfer`.

### 12.11 Atomics

`atomics_load`, `atomics_store`, `atomics_add`, `atomics_sub`, `atomics_and`, `atomics_or`, `atomics_xor`, `atomics_exchange`, `atomics_compare_exchange`, `atomics_wait`, `atomics_notify`.

### 12.12 Text encoding

`text_encode`, `text_decode`, `btoa`, `atob`.

### 12.13 Crypto

```kabootar
let bytes = [0, 0, 0, 0, 0, 0, 0, 0]
crypto.getRandomValues(bytes)
println(bytes)                    // e.g. [123, 45, ...]
```

`crypto.getRandomValues(array)`.

### 12.14 Environment

```kabootar
env_set("APP_NAME", "MyApp")
println(env_get("APP_NAME"))       // "MyApp"
println(env_has("PATH"))           // true or false

println(cwd())                     // current working directory
let globals = globalThis()
println("APP_NAME" in globals)       // true
```

`env_get`, `env_set`, `env_has`, `env_delete`, `env_to_object`, `cwd`, `globalThis()`.

### 12.15 Type checks

```kabootar
println(typeof(42))                 // "number"
println(typeof([]))                  // "array"
println(is_null(null))              // true
println(is_undefined(undefined))      // true
println(is_array([1, 2]))           // true
println(is_nan(NaN))                // true

type_assert(42, "number")           // ok
// type_assert("x", "number")       // error
```

`typeof()`, `is_null`, `is_undefined`, `is_nan`, `is_array`, `is_promise`, `is_error`, `is_regexp`, `is_proxy`, `is_weakmap`, `is_weakset`, `type_assert`.

---

## Chapter 13 — Runtime Environment

### 13.1 CLI commands

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

`sql_async`, KV-store via `open_kv`, `kv_get`, `kv_set`, `kv_list`, `kv_atomic`, `kv_watch`, `kv_enqueue`, `kv_dequeue`.

### 13.4 OS and filesystem

```kabootar
import "os"

os_mkdir("/data")
os_write("/data/log.txt", "hello")
let text = os_read("/data/log.txt")
```

Async variants: `os_read_async`, `os_write_async`.

### 13.5 Workers

```kabootar
let w = worker_new()
worker_start(w)
worker_post_message(w, "hello")
let msg = worker_recv(w)
worker_terminate(w)
```

In worker context: `onmessage(handler)`, `postMessage(msg)`, `worker_run_message_loop()`.

### 13.6 Streams

```kabootar
let s = stream_from_array([1, 2, 3])
let reader = stream_get_reader(s)
let chunk = reader_read(reader)
println(chunk)                     // 1

let writable = writable_stream_new()
let writer = writable_get_writer(writable)
writer_write(writer, "hello")
writer_close(writer)
```

`stream_new`, `stream_from_array`, `stream_read`, `stream_read_all`, `stream_pipe_to`, `stream_tee`, `stream_transfer`, `byte_stream_new`, `transform_stream_new`, `reader_read`, `writer_write`.

### 13.7 WebSocket

```kabootar
let ws = ws_connect("wss://echo.example.com/")
ws_send(ws, "hello")
let msg = ws_recv(ws)
println(msg)                       // "hello"
ws_close(ws)

let server = tcp_listen("127.0.0.1:9000")
let client = tcp_accept(server)
```

`ws_channel_pair`, `ws_link`, `ws_send`, `ws_recv`, `ws_connect`, `tcp_listen`, `tcp_connect`, `tcp_accept`, `udp_bind`, `udp_send`, `udp_recv`.

### 13.8 DOM and rendering

```kabootar
let ui = html! {
    div {
        h1 { "Hello" }
        p { "Kabootar" }
    }
}
kdom.render(ui)

let canvas = kdom.createCanvas(800, 600)
canvas_draw_rect(canvas, 10, 10, 100, 50)
```

`kdom` global and `html! { }` macro for Kv8 UI. Canvas 2D API via `canvas_*` natives. KML parsing via `kml("...")`.

### 13.9 FFI and npm

```kabootar
let lib = ffi_load("mylib.dll")
ffi_call(lib, "add", [1, 2])

npm_install("lodash")
npm_import("lodash")
```

TypeScript stripping: `ts_strip_types`, `ts_transpile`, `ts_compile`.

### 13.10 Node.js compatibility modules

```kabootar
import "node:fs"
import "node:path"

let text = readFileSync("input.txt")
writeFileSync("output.txt", text)
println(join("/home", "user", "file.txt"))
```

`import "node:fs"`, `"node:path"`, `"node:process"`, `"node:os"`, `"node:url"`, `"node:crypto"`, `"node:buffer"`.

### 13.11 `Intl` and `Temporal`

```kabootar
let fmt = Intl.NumberFormat("sv-SE", { style: "currency", currency: "SEK" })
println(fmt.format(1234.5))        // "1 234,50 kr"

let today = Temporal.Now.plainDateISO()
println(today)

let date = Temporal.PlainDate.from({ year: 2026, month: 7, day: 11 })
println(date.year)                 // 2026
```

---

## Chapter 14 — Compiler and Self-Hosting

### 14.1 Pipeline

```
source text
    → lexer.kab        → token[]
    → parser.kab         → AST
    → emit.kab           → opcode IR
    → serialize.kab      → .kbc text
    → compile.kab        → full pipeline
```

The Rust implementation mirrors these stages in `src/lexer.rs`, `src/parser.rs`, `src/bytecode/compiler.rs`, and `src/bytecode/types.rs`.

### 14.2 Bytecode cache

`.kabootar/cache/*.kbc` stores compiled bytecode. Cache entries are invalidated when the source file is newer.

### 14.3 Self-hosting status

- `lexer.kab` compiles and runs via self-host.
- `parser.kab` compiles and runs via self-host.
- `emit.kab` compiles and runs via self-host (heavy).
- Next milestones: `serialize.kab` full self-host, then true bootstrap (`compile.kab` compiled by self-hosted `compile()`).

### 14.4 Self-host design rules

- Module-global state is used for scratch slots; Kabootar function locals are not re-entrant across recursive calls.
- `push` returns a new array: always assign `arr = push(arr, item)`.
- Save AST fields before recursive descent.
- Use bracket access for AST keys to avoid collisions.
- Keep top-level functions per module low (~4–7) to avoid stack overflow on Windows.
- Nested if/while require explicit stacks for jump patching.

---

## Appendix A — Grammar Reference

### Statements

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

### Expressions

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

### Patterns

```
pattern     := literal | identifier | "_" | "Some" "(" pattern ")"
             | "None" | "Ok" "(" pattern ")" | "Err" "(" pattern ")"
             | enum_name "." variant ("(" pattern* ")")?
             | "[" pattern_piece* "]" | "{" object_pattern_field* "}"
```

---

## Appendix B — Builtin and Native Functions Summary

### Console

- `println(...)`, `log(...)`, `console_log(...)`, `console_warn(...)`, `console_error(...)`

### Math

See Chapter 12.1.

### Type checks

See Chapter 12.15.

### Array/Object

See Chapter 7.

### Runtime introspection

- `lang_info()`, `ecosystem_info()`, `modules_catalog()`

### Registry

- `registry_publish(path)`, `registry_install(name, version)`, `registry_list()`, `registry_search(query)`, `registry_seed()`, `registry_uninstall(name, version)`

---

## Appendix C — Differences from JavaScript

| JavaScript | Kabootar |
|------------|----------|
| `var` | removed; only `let`/`const` |
| `===` | `==` is already strict per type |
| implicit coercion (`"1" + 2`) | runtime error |
| `null == undefined` | `false` |
| prototype inheritance | C#-style `class` with `this` |
| `function` | `fn` |
| arrow `=>` | same syntax, plus block body support |
| `constructor` | `fn init(...)` |
| `for…in` on objects | `for key in obj` |
| `for…of` on arrays | `for x of xs` |
| `with` statement | removed |
| `eval()` | removed |
| `Infinity`/`-Infinity` | integer division is error; floats use `NaN` |

---

*End of The Kabootar Language — A Complete Reference (English).*
