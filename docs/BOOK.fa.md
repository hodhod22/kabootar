# زبان کبوتر — مرجع کامل

<div dir="rtl" lang="fa">


## مقدمه

کبوتر یک زبان برنامه‌نویسی fullstack برای سیستم‌ها و برنامه‌های کاربردی با ساختاری شبیه به JavaScript، رفتار تایپ صریح و یک runtime بزرگ است که همه چیز را از HTTP/SQL گرفته تا workers، رمزنگاری و self-hosting پوشش می‌دهد. این کتاب زبان را همان‌گونه که در پیاده‌سازی `nova-interpreter` وجود دارد توصیف می‌کند.

این زبان عمداً JavaScript نیست: implicit coercion، `var` hoisting، ارث‌بری prototype و `eval` حذف شده‌اند. در عوض، تایپ‌های قابل پیش‌بینی، `match` به سبک Rust، کلاس‌هایی به سبک C#، ماژول‌ها، async/generatorها و یک pipeline کامپایل self-hosted ارائه می‌دهد.

این مرجع به یک آموزش اصلی زبان (فصول ۱ تا ۱۰)، یک راهنمای runtime (فصول ۱۱ تا ۱۳) و یک راهنمای کامپایلر/self-hosting (فصل ۱۴) تقسیم شده است.

---

## فصل ۱ — مقدمه

### ۱.۱ کبوتر چیست؟

کبوتر یک زبان همه‌منظوره است که به یک فرمت bytecode داخلی (`.kbc`) کامپایل می‌شود و روی یک VM مبتنی بر Rust اجرا می‌گردد. سه هدف اصلی آن عبارتند از:

۱. **ساختار آشنا** — tokenها و بلوک‌هایی شبیه به C/Rust/JavaScript.
۲. **تایپ‌های قابل پیش‌بینی** — بدون تبدیل صامت؛ `null`، `undefined`، `NaN` و `Result` صریح هستند.
۳. **runtime همه‌چیز‌داخل‌خود** — ماژول‌هایی برای SQL، HTTP، دسترسی OS، رمزنگاری، علوم، رندر DOM و غیره.

### ۱.۲ برنامهٔ آغازین (Hello world)

```kabootar
println("Hello, Kabootar!")
```

اجرا با:

```bash
kabootar hello.kab
```

### ۱.۳ پسوند فایل و نقطه ورود ماژول

فایل‌های منبع از پسوند `.kab` استفاده می‌کنند. کامپایلر/مفسر به طور پیش‌فرض `main.kab` یا فیلد `entry` را از `kabootar.toml` بارگذاری می‌کند.

---

## فصل ۲ — ساختار لغوی و انواع داده

### ۲.۱ توکن و فضای خالی

- شناسه‌ها: حروف ASCII، ارقام و `_`، بدون شروع با رقم.
- کلمات کلیدی: `fn`، `let`، `const`، `if`، `else`، `while`، `for`، `in`، `of`، `return`، `break`، `continue`، `throw`، `raise`، `try`، `catch`، `finally`، `pass`، `assert`، `with`، `using`، `match`، `switch`، `case`، `default`، `fallthrough`، `do`، `async`، `await`، `yield`، `fn*`، `class`، `extends`، `interface`، `implements`، `enum`، `import`، `pub`، `this`، `super`، `true`، `false`، `null`، `undefined`، `NaN`، `Some`، `None`، `Ok`، `Err`، `is`، `not`، `delete`.
- عملگرها: `+ - * / % **`، `== != < <= > >=`، `&& || ??`، `! ~`، `& | ^ << >> >>>`، `? :`، `=>`، `= += -= *= /= %= **=`.
- توضیحات: `// توضیح خطی` و `/* توضیح بلوکی */`.
- استفاده از سمی‌کالن اختیاری است؛ خطوط جدید دستورات را در بلوک‌ها جدا می‌کنند.

### ۲.۲ انواع اولیه

| توضیح | نمونه | نوع |
|-------|-------|-----|
| ۶۴ بیتی علامت‌دار. | `42`، `-7` | عدد صحیح |
| ۶۴ بیتی IEEE 754. | `3.14`، `NaN` | عدد اعشاری |
| عدد صحیح با دقت دلخواه؛ با `number` ترکیب نمی‌شود. | `123n`، `BigInt("99")` | BigInt |
| UTF-8؛ تمپلیت‌ها از `${expr}` پشتیبانی می‌کنند. | `"text"`، `` `template ${x}` `` | رشته |
| | `true`، `false` | بولین |
| «مقدار ندارد» به‌صورت صریح. | `null` | Null |
| binding مقداردهی‌نشده یا کلید/ایندکس ناموجود. | `undefined` | Undefined |

### ۲.۳ انواع مرکب

- **آرایه**: `[1, 2, 3]`.
- **آبجکت**: `{ "a": 1, b: 2 }`.
- **Map/Set**: از طریق `map_new` / `set_new` ایجاد می‌شوند.
- **نمونه کلاس**: از تعاریف `class` ایجاد می‌شوند.
- **Result**: `Ok(v)`، `Err(e)`.
- **Option**: `Some(v)`، `None`.

### ۲.۴ ارزش منطقی (truthiness)

مقادیر falsy: `null`، `undefined`، `false`، `0`، `""`، `NaN`. بقیه truthy هستند.

### ۲.۵ `null` در مقابل `undefined`

- `null` = عدم وجود عمدی.
- `undefined` = مقداردهی نشده یا ناموجود.
- `null == undefined` در کبوتر `false` است.

---

## فصل ۳ — متغیرها و ثوابت

### ۳.۱ `let` و `const`

```kabootar
let x = 10
x = 20               // ok

const PI = 3.14
PI = 3               // خطای runtime

let y                // undefined
println(y)           // "undefined" را چاپ می‌کند
```

`var` وجود ندارد.

**مثال: جابجایی مقادیر**

```kabootar
let a = 1
let b = 2
let tmp = a
a = b
b = tmp
println(a)           // 2
println(b)           // 1
```

### ۳.۲ بازآرایی (Destructuring)

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

### ۳.۳ گسترش (Spread)

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

### ۳.۴ محدوده (Scope)

Scope در سطح بلوک مانند `let` در JavaScript است. خواندن یک متغیر اعلام‌نشده خطای runtime است، نه `undefined`.

```kabootar
let outer = 10
if true {
    let inner = 20
    println(outer)   // 10
}
// println(inner)    // خطای runtime: متغیر تعریف نشده
```

---

## فصل ۴ — عبارات و عملگرها

### ۴.۱ حسابی

```kabootar
println(1 + 2)          // 3
println(3 - 4)          // -1
println(5 * 6)          // 30
println(7 / 8)          // 0.875 (تقسیم اعشاری)
println(9 % 2)          // 1
println(2 ** 3 ** 2)    // 512 (راست-شرکت‌پذیر)
```

تقسیم عدد صحیح بر صفر خطاست. `NaN` اعشاری صریحاً از طریق literal `NaN` یا ریاضیات اعشاری به دست می‌آید.

**مثال: مساحت دایره**

```kabootar
const PI = 3.14159
let r = 5
let area = PI * r * r
println(area)           // 78.53975
```

### ۴.۲ مقایسه

```kabootar
println(1 == 1)                  // true
println(1 != 2)                  // true
println(3 < 5)                   // true
println("5" == 5)                // false (سخت بر اساس نوع)
println(null == undefined)       // false
```

### ۴.۳ منطقی

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

### ۴.۴ بیتی

```kabootar
println(5 & 3)           // 1   (0101 & 0011)
println(5 | 3)           // 7   (0101 | 0011)
println(5 ^ 3)           // 6   (0101 ^ 0011)
println(~5)              // -6
println(1 << 3)          // 8
println(8 >> 2)          // 2
println(-8 >>> 2)        // 1073741822
```

با معنای ۳۲ بیتی (`ToInt32` / `ToUint32`) اعمال می‌شوند.

### ۴.۵ سه‌تایی

```kabootar
let n = -5
let sign = n > 0 ? "positive" : "non-positive"
println(sign)            // "non-positive"

let age = 20
let label = age >= 18 ? "adult" : "minor"
println(label)           // "adult"
```

### ۴.۶ `?` روی Result

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

`?` مقدار `Ok(v)` را باز می‌کند یا `Err(e)` را از تابع فعلی برمی‌گرداند.

### ۴.۷ `in`

```kabootar
let obj = { a: 1, b: 2 }
let arr = [10, 20, 30]
println("a" in obj)      // true
println("c" in obj)      // false
println(1 in arr)        // true
println(5 in arr)        // false
println("x" in "xyz")    // true
```

### ۴.۸ `is` / `is not`

```kabootar
let x = null
println(x is null)           // true
println(x is not undefined)  // true

let y = 0
println(y is false)          // false
```

### ۴.۹ دسترسی عضو و ایندکس

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

### ۴.۱۰ عبارات تخصیص

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

### ۴.۱۱ `delete`

```kabootar
let obj = { a: 1, b: 2 }
delete obj.a
println("a" in obj)      // false
println("b" in obj)      // true
```

---

## فصل ۵ — کنترل جریان

### ۵.۱ `if` / `else`

استفاده از پرانتز در شرط اختیاری است:

```kabootar
let x = -5
if x < 0 {
    println("negative")
} else if x == 0 {
    println("zero")
} else {
    println("positive")
}
// "negative" را چاپ می‌کند
```

**مثال: طبقه‌بندی نمرات**

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

### ۵.۲ `while`

```kabootar
let i = 0
while i < 5 {
    println(i)   // 0, 1, 2, 3, 4
    i = i + 1
}

// جمع 1 تا 10
let sum = 0
let n = 1
while n <= 10 {
    sum = sum + n
    n = n + 1
}
println(sum)     // 55
```

`break` و `continue` پشتیبانی می‌شوند.

### ۵.۳ `do … while`

```kabootar
let i = 0
let sum = 0
do {
    sum = sum + i
    i = i + 1
} while i < 5
println(sum)     // 10 (0+1+2+3+4)
```

### ۵.۴ حلقه‌های `for`

```kabootar
let xs = ["a", "b", "c"]

// سبک C
for let i = 0; i < len(xs); i = i + 1 {
    println(xs[i])   // a, b, c
}

// مقادیر
for x of xs {
    println(x)       // a, b, c
}

// ایندکس/کلید
for i in xs {
    println(i)       // 0, 1, 2
}

let obj = { name: "Ada", age: 36 }
for key in obj {
    println(key)     // "name", "age"
}
```

`for const x of xs` و `for let x of xs` تغییرپذیری متغیر حلقه را کنترل می‌کنند.

### ۵.۵ `match`

```kabootar
let n = 3
let desc = match n {
    0 => "zero",
    x if x > 0 => "positive",
    _ => "negative"
}
println(desc)        // "positive"
```

**مثال: تطابق Option**

```kabootar
let opt = Some(42)
let value = match opt {
    Some(x) => x,
    None => 0
}
println(value)       // 42
```

**مثال: تطابق آرایه‌ها**

```kabootar
let pair = [1, 2]
match pair {
    [a, b] => println(a + b),   // 3
    _ => println("other")
}
```

الگوها شامل: literal، متغیر، wildcard `_`، `Some`/`None`، `Ok`/`Err`، variantهای enum، آرایه‌ها، آبجکت‌ها و guardها (`if expr =>`) می‌شوند.

### ۵.۶ `if let` / `while let`

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
// 1, 2 را چاپ می‌کند
```

### ۵.۷ `switch`

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
// "two or three" را چاپ می‌کند
```

fall-through ضمنی وجود ندارد؛ صریحاً از `fallthrough` استفاده کنید.

### ۵.۸ `try` / `catch` / `finally`

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

`try`/`catch` روی مقادیر `Result` کار می‌کند: `Ok(v)` به `v` باز می‌شود، `Err(e)` وارد بلوک catch می‌شود.

### ۵.۹ `pass`، `assert`، `with`، `using`

```kabootar
pass                            // no-op

assert(1 + 1 == 2, "math works")

// with منبع را در پایان بلوک آزاد می‌کند
with open_resource() as r {
    use(r)
}

// using صریحاً مقدار را dispose می‌کند
using file = open_file("log.txt");
// file اینجا dispose می‌شود
```

---

## فصل ۶ — توابع

### ۶.۱ توابع نام‌گذاری‌شده

```kabootar
fn add(a, b) {
    return a + b
}
println(add(2, 3))       // 5

pub fn exported(a) {            // از ماژول export می‌شود
    return a
}
```

### ۶.۲ توابع arrow

```kabootar
let double = (x) => x * 2
println(double(4))        // 8

let sum = (a, b) => {
    return a + b
}
println(sum(1, 2))        // 3
```

### ۶.۳ پارامترها

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

### ۶.۴ توابع async و `await`

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

`await` فقط داخل `async fn` یا arrowهای async کار می‌کند. صف microtask را به صورت FIFO تخلیه می‌کند.

### ۶.۵ Generatorها

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

`yield*` به iterable/generator دیگر تفویض می‌کند. Generatorهای async از `async fn*` استفاده می‌کنند.

### ۶.۶ توابع مرتبه بالا

توابع first-class هستند و می‌توان آن‌ها را ارسال، بازگرداند و در متغیرها ذخیره کرد.

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

### ۶.۷ `return`

`return` در پایان تابع اختیاری است؛ اگر `return` صریح نباشد، مقدار آخرین عبارت به طور خودکار بازگردانده می‌شود.

```kabootar
fn square(x) {
    x * x    // return ضمنی
}
println(square(3))        // 9
```

---

## فصل ۷ — آبجکت‌ها، آرایه‌ها و کلاس‌ها

### ۷.۱ آرایه‌ها

```kabootar
let xs = [1, 2, 3]
xs.push(4)                     // تغییر می‌دهد
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

کمک‌کننده‌های آرایه شامل `map`، `filter`، `reduce`، `find`، `slice`، `sort`، `reverse`، `join`، `includes`، `some`، `every`، `index_of`، `flat`، `flat_map`، `at`، `fill`، `to_spliced`، `to_reversed`، `to_sorted`، `shift`، `unshift`، `splice`، `concat` می‌شوند.

### ۷.۲ آبجکت‌ها

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

کمک‌کننده‌های آبجکت شامل `assign`، `has_key`، `delete_prop`، `keys`، `values`، `entries`، `from_entries`، `clone_shallow`، `group_by` می‌شوند.

### ۷.۳ کلاس‌ها

کلاس‌های کبوتر به سبک C# هستند، نه prototypeهای JavaScript.

```kabootar
class Point {
    x: number;
    y: number;

    fn init(a, b) {
        self.x = a
        self.y = b
    }

    fn sum() {
        return self.x + self.y
    }
}

let p = Point(3, 4)
println(p.sum())                     // 7
```

### ۷.۴ ارث‌بری

```kabootar
class Animal {
    name: string;
    fn init(n) { self.name = n }
    fn label() { return self.name }
}

class Dog extends Animal {
    breed: string;

    fn init(n, b) {
        super.init(n)
        self.breed = b
    }

    fn label() {
        return super.label() + " (" + self.breed + ")"
    }
}

let d = Dog("Rex", "lab")
println(d.label())             // "Rex (lab)"
```

### ۷.۵ Interfaceها

```kabootar
interface Greeter {
    fn greet();
}

class Person implements Greeter {
    name: string;
    fn greet() {
        return "hi " + self.name
    }
}

let p = Person()
p.name = "Ada"
println(p.greet())             // "hi Ada"
println(is_impl(p, "Greeter")) // true
```

### ۷.۶ Enumها

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

### ۷.۷ فیلدهای خصوصی

```kabootar
class Counter {
    #n: number = 0;

    fn inc() {
        self.#n = self.#n + 1
    }

    fn get() {
        return self.#n
    }
}

let c = Counter()
c.inc()
c.inc()
println(c.get())               // 2
// println(c.#n)               // خطا: فیلد خصوصی
```

---

## فصل ۸ — تطابق الگو

### ۸.۱ الگوها

یک الگو می‌تواند باشد:

- Literal: `1`، `"x"`، `true`، `null`، `undefined`، `NaN`
- متغیر: `x`
- Wildcard: `_`
- `Some(p)`، `None`
- `Ok(p)`، `Err(p)`
- variant Enum: `Color.Red`، `Msg.Move(x, y)`
- آرایه: `[a, b, ...rest]`
- آبجکت: `{ name, age: a }`

**مثال: تطابق با Option**

```kabootar
let opt = Some(7)
match opt {
    Some(x) => println(x),       // 7
    None => println("none")
}
```

**مثال: تطابق آرایه‌ها و آبجکت‌ها**

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

### ۸.۲ شاخه‌های match با guard

```kabootar
let n = 5
let desc = match n {
    0 => "zero",
    x if x > 0 => "positive",   // guard
    _ => "negative"
}
println(desc)                  // "positive"
```

### ۸.۳ `if let` / `while let`

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

## فصل ۹ — ماژول‌ها

### ۹.۱ syntax import

```kabootar
import "math"
println(add(1, 2))             // 3

import "greet"
greet("Kabootar")
```

importها نام‌ها را در محیط فعلی bind می‌کنند.

### ۹.۲ ماژول‌های توکار

| ماژول | محتوا |
|-------|-------|
| `std` | JSON، کمک‌کننده‌های تایپ |
| `json` | `parse`، `dump` |
| `collections` | کمک‌کننده‌های Map/Set |
| `strings` | ابزارهای رشته |
| `math` | کمک‌کننده‌های حساب پایه |
| `http` | مسیریابی HTTP و دریافت |
| `crypto` | توابع رمزنگاری |
| `science` | فیزیک، شیمی، آمار |
| `docai` | کمک‌کننده‌های AI مستندات |
| `codai` | کمک‌کننده‌های AI کد |
| `sql` | پایگاه داده SQL درون‌پردازه |
| `os` | سیستم فایل sandboxed |

**مثال: استفاده از ماژول‌های توکار**

```kabootar
import "json"
import "math"

let obj = json_parse('{"x":3,"y":4}')
println(obj.x)                 // 3
println(add(obj.x, obj.y))     // 7
```

### ۹.۳ ماژول‌های فایلی

`lib/greet.kab` را بسازید:

```kabootar
pub fn greet(name) {
    return "Hello, " + name
}

fn secret() { }             // خصوصی
```

فقط `pub fn`، `pub let` و `pub const` export می‌شوند.

### ۹.۴ ساختار پروژه

```toml
# kabootar.toml
version = "0.1.0"
entry = "main.kab"
port = 8080

[dependencies]
greet = "1.0.0"
```

**مثال: `main.kab`**

```kabootar
import "greet"

fn main() {
    println(greet("Kabootar"))
}

main()
```

### ۹.۵ importهای نسخه‌دار

```kabootar
import "greet@1.0"
```

### ۹.۶ رجیستری بسته محلی

```bash
kabootar publish lib/greet.kab
kabootar install greet@1.0
kabootar install
```

دسترسی برنامه‌ای:

```kabootar
registry_publish("lib/greet.kab")
registry_install("greet", "1.0")
let mods = registry_list()
println(mods)                  // [{ name: "greet", version: "1.0.0" }, ...]
```

### ۹.۷ `import.meta` و import پویا

```kabootar
println(import.meta.url)       // URL ماژول فعلی

async fn loadMath() {
    let math = await import("math")
    println(math.add(2, 3))    // 5
}
```

---

## فصل ۱۰ — مدیریت خطا

### ۱۰.۱ `Result` و `Option`

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

### ۱۰.۲ عملگر `?`

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

### ۱۰.۳ `throw` / `raise`

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

با `try`/`catch` گرفته می‌شود.

### ۱۰.۴ کمک‌کننده‌های `Error`

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

## فصل ۱۱ — برنامه‌نویسی async و generatorها

### ۱۱.۱ Promiseها

```kabootar
let p = promise_new((resolve, reject) => {
    resolve(42)
})

promise_then(p, (v) => v * 2)
promise_all([p1, p2])
await_all([p1, p2])             // آرایه را به صورت sync برمی‌گرداند
```

**مثال: زنجیره promise**

```kabootar
let p = promise_new((resolve, reject) => {
    set_timeout(() => resolve(10), 10)
})

let q = promise_then(p, (v) => v * 2)
let r = promise_then(q, (v) => println(v))   // 20
```

**مثال: منتظر ماندن برای چند promise**

```kabootar
async fn load() {
    let a = promise_resolve(1)
    let b = promise_resolve(2)
    let values = await_all([a, b])
    println(values)              // [1, 2]
}
```

### ۱۱.۲ Iteratorهای async

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

### ۱۱.۳ کمک‌کننده‌های iterator

آداپتورهای lazy iterator:

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

**مثال: pipeline lazy**

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

با `for … of`، `.toArray()`، `.reduce()`، `.find()` و غیره مصرف می‌شوند.

---

## فصل ۱۲ — کتابخانه استاندارد

### ۱۲.۱ ریاضی

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

`floor`، `ceil`، `round`، `abs`، `min`، `max`، `sqrt`، `pow`، `random`، `sign`، `trunc`، `clamp`، `pi`، `e`، `log`، `log2`، `log10`، `exp`، `sin`، `cos`، `tan`، `asin`، `acos`، `atan`، `atan2`، `hypot`، `cbrt`، `imul`، `clz32`، `fround`، `fmod`، `log1p`، `expm1`.

### ۱۲.۲ فرمت‌بندی اعداد

```kabootar
println(format("{:.2}", 3.14159))    // "3.14"
println(format("{:05}", 42))          // "00042"
println(format("{:x}", 255))          // "ff"
```

### ۱۲.۳ رشته‌ها

```kabootar
let s = "  Hello, Kabootar!  "
println(trim(s))                       // "Hello, Kabootar!"
println(to_upper_case("hello"))        // "HELLO"
println(to_lower_case("WORLD"))        // "world"
println(split("a,b,c", ","))          // ["a", "b", "c"]
println(replace("hello world", "world", "Kabootar")) // "hello Kabootar"
println(concat("a", "b", "c"))         // "abc"
println(string_includes("abcdef", "cd")) // true
println(string_starts_with("abc", "ab")) // true
println(string_ends_with("abc", "bc"))   // true

// تمپلیت
let name = "Ada"
println(`Hello, ${name}!`)             // "Hello, Ada!"
```

### ۱۲.۴ JSON

```kabootar
let data = { name: "Kabootar", year: 2025 }
let text = json_stringify(data)
println(text)                          // '{"name":"Kabootar","year":2025}'

let parsed = json_parse(text)
println(parsed.name)                   // "Kabootar"

let pretty = json_stringify(data, { indent: 2 })
println(pretty)
```

### ۱۲.۵ RegExp

```kabootar
let re = regexp_new(r"\b\w+\b", "g")
let m = regexp_exec(re, "hello world")
println(m[0])                            // "hello"

println(regexp_test(re, "test"))       // true
println(regexp_split(re, "one two"))     // ["one", "two"]
println(regexp_replace(re, "a b c", "X")) // "X X X"
```

### ۱۲.۶ تاریخ و تایمرها

```kabootar
let now = date_now()
println(now)

set_timeout(() => println("later"), 100)
let id = set_interval(() => println("tick"), 1000)
clear_interval(id)
```

### ۱۲.۷ مجموعه‌ها

```kabootar
let m = map_new()
map_set(m, "en", "hello")
println(map_get(m, "en"))              // "hello"
println(map_has(m, "sv"))              // false

let s = set_new()
set_add(s, 1)
set_add(s, 2)
set_add(s, 2)
println(set_size(s))                   // 2
```

### ۱۲.۸ آرایه‌های تایپ‌شده

```kabootar
let i8 = i8_array_new([1, 2, 3])
println(len(i8))                       // 3
println(i8[0])                         // 1

let u32 = u32_array_new([10, 20, 30])
println(sum(u32))                      // 60
```

`i8_array_new`، `u8_array_new`، `i16_array_new`، `u16_array_new`، `i32_array_new`، `u32_array_new`، `i64_array_new`، `u64_array_new`، `f32_array_new`، `f64_array_new`، `buffer_new`، `dataview_new`.

### ۱۲.۹ Atomics

```kabootar
let mem = shared_array_buffer_new(16)
let view = i32_array_new(mem)
atomic_store(view, 0, 5)
println(atomic_load(view, 0))          // 5
println(atomic_add(view, 0, 1))        // 6
atomic_wait(view, 0, 6)
atomic_notify(view, 0, 1)
```

### ۱۲.۱۰ رمزنگاری

```kabootar
let hash = sha256("hello")
println(hash)

let sig = ed25519_sign(key, "payload")
println(ed25519_verify(key, "payload", sig)) // true

let aes = aes_gcm_encrypt(key, "secret", iv)
println(aes_gcm_decrypt(key, aes, iv))       // "secret"
```

### ۱۲.۱۱ URI

```kabootar
let encoded = encode_uri_component("hello world")
println(encoded)                       // "hello%20world"
println(decode_uri_component(encoded))   // "hello world"

let url = parse_url("https://user:pass@example.com:8080/path?x=1#frag")
println(url.host)                      // "example.com"
```

### ۱۲.۱۲ بررسی تایپ

```kabootar
println(typeof(42))                      // "number"
println(is_number("x"))                // false
println(is_array([1, 2]))              // true
println(is_object({}))                   // true
println(is_function((x) => x))         // true
println(is_promise(promise_resolve(1)))  // true
println(is_undefined(undefined))       // true
```

### ۱۲.۱۳ محیط

```kabootar
println(env_get("HOME"))
set_env("KABOOTAR", "v1")
println(env_get("KABOOTAR"))           // "v1"
println(get_args())                    // آرگومان‌های CLI
println(cwd())                         // دایرکتوری کاری فعلی
```

---

## فصل ۱۳ — محیط runtime

### ۱۳.۱ CLI

```bash
kabootar run main.kab              # اجرا
kabootar build main.kab            # کامپایل به .kbc
kabootar test *.kab                 # اجرای آزمون‌ها
kabootar fmt main.kab               # فرمت فایل
kabootar lsp                        # سرور زبان
kabootar --target-dir build         # دایرکتوری خروجی
kabootar --version
kabootar --help
```

### ۱۳.۲ HTTP

```kabootar
import "http"

fn handler(req, res) {
    return json({ message: "hello", path: req.url })
}

http_serve(8080, handler)

// درخواست
let response = http_get("https://api.example.com/data")
println(response.status)
println(response.body)
```

`http_get`، `http_post`، `http_put`، `http_delete`، `http_serve`، `http_route`، `http_middleware`، `http_static`، `http_ws_upgrade`.

### ۱۳.۳ SQL

```kabootar
import "sql"

let db = sql_open("app.db")
sql_exec(db, "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)")
sql_exec(db, "INSERT INTO users (name) VALUES (?)", "Ada")
let rows = sql_query(db, "SELECT * FROM users")
println(rows[0].name)                // "Ada"
```

`sql_open`، `sql_query`، `sql_exec`، `sql_transaction`، `sql_migrate`.

### ۱۳.۴ OS

```kabootar
import "os"

let files = os_list_dir(".")
println(files)                       // ["main.kab", "lib"]

println(os_read_file("hello.txt"))  // محتوای فایل
os_write_file("out.txt", "data")
println(os_env("HOME"))             // مسیر home
```

### ۱۳.۵ Workers

```kabootar
let w = worker_new("worker.kab")
worker_post_message(w, { job: 1 })
let result = worker_await_message(w)
println(result)
worker_terminate(w)
```

`worker_new`، `worker_post_message`، `worker_await_message`، `worker_terminate`، `worker_pool_map`.

### ۱۳.۶ Streams

```kabootar
let source = stream_from_array(["a", "b", "c"])
let mapped = stream_map(source, (x) => x + x)
let filtered = stream_filter(mapped, (x) => len(x) > 1)

for chunk of filtered {
    println(chunk)                   // "aa", "bb", "cc"
}

let writable = writable_stream_new()
let writer = writable_get_writer(writable)
writer_write(writer, "hello")
writer_close(writer)
```

`stream_new`، `stream_from_array`، `stream_read`، `stream_read_all`، `stream_pipe_to`، `stream_tee`، `stream_transfer`، `byte_stream_new`، `transform_stream_new`، `reader_read`، `writer_write`.

### ۱۳.۷ WebSocket

```kabootar
let ws = ws_connect("wss://echo.example.com/")
ws_send(ws, "hello")
let msg = ws_recv(ws)
println(msg)                         // "hello"
ws_close(ws)

let server = tcp_listen("127.0.0.1:9000")
let client = tcp_accept(server)
```

`ws_channel_pair`، `ws_link`، `ws_send`، `ws_recv`، `ws_connect`، `tcp_listen`، `tcp_connect`، `tcp_accept`، `udp_bind`، `udp_send`، `udp_recv`.

### ۱۳.۸ DOM و رندر

```kabootar
let ui = html! {
    div {
        h1 { "Hello" }
        p { "Kabootar" }
    }
}

dom_append(document_body(), ui)

let btn = dom_query("#submit")
dom_on(btn, "click", () => println("clicked"))
```

`html!`، `svg!`، `dom_query`، `dom_create`، `dom_append`، `dom_remove`، `dom_set_attr`، `dom_set_text`، `dom_get_text`، `dom_on`، `dom_add_class`.

### ۱۳.۹ FFI و WASM

```kabootar
let lib = ffi_open("./libcalc.so")
let add = ffi_lookup(lib, "add")
println(ffi_call(add, [2, 3]))        // 5

let module = wasm_compile(bytes)
wasm_instantiate(module, { env: { print: println } })
```

### ۱۳.۱۰ سازگاری npm/Node.js

```kabootar
import { readFileSync, writeFileSync } from "node:fs"
import { join } from "node:path"

let text = readFileSync("input.txt")
writeFileSync("output.txt", text)
println(join("/home", "user", "file.txt"))
```

`import "node:fs"`، `"node:path"`، `"node:process"`، `"node:os"`، `"node:url"`، `"node:crypto"`، `"node:buffer"`.

### ۱۳.۱۱ `Intl` و `Temporal`

```kabootar
let fmt = Intl.NumberFormat("sv-SE", { style: "currency", currency: "SEK" })
println(fmt.format(1234.5))          // "1 234,50 kr"

let today = Temporal.Now.plainDateISO()
println(today)

let date = Temporal.PlainDate.from({ year: 2026, month: 7, day: 11 })
println(date.year)                     // 2026
```

---

## فصل ۱۴ — کامپایلر و self-hosting

### ۱۴.۱ خط لوله

```
متن منبع
    → lexer.kab        → token[]
    → parser.kab       → AST
    → emit.kab         → IR opcode
    → serialize.kab    → متن .kbc
    → compile.kab      → خط لولهٔ کامل
```

پیاده‌سازی Rust این مراحل را در `src/lexer.rs`، `src/parser.rs`، `src/bytecode/compiler.rs` و `src/bytecode/types.rs` منعکس می‌کند.

### ۱۴.۲ کش bytecode

`.kabootar/cache/*.kbc` bytecode کامپایل‌شده را ذخیره می‌کند. اگر فایل منبع جدیدتر باشد، ورودی کش باطل می‌شود.

### ۱۴.۳ وضعیت self-hosting

- `lexer.kab` از طریق self-host کامپایل و اجرا می‌شود.
- `parser.kab` از طریق self-host کامپایل و اجرا می‌شود.
- `emit.kab` از طریق self-host کامپایل و اجرا می‌شود (سنگین).
- نقاط بعدی: self-host کامل `serialize.kab`، سپس bootstrap واقعی (`compile.kab` کامپایل‌شده با `compile()` خودکامپایل).

### ۱۴.۴ قوانین طراحی self-hosting

- برای slotهای scratch از state سطح ماژول استفاده کنید؛ localهای تابع Kabootar در فراخوانی بازگشتی re-entrant نیستند.
- `push` آرایهٔ جدید برمی‌گرداند: همیشه `arr = push(arr, item)` بنویسید.
- برای کوتاه‌کردن stack از `pop(arr)` استفاده کنید (نه حلقهٔ دستی کپی).
- `push(stack, len(x))` در self-host compile اشتباه codegen می‌شود؛ از `pushLen(stack, x)` یا scratch جداگانه استفاده کنید.
- قبل از نزول بازگشتی، فیلدهای AST را ذخیره کنید.
- برای کلیدهای AST از دسترسی براکتی `node["field"]` استفاده کنید تا برخورد نام رخ ندهد.
- تعداد توابع سطح بالای هر ماژول را کم (~۴–۷) نگه دارید تا روی Windows stack overflow نشود.
- `if`/`while` تو در تو به stackهای jump-patch صریح نیاز دارند.

### ۱۴.۵ فایل‌های self-host

| فایل | نقش |
|------|------|
| `self_host/lexer.kab` | tokenization |
| `self_host/parser.kab` | ساخت AST |
| `self_host/emit.kab` | تولید IR |
| `self_host/serialize.kab` | نوشتن .kbc |
| `self_host/compile.kab` | wrapper خط لوله |

### ۱۴.۶ اجرای self-hosting

```bash
cargo test self_host_emit_full_compile_smoke -- --nocapture
CARGO_TARGET_DIR=target-alt3 cargo test --test self_host self_host_emit_kbc_run_only -- --ignored --nocapture
```

### ۱۴.۷ آزمون

```bash
cargo test --all-features
cargo test parser -- --nocapture
CARGO_TARGET_DIR=target-alt3 cargo test --test self_host self_host -- --test-threads=1
```

---

## پیوست الف — مرجع گرامر

### دستورات

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

### عبارات

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

### الگوها

```
pattern     := literal | identifier | "_" | "Some" "(" pattern ")"
             | "None" | "Ok" "(" pattern ")" | "Err" "(" pattern ")"
             | enum_name "." variant ("(" pattern* ")")?
             | "[" pattern_piece* "]" | "{" object_pattern_field* "}"
```

---

## پیوست ب — خلاصهٔ توابع builtin و native

### کنسول

- `println(...)`، `log(...)`، `console_log(...)`، `console_warn(...)`، `console_error(...)`

### ریاضی

فصل ۱۲.۱ را ببینید.

### بررسی نوع

فصل ۱۲.۱۵ را ببینید.

### آرایه / آبجکت

فصل ۷ را ببینید.

### introspection زمان اجرا

- `lang_info()`، `ecosystem_info()`، `modules_catalog()`

### رجیستری

- `registry_publish(path)`، `registry_install(name, version)`، `registry_list()`، `registry_search(query)`، `registry_seed()`، `registry_uninstall(name, version)`

---

## پیوست ج — تفاوت‌ها با JavaScript

| JavaScript | Kabootar |
|------------|----------|
| `var` | حذف شده؛ فقط `let`/`const` |
| `===` | `==` از قبل strict است |
| coercion ضمنی (`"1" + 2`) | خطای runtime |
| `null == undefined` | `false` |
| ارث‌بری prototype | `class` به سبک C# با `self` |
| `function` | `fn` |
| arrow `=>` | همان syntax، با پشتیبانی block body |
| `constructor` | `fn init(...)` |
| `for…in` روی آبجکت | `for key in obj` |
| `for…of` روی آرایه | `for x of xs` |
| دستور `with` | حذف شده |
| `eval()` | حذف شده |
| `Infinity`/`-Infinity` | تقسیم صحیح خطاست؛ float از `NaN` استفاده می‌کند |

---

## پیوست د — ویژگی‌های متمایز

۱. `==` سخت و بدون coercion.
۲. کلاس‌های C#-style با `self` و ارث‌بری صریح.
۳. `match`، `if let`، `while let` به سبک Rust.
۴. `Option` و `Result` داخلی.
۵. توابع generator و async/generator.
۶. ماژول‌ها با `import`، `pub` و رجیستری بسته محلی.
۷. Runtime HTTP/SQL/OS/Workers/Streams/WebSocket/DOM/FFI/WASM.
۸. self-hosting با کامپایلر کاملاً در کبوتر نوشته شده.
۹. هیچ `eval`، `var` hoisting یا implicit coercion وجود ندارد.
۱۰. BigInt، تایپ‌شده Arrays و Atomics.

---

*این کتاب آخرین وضعیت زبان کبوتر را در `nova-interpreter` توصیف می‌کند. برای جزئیات بیشتر مستندات تکمیلی `docs/LANGUAGE.md`، `docs/FEATURES.md`، `docs/JAVASCRIPT.md`، `docs/CLASSES.md` و `docs/MODULES.md` را ببینید.*

</div>
