# Kabootar — språkfunktioner (feature-matris)

> **Princip:** Kabootar ska ha **allt som finns i JavaScript** utom medvetet borttagna problematiska delar, plus **kompletta** lånade konstruktioner från Rust, C#, och andra språk.

Se även [JAVASCRIPT.md](JAVASCRIPT.md) (skillnader för JS-utvecklare) och [LANGUAGE.md](LANGUAGE.md) (grundsyntax).

---

## JavaScript-paritet

### ✅ Finns (v2.2)

| Kategori | Funktion |
|----------|----------|
| Variabler | `let`, **`const`** (immutable) |
| Funktioner | `fn`, `return`, nästlade funktioner, **`fn id<T>(x: T) -> T`** (native generics v1, monomorphisering; inferens från literals **och variabler** — Rust + self-host; **kab-only** `id<Number>(42)` / `id$Number` / `id("hi")` / `id$String` / `id(id(42))` / `pair$Number_String` / `len(pair(1, "a"))` / `id(b)` / `id$Box` / `pair(x, s)` / `len(wrap(1))`), **`fn f(a, b = 3)`** / **`(a, b = 3) =>`** / **`class` method `fn add(a, b = 3)`** / **`{ add(a, b = 3) {} }`** / **`trait` default `fn add(a, b = 3)`**, **`fn f(a, ...xs)`** / **`(a, ...xs) =>`** / **`fn rest(a, ...xs)`** / **`{ rest(a, ...xs) {} }`** (self-host + Kab-VM; rust `try_compile` vägrar defaults/rest) |
| Kontroll | `if`/`else`, `while`, **`do { } while`** (host + self-host + Kab-VM), **`for x in xs`** / **`for x of xs`** (host + self-host + Kab-VM), **`for let i = 0; …`** (host + self-host + Kab-VM), **`switch`** + **`fallthrough`** (host + self-host + Kab-VM), **`match 1 { 1 => 2, _ => 0 }`** / **`match [x, y]`** / **`match { p, q }`** / **`match 1..=5`** / **`n @ 1..=5`** / **`1 | 2 | 3`** / **`..5`** / **`5..`** / **`[h, ...t]`** / **`{ k, ...s }`** / **`[h, ...mid, last]`** / **`n @ 1..=5 if n != 3`** / **`Color.Red`** / **`Msg.Move(p)`** / **`xs @ [p, q]`** / **`wrap @ { k, ...s }`** / **`{ k: n @ 1..=5 }`** / **`[n @ 1, ...r]`** / **`Ok(n @ 1..=5)`** / **`Some(n @ 1..=5)`** / **`(1 | 2)`** / **`Option.Some(n)`** / **`Option.Some("x")`** / **`Option<Number>.None`** / **`1.0..=2.0`** / **`Result.Ok(n)`** / **`Result<Number, String>.Err`** / **`n @ 1 | 2`** / **`v @ Msg.Move(x)`** (host + self-host + Kab-VM `jump_unless_const_eq` / `jump_unless_array` / `jump_unless_has_member` / `ge`/`le` / `array_slice_rest` / `object_rest` / `index_peek_from_end` / `jump_if_false` / `jump_unless_enum_variant` / `unpack_enum_fields` / `unwrap_result_ok` / `unwrap_option_some`), **`if let Some(x)`** / **`while let Ok(v)`** / **`if let 1 | 2`** / **`while let 1 | 2`** / **`if let n @ Some(x)`** (host + self-host + Kab-VM), `break`, `continue`, **`pass`/`assert`/`not`/`raise`** (host + self-host + Kab-VM), **`with`/`is`/`is not`** (self-host + Kab-VM `object_is`) |
| Operatorer | `+ - * / %`, **`**`**, `== != < <= > >=`, **`in`** (membership; host + self-host + Kab-VM), `&& \|\|`, **`??`** (host + self-host + Kab-VM), **`+= -= *= /= %= &= |= ^= <<= >>= >>>= **=`** (inkl. **`n **= 2`**, **`o.x **= 2`**, **`xs[0] **= 2`**, **`this.n **= 2`**, **`super.n **= 2`**, **`o.a.b **= 2`**, **`xs[0].x **= 2`**, **`o.items[0] **= 2`** / **`o.items[0][0] **= 2`** / **`xs[0][0].x **= 2`** / **`o.items[0][0].x **= 2`** / **`xs[0][0] **= 2`** / **`xs[0][0][0] **= 2`** / **`n %= 7`** / **`o.x %= 7`** / **`xs[0] %= 7`** / **`this.n %= 7`** / **`super.n %= 7`** / **`o.a.b %= 7`** / **`xs[0].x %= 7`** / **`o.items[0] %= 7`** / **`o.items[0][0] %= 7`** / **`xs[0][0].x %= 7`** / **`o.items[0][0].x %= 7`** / **`xs[0][0] %= 7`** / **`xs[0][0][0] %= 7`** / **`n -= 2`** / **`o.x -= 2`** / **`xs[0] -= 2`** / **`this.n -= 2`** / **`super.n -= 2`** / **`o.a.b -= 2`** / **`xs[0].x -= 2`** / **`o.items[0] -= 2`** / **`o.items[0][0] -= 2`** / **`xs[0][0].x -= 2`** / **`o.items[0][0].x -= 2`** / **`xs[0][0] -= 2`** / **`xs[0][0][0] -= 2`** / **`n *= 3`**, **`n <<= 1`**, **`n >>= 1`**, **`n >>>= 1`**, **`o.x <<= 1`**, **`o.x >>= 1`**, **`o.x >>>= 1`**, **`xs[0] <<= 1`**, **`xs[0] >>= 1`**, **`xs[0] >>>= 1`**, **`this.n <<= 1`**, **`this.n >>= 1`**, **`this.n >>>= 1`**, **`super.n <<= 1`**, **`super.n >>= 1`**, **`super.n >>>= 1`**, **`o.a.b <<= 1`**, **`o.a.b >>= 1`**, **`o.a.b >>>= 1`**, **`xs[0].x <<= 1`**, **`xs[0].x >>= 1`**, **`xs[0].x >>>= 1`**, **`o.items[0] <<= 1`**, **`o.items[0] >>= 1`**, **`o.items[0] >>>= 1`**, **`o.items[0][0] <<= 1`**, **`o.items[0][0] >>= 1`**, **`o.items[0][0] >>>= 1`**, **`xs[0][0].x <<= 1`**, **`xs[0][0].x >>= 1`**, **`xs[0][0].x >>>= 1`**, **`o.items[0][0].x <<= 1`**, **`o.items[0][0].x >>= 1`**, **`o.items[0][0].x >>>= 1`**, **`xs[0][0] <<= 1`**, **`xs[0][0] >>= 1`**, **`xs[0][0] >>>= 1`**, **`xs[0][0][0] <<= 1`**, **`xs[0][0][0] >>= 1`**, **`xs[0][0][0] >>>= 1`**, **`xs[i] +=`**, **`xs[0] |=`**, **`xs[0] &=`**, **`xs[0] ^=`**, **`o.x +=`**, **`o.x &=`**, **`o.x |=`**, **`o.x ^=`**, **`this.n &=`**, **`this.n |=`**, **`this.n ^=`**, **`super.n |=`**, **`super.n &=`**, **`super.n ^=`**, **`o.a.b &=`**, **`o.a.b |=`**, **`o.a.b ^=`**, **`o.a.b +=`**, **`o.items[0] +=`**, **`o.items[0] &=`**, **`o.items[0] |=`**, **`o.items[0] ^=`**, **`o.items[0][0] +=`**, **`o.items[0][0] &=`**, **`o.items[0][0] |=`**, **`o.items[0][0] ^=`**, **`xs[0].x +=`**, **`xs[0].x &=`**, **`xs[0].x |=`**, **`xs[0].x ^=`**, **`xs[0][0].x +=`**, **`xs[0][0].x &=`**, **`xs[0][0].x |=`**, **`xs[0][0].x ^=`**, **`o.items[0][0].x +=`**, **`o.items[0][0].x &=`**, **`o.items[0][0].x |=`**, **`o.items[0][0].x ^=`**, **`xs[0][0] +=`**, **`xs[0][0] &=`**, **`xs[0][0] |=`**, **`xs[0][0] ^=`**, **`xs[0][0][0] +=`**, **`xs[0][0][0] &=`**, **`xs[0][0][0] |=`**, **`xs[0][0][0] ^=`** host + self-host + Kab-VM), **`&&= \|\|= ??=`** (inkl. **`o.x ||= `** / **`o.x &&=`** / **`o.x ??=`** / **`xs[0] ||= `** / **`xs[0] ??=`** / **`o.a.b ??=`** / **`o.items[0] ||= `** / **`xs[0].x ??=`** / **`xs[0][0] ||= `** / **`this.n ||= `** / **`o.items[0][0] ||= `** / **`xs[0][0].x ??=`** / **`super.n ||= `** / **`o.items[0][0].x ??=`** / **`xs[0][0][0] ||= `** / **`Child<T> super.n ||= `** host + self-host + Kab-VM), **`!`**, **`? :`** (host + self-host + Kab-VM), **`& \| ^ ~ << >> >>>`** |
| Data | array `[1,2]`, **objekt `{ a: 1 }`**, strängar `"..."`, **template `` `Hej ${name}` ``** (host + self-host + Kab-VM) |
| Åtkomst | **`arr[i]`**, **`obj.key`**, **`obj["key"]`**, **`.length`** |
| Array-API | **`map`**, **`filter`**, **`push`**, **`pop`**, **`reduce`**, **`find`**, **`slice`**, **`sort`**, **`reverse`**, **`join`**, **`shift`**, **`unshift`**, **`splice`**, **`for_each`**, **`concat`**, **`flat`**, **`flat_map`**, **`len`**, **`includes`**, **`some`**, **`every`**, **`index_of`**, **`last_index_of`**, **`find_last`**, **`find_last_index`** |
| Math | **`floor`**, **`ceil`**, **`round`**, **`abs`**, **`min`**, **`max`**, **`sqrt`**, **`pow`**, **`random`**, **`sign`**, **`trunc`**, **`clamp`**, **`pi`**, **`e`**, **`log`/`log2`/`log10`**, **`exp`**, **`sin`/`cos`/`tan`**, **`asin`/`acos`/`atan`/`atan2`**, **`hypot`**, **`cbrt`** |
| Number | **`parse_int`**, **`parse_float`**, **`is_finite`** |
| Object | **`assign`**, **`has_key`**, **`delete_prop`**, **`clone_shallow`**, **`keys`** / **`object_keys`**, **`object_values`**, **`object_entries`**, **`values`**, **`entries`** |
| Promise | **`promise_new`** / **`promise`**, **`promise_resolve`**, **`promise_reject`**, **`is_promise`**, **`promise_then`**, **`promise_catch`**, **`promise_finally`**, **`await_all`** / **`promise_all`**, **`promise_race`**, **`promise_any`**, **`promise_all_settled`** |
| Console | **`println`**, **`log`** |
| Utility | **`typeof()`**, **`keys()`**, **`values()`**, **`entries()`**, `is_null`, `is_undefined`, `is_nan`, **`is_array`** / **`array_is_array`**, **`type_assert`** |
| JSON | **`json_parse`**, **`json_stringify`**, `import "std"` |
| Collections | **`map_new`/`map_get`/`map_set`**, **`map_keys`/`map_values`/`map_entries`**, **`map_has`/`map_delete`/`map_clear`**, **`map_for_each`**, **`set_new`/`set_add`**, **`set_has`/`set_delete`/`set_values`**, **`set_for_each`** |
| Sträng | **`trim`**, **`split`**, **`starts_with`**, **`replace`**, **`repeat`**, **`pad_start`**, **`pad_end`**, **`char_at`**, **`str_includes`**, **`str_last_index_of`**, **`trim_start`**, **`trim_end`** |
| URI | **`encode_uri`**, **`decode_uri`**, **`encode_uri_component`**, **`decode_uri_component`** |
| Date | **`date_now`**, **`date_parse`**, **`date_format`** (objekt med `ms`, `year`, …) |
| Timers | **`sleep_ticks`** (scheduler-ticks), **`sleep_ms`**, **`set_timeout`**, **`clear_timeout`**, **`set_interval`**, **`clear_interval`** (ms = wall-clock) |
| RegExp | **`regex_test`**, **`regex_match`**, **`regex_replace`** |
| Kommentarer | **`//` radkommentar** |
| Destructuring | **`let [a, b] = xs`**, **`let { name, age } = obj`**, **`...rest`**, nested **`let { x: [a, b] }`** (Kab-VM **`array_slice_from`/`array_slice_rest`/`object_rest`**; self-host **`let [a, ...rest]`** / **`let { a, ...rest }`** / **`let { x: [a, b] }`**) |
| Spread | **`...arr`** i array/objekt/call (Kab-VM **`new_instance_from_array`** / **`concat_array`** / **`merge_object`**; self-host parse/emit `...` i anrop, **`[1, ...xs]`**, **`{ ...obj }`**, shorthand **`{ a }`**, computed **`{ [k]: v }`**) |
| Klassisk `for` | **`for let i = 0; i < n; i = i + 1`** (host + self-host + Kab-VM) |
| `try`/`catch` | **`try { } catch (e) { }`** på `Result` (`Ok`/`Err`); **`raise`/`throw`** (host + self-host + Kab-VM) |
| Pilfunktioner | **`(a, b) => a + b`**, **`(a, b = 3) =>`**, **`(a, ...xs) =>`**, block-kropp `{ return ... }`, objekt-metod **`{ foo() {} }`** / **`{ add(a, b = 3) {} }`** / **`{ rest(a, ...xs) {} }`** (self-host + Kab-VM `make_arrow_fn`) |
| `async`/`await` | **`async fn`** (self-host `fn_async`; Kab-VM kör kroppen sync och wrappar med **`promise_resolve`** — `sh6_self_host_async_fn_ok`), **`for await x of xs`** i `async fn` (array via `async_iterator_begin` + `async_iterator_step_in_place` — `sh6_self_host_for_await_array_ok`), **`async (n) => ...`** (host), **`await`** (Kab-VM via `await_all`) |
| Typer | `null`, `undefined`, `NaN`, `true`/`false`, heltal, flyttal |

### ✅ JS-paritet (stdlib våg 1–4)

| Funktion | JS-motsvarighet | Kabootar |
|----------|-----------------|----------|
| `switch` | `switch` | **`switch (x) { case 1: { } default: { } }`** + explicit **`fallthrough`**; Kab-VM match + default + fallthrough |
| `for…of` / `for…in` | värden vs index | **`for x of xs`** (host + self-host + Kab-VM); **`for i in xs`** index / **`for k in obj`** nycklar (host + self-host + Kab-VM) |
| `**` / `??` | operatorer | **`**`**, **`??`** (host + self-host + Kab-VM) |
| `& \| ^ ~ << >> >>>` | bitwise | **bitwise-operatorer** (ToInt32 / ToUint32 för `>>>`) |
| `do…while` | loop | **`do { } while`** (self-host + Kab-VM) |
| `Object.*` / array | helpers | **`assign`**, **`at`**, **`fill`**, **`to_spliced`**, **`to_fixed`**, … |
| `encodeURI` / timers | URL + timeout | **`encode_uri`**, **`set_timeout`**, **`set_interval`**, **`sleep_ms`** (wall-clock ms) |
| `Date` (enkel) | timestamp | **`date_now`**, **`date_format`**, **`date_iso`** |

### ✅ JS-paritet (stdlib våg 5)

| Funktion | JS-motsvarighet | Kabootar |
|----------|-----------------|----------|
| `Array.from` / `with` | array helpers | **`array_from`**, **`array_with`** |
| `Object.fromEntries` | objekt | **`object_from_entries`** |
| `structuredClone` | djup klon (array/objekt) | **`structured_clone`** |
| `Number.isInteger` | heltalstest | **`is_integer`** |
| `toExponential` / `toPrecision` | talformat | **`to_exponential`**, **`to_precision`** |
| `Math.log/sin/cos/…` | fler Math | **`log`**, **`sin`**, **`cos`**, **`hypot`**, **`pi`**, **`e`**, … |
| String | kodpunkter | **`char_code_at`**, **`from_char_code`** |
| RegExp | global ersätt | **`regex_replace_all`** |
| JSON | pretty-print | **`json_stringify(v, indent)`** |
| `switch` | flera case-labels | **`case 1: case 2: { }`** |
| `instanceof` | klasscheck | **`instanceof(obj, "Class")`** / **`is(obj, "Class")`** (host + self-host + Kab-VM) |
| `console` | loggning | **`console_log`**, **`console_warn`**, **`console_error`** |

### ✅ JS-paritet (stdlib våg 6)

| Funktion | JS-motsvarighet | Kabootar |
|----------|-----------------|----------|
| `Math.asin/acos/atan/atan2` | trigonometri | **`asin`**, **`acos`**, **`atan`**, **`atan2`** |
| `Map` | värden/iteration | **`map_values`**, **`map_has`**, **`map_delete`**, **`map_clear`**, **`map_for_each`** |
| `Set` | värden/iteration | **`set_delete`**, **`set_clear`**, **`set_values`**, **`set_for_each`** |
| Bytecode | bitwise/switch/do-while | **`& \| ^ ~ << >> >>>`**, **`switch`+`fallthrough`**, **`do while`** kompileras |

### ✅ JS-paritet (stdlib våg 7)

| Funktion | JS-motsvarighet | Kabootar |
|----------|-----------------|----------|
| `Promise.resolve/reject` | skapa promise | **`promise_resolve`**, **`promise_reject`** (`reject` → `Result::Err`) |
| `Promise.all/race/any/allSettled` | kombinera | **`promise_all`** (returnerar promise), **`await_all`** (synkront array), **`promise_race`**, **`promise_any`**, **`promise_all_settled`** |
| `Promise.then/catch/finally` | kedja | **`promise_then`**, **`promise_catch`**, **`promise_finally`** |
| `new Promise(executor)` | konstruktor | **`promise_new`** / **`promise`** |
| `await` + reject | fel vid rejection | **`await`** kastar vid `promise_reject` |
| `is_promise` | typkontroll | **`is_promise`** |
| `Map.fromEntries` | map från par | **`map_from_entries`** |
| `Number.isSafeInteger` | säkert heltal | **`is_safe_integer`** |
| `Math.fmod` | flyttalsmodulo | **`fmod`** |

### ✅ JS-paritet (stdlib våg 8)

| Funktion | JS-motsvarighet | Kabootar |
|----------|-----------------|----------|
| `Array.prototype.reduceRight` | höger-till-vänster fold | **`reduce_right`** |
| `Array.prototype.toReversed` / `toSorted` | icke-muterande kopia | **`to_reversed`**, **`to_sorted`** |
| Set union/intersection/difference | mängdalgebra | **`set_union`**, **`set_intersection`**, **`set_difference`**, **`set_is_subset`** |
| `Math.imul` / `clz32` / `fround` | 32-bit heltal / biträkning / f32 | **`imul`**, **`clz32`**, **`fround`** |
| `Number.prototype.toString` | tal → sträng | **`number_to_string`** |
| `String.prototype.split` m.fl. | alias för sträng-API | **`string_split`**, **`string_to_lower`**, **`string_to_upper`**, **`string_replace`**, **`string_replace_all`** |

### ✅ JS-paritet (stdlib våg 9)

| Funktion | JS-motsvarighet | Kabootar |
|----------|-----------------|----------|
| `Array.prototype.flat` m.fl. | array-alias | **`array_flat`**, **`array_flat_map`**, **`array_includes`**, **`array_find_index`**, **`array_index_of`**, **`array_concat`**, **`array_slice`**, **`array_reduce`**, **`array_reduce_right`**, **`array_join`**, **`array_sort`**, **`array_reverse`** |
| `Object.hasOwn` / `delete` / `is` | objekt-alias | **`object_has`**, **`object_has_key`**, **`object_delete`**, **`object_delete_prop`**, **`object_is`**, **`object_clone_shallow`**, **`object_keys`** (även i object-modulen) |
| `Set.isSupersetOf` / `isDisjointFrom` | mängd-relationer | **`set_is_superset`**, **`set_is_disjoint`** |

### ✅ JS-paritet (stdlib våg 10)

| Funktion | JS-motsvarighet | Kabootar |
|----------|-----------------|----------|
| `fetch` | HTTP-anrop | **`fetch(url, { method, body, headers })`** → promise med `{ status, ok, body, headers }` |
| `Response` helpers | läsa svar | **`response_text`**, **`response_json`**, **`response_ok`** |
| `queueMicrotask` | microtask | **`queue_microtask(fn, ...args)`** |
| `?.` optional chaining | null-säker åtkomst | **`obj?.field`**, **`obj?.[i]`**, **`fn?.()`** — host + self-host + Kab-VM (`__opt_member` / `__opt_index`; `?.()` via `jump_if_not_nullish` + `call`) |
| `delete obj.key` | ta bort property | **`delete o.x`** / **`delete o.a.b`** / **`delete xs[0].x`** / **`delete o[k]`** / **`delete o.items[0].x`** / **`delete xs[0][0].x`** / **`delete this.z`** / **`delete o.items[0][0].x`** / **`delete o.a.b.c`** / **`delete this.a.b`** / **`delete o[k].x`** / **`delete super.z`** / **`delete this[k]`** / **`delete super[k]`** / **`delete super.a.b`** / **`delete o[k][j]`** / **`delete super.a[k]`** / **`delete this[k].x`** / **`delete super[k].x`** / **`delete this.a[k]`** / **`delete o.a[k]`** / **`delete this[k][j]`** / **`delete super[k][j]`** / **`delete o.items[0][k]`** / **`delete this.a.b[k]`** / **`delete o.a.b[k]`** / **`delete xs[0][0][k]`** / **`delete super.a.b[k]`** / **`delete this.items[0][k]`** / **`delete super.items[0][k]`** / **`delete this.items[0][0][k]`** / **`delete o.items[0][0][k]`** / **`delete super.items[0][0][k]`** (syntax; **self-host** + Kab-VM → `object_delete_prop` + store-back; rust `try_compile` vägrar `delete`) |
| `Array.map/filter` alias | array | **`array_map`**, **`array_filter`**, **`array_find`**, **`array_some`**, **`array_every`**, **`array_of`** |
| `Set.symmetricDifference` | mängd | **`set_symmetric_difference`** |
| `String.codePointAt` | unicode | **`code_point_at`**, **`from_code_point`** |
| `Date` objekt | timestamp | **`date_new`**, **`date_get_time`**, **`date_get_full_year`**, **`date_set_time`**, **`date_to_iso_string`** |
| `Object.freeze/seal` | immutabilitet | **`object_freeze`**, **`object_seal`**, **`object_prevent_extensions`**, **`object_is_frozen`**, **`object_is_sealed`**, **`object_define_property`** |
| `Math.log1p/expm1` | flyttal | **`log1p`**, **`expm1`** |

### ✅ JS-paritet (stdlib våg 11)

| Funktion | JS-motsvarighet | Kabootar |
|----------|-----------------|----------|
| `Object.groupBy` / `Map.groupBy` | ES2024 gruppering | **`object_group_by`**, **`map_group_by`** |
| `Promise.withResolvers` | ES2024 promise-deferred | **`promise_with_resolvers`** → `{ promise, resolve, reject }` |
| `AbortController` | avbryt `fetch` | **`abort_controller_new`**, **`abort_controller_abort`**, `fetch({ signal })` |
| `for…of` Map/Set | iteration | **`for x of set`**, **`for pair of map`** (entries som `[key, val]`) |

### ✅ JS-paritet (stdlib våg 12)

| Funktion | JS-motsvarighet | Kabootar |
|----------|-----------------|----------|
| `throw` / `Error` | undantag | **`throw expr`**, **`error_new`**, **`type_error`**, **`is_error`** — fångas av **`try/catch`** |
| `Promise.try` | ES2025 | **`promise_try(fn)`** — sync throw → rejected promise |
| `Object.groupBy` alias | kortnamn | **`group_by`** → samma som **`object_group_by`** |
| `Iterator.from` | iterable → array | **`iterator_from(iterable)`** |
| `URL` / `URLSearchParams` | URL-parsing | **`url_new`**, **`url_search_params_new`**, **`usp_get`/`set`/`to_string`** |

### ✅ JS-paritet (stdlib våg 13)

| Funktion | JS-motsvarighet | Kabootar |
|----------|-----------------|----------|
| `try/finally` | cleanup-block | **`try { } catch (e) { } finally { }`** |
| Default/rest params | `fn(a=1, ...rest)` | **`fn f(a, b = 2, ...xs)`**, **`(a, b = 3) =>`**, klassmetod **`fn add(a, b = 3)`** / **`fn rest(a, ...xs)`**, objekt-metod **`{ add(a, b = 3) {} }`** / **`{ rest(a, ...xs) {} }`**, trait default **`fn add(a, b = 3)`** / **`fn rest(a, ...xs)`** |
| `/* */` | blockkommentar | **`/* ... */`** i lexer |
| `globalThis` | global referens | **`globalThis()`** / **`global_this()`** → snapshot av globals |
| `TextEncoder`/`Decoder` | UTF-8 bytes | **`text_encode`**, **`text_decode`**, **`btoa`**, **`atob`** |
| `performance.now` | monotonic high-res ms | **`performance.now()`** — `DOMHighResTimeStamp` (float ms since runtime start) |
| `crypto.getRandomValues` | CSPRNG fill | **`crypto.getRandomValues(array)`** — `Array` (0..255) or **`Uint8Array`** (`uint8_array_new`); max 65536 bytes |
| `BigInt` | arbitrary integers | **`123n`**, **`BigInt("99")`**, **`BigInt(42)`** — `+ - * / % **`**, unary `-`; no mix with `number` in arithmetic; **`typeof(1n)`** → **`"bigint"`** |
| Private class fields | `#x`, private methods | **`class C { #n: number = 0; fn get() { return this.#n } }`** — lexical access from declaring class methods only |
| `RegExp` | flags, dotAll, lookbehind | **`regexp_new("a.b", "s")`**, **`.flags`**, **`.dotAll`**, **`regexp_test`**, **`regexp_exec`**, **`is_regexp`** — `regex_*` literals **`/pat/flags`**; lookbehind **`(?<=…)`**; **`RegExp_escape`** / **`regex_escape`** |
| `Date` (full) | UTC/local, timezone | **`date_new`**, **`Date_now`**, **`getUTC*`** / **`get*`** getters, **`setUTC*`** / **`set*`**, **`date_get_timezone_offset`**, **`date_to_iso_string`**, **`date_parse`** (ISO) |
| Typed arrays | Float32/64, Uint8, Int32, DataView, SAB | **`array_buffer_new`**, **`shared_array_buffer_new`**, **`float32_array_*`**, **`float64_array_*`**, **`uint8_array_*`**, **`int32_array_*`**, **`data_view_*`** — P2: Float32→`createBuffer`, Uint8→PCM LE i16 + `texImage2D` staging ([GAME.md](GAME.md), [XR.md](XR.md)) |
| Proxy/Reflect | traps, construct | **`Proxy`**, **`is_proxy`**, **`Reflect.isProxy`**, **`Reflect.construct`** |
| WeakMap / WeakSet | weak keys | **`weak_map_new/set/get/has/delete`**, **`weak_set_new/add/has/delete`**, **`is_weakmap`**, **`is_weakset`** |
| `using` / modules | explicit dispose, `import.meta`, dynamic import | **`using x = expr;`** (dispose via **`Symbol.dispose`**, **`dispose()`**, **`close()`**; **self-host** + **Kab-VM** instans-heap: `close()` this-write syns på anroparen — `sh6_self_host_using_class_close_writeback_ok`), **`import.meta.url`**, **`import.meta.path`** (self-host → `import_meta()`; Kab-VM), **`import("math")`** → Promise of module namespace (self-host → `dynamic_import`; **kab-only** `await import("math")` + `ns.add` — `sh6_self_host_dynamic_import_math_ok`) |
| `Intl` | `NumberFormat`, `DateTimeFormat` | **`Intl.NumberFormat(locale, opts).format(n)`**, **`Intl.DateTimeFormat(locale, opts).format(date)`** — decimal/percent/currency, grouping, date/time styles |
| `Temporal` | polyfill subset | **`Temporal.PlainDate.from({ year, month, day })`**, **`Temporal.Instant.from(ms)`**, **`Temporal.Now.instant()`**, **`Temporal.Now.plainDateISO()`** |
| Error `.cause` / `.stack` | chained errors, stack traces | **`error_new(msg, { cause: err })`**, **`error_cause(e)`**, **`error_stack(e)`** — auto stack on **`throw`** |
| `Iterator.map/filter` | iterator helpers | **`iterator_map`**, **`iterator_filter`**, **`iterator_take`**, **`iterator_skip`**, **`iterator_chain`**, **`iterator_zip`**, **`iterator_enumerate`**, **`iterator_flat_map`**, **`iterator_drop_while`**, **`iterator_take_while`**, **`iterator_accumulate`**, **`iterator_pairwise`** return **lazy iterators** (use with **`for…of`**)
| `RegExp.escape` | säkra regex-strängar | **`regex_escape`** |
| Fler `Error` | undantagstyper | **`reference_error`**, **`range_error`** |

### ✅ JS-paritet (iterator protocol)

| Funktion | JS-motsvarighet | Kabootar |
|----------|-----------------|----------|
| `Symbol.iterator` / `{ next() }` | sync iteration | **`for x of iterable`**, **`iterator_begin`**, **`iterator_step`**, custom **`{ next() { return { value, done } } }`** — **`value: null`** när **`done: true`** (inte `undefined`) |
| `fn*` / `yield` | generator functions | **`fn* gen() { yield 1; yield* [2, 3]; return 99 }`**, **`gen()`** → iterator, **`.next()`**; self-host + Kab-VM **`.next().value`** (`sh6_self_host_generator_yield_ok`); **`return 99`** completion (`sh6_self_host_generator_return_ok`); **`g.return(v)`** close (`sh6_self_host_generator_method_return_ok`); **`yield* [1, 2]`** (`sh6_self_host_yield_star_array_ok`); **`yield* inner()`** (`sh6_self_host_yield_star_generator_ok`); **`for x of gen()`** (`sh6_self_host_for_of_generator_ok`); **`.throw()`** host |
| `Symbol.asyncIterator` | async iteration | **`for await x of iterable`** (inside **`async fn`** only) |
| `async fn*` | async generators | **`async fn* gen() { yield 1 }`** — **`yield*`** över async/sync; **`await`** i kroppen; **`for await…of`**; **`.next()` / `.throw()` / `.return()`** return **Promises**; **`.throw(e)`** resumes **`catch`** around **`yield`** when present; **`.return()`** / **`.throw()`** forwarded through active **`yield*`**; **`gen[Symbol.asyncIterator]()`** returnerar **`gen`** |
| `Iterator` / `AsyncIterator` classes | static + instance adapters | **`Iterator.from`**, **`Iterator.fromAsync`**, **`Iterator.zip`**, **`it.map(fn)`**, **`it.zip`**, **`it.enumerate()`**, **`it.chain(...)`**; **`AsyncIterator.from`**, **`for await…of`** over lazy chains; iterator-objekt: **`it[Symbol.iterator]()`** / **`it[Symbol.asyncIterator]()`** returnerar **`it`** |
| Lazy `Iterator.map/filter/take/skip/chain/zip/enumerate/flatMap/dropWhile/takeWhile` | streaming adapters | class methods + **`iterator_*`** natives — lazy; consume with **`for…of`** / **`for await…of`**; **`flatMap(fn, depth?)`** default depth **`1`** |
| Iterator terminals | consuming helpers | **`.toArray()`**, **`.reduce()`**, **`.some()`**, **`.every()`**, **`.forEach()`**, **`.find()`**, **`.findIndex()`**, **`.includes()`**; **`iterator_to_array`**, **`iterator_reduce`**, **`iterator_for_each`**, **`iterator_find`**, **`iterator_find_index`**, **`iterator_includes`** |
| Iterator `return()` / `throw()` | early close | **`it.return(value)`**, **`it.throw(reason)`** — generator **`.throw(e)`** med **`try/catch`** runt **`yield`** resume:ar catch-blocket; utan catch stängs generatorn; under **`yield*`** vidarebefordras till delegat; **`break`** / **`return`** / **`raise`** i **`for…of`** / **`for await…of`** anropar **`iterator_close`**; async generators och **`AsyncIterator.throw`** samma semantik via Promises |
| `AsyncIterator` static adapters | async lazy factories | **`AsyncIterator.map`**, **`filter`**, **`take`**, **`skip`**, **`flatMap`**, **`dropWhile`**, **`takeWhile`**, **`zip`**, **`enumerate`**, **`chain`** — native async lazy (not sync-delegate-only) |
| `array_from_async` | materialize async iterable | **`array_from_async(iterable)`** → Promise of array (like **`Array.fromAsync`**) |
| `Iterator.fromAsync` | async iterable → async iterator | **`iterator_from_async(asyncIterable)`** / **`Iterator.fromAsync(asyncIterable)`** |

### ✅ Deno-paritet (runtime våg 1)

| Deno | Kabootar |
|------|----------|
| `Deno.env` | **`env_get`**, **`env_set`**, **`env_has`**, **`env_delete`**, **`env_to_object`** |
| `Deno.serve` | **`serve_handler`**, **`serve`**, **`response_new`**, **`request_*`** |
| Streams (förenklad) | **`stream_from_array`**, **`stream_read`**, **`stream_read_all`** |
| WebSocket (in-process) | **`ws_channel_pair`**, **`ws_link`**, **`ws_send`**, **`ws_recv`** |

### ✅ Deno-paritet (runtime våg 2)

| Deno | Kabootar |
|------|----------|
| `Deno.cwd()` | **`cwd`**, **`Deno_cwd`** |
| `Deno.readTextFile` | **`read_text_file`**, **`Deno_readTextFile`** |
| `Deno.writeTextFile` | **`write_text_file`**, **`Deno_writeTextFile`** |
| `ReadableStream` | **`stream_new`**, **`stream_from_string`**, **`stream_cancel`** |
| `WritableStream` | **`writable_stream_new`**, **`writable_write`**, **`writable_close`**, **`writable_read_all`** |
| `WebSocket` (TCP) | **`ws_connect`**, **`Deno_connect`** (`ws://`) |

### ✅ Deno-paritet (runtime våg 3)

| Deno | Kabootar |
|------|----------|
| `wss://` WebSocket | **`ws_connect("wss://...")`** med `tls_ca_only` / `tls_add_ca` |
| `Deno.listen` / `Deno.connect` | **`tcp_listen`**, **`tcp_connect`**, **`tcp_accept`**, **`tcp_read`**, **`tcp_write`**, **`tcp_close`** |
| `Deno.run` | **`deno_run`**, **`Deno_run`** |
| `stream.tee` / `pipeTo` | **`stream_tee`**, **`stream_pipe_to`** |

### ✅ Deno-paritet (runtime våg 4)

| Deno | Kabootar |
|------|----------|
| UDP sockets | **`udp_bind`**, **`udp_send`**, **`udp_recv`**, **`udp_close`**, **`udp_local_addr`** |
| `Deno.Command` | **`run_command`**, **`Deno_command`** |
| `Deno.chdir` | **`chdir`**, **`Deno_chdir`** |
| `Deno.resolveDns` | **`resolve_dns`**, **`Deno_resolveDns`** |
| Stream backpressure | **`stream_locked`**, **`stream_lock`**, **`stream_desired_size`**, **`writable_desired_size`** |

### ✅ Deno-paritet (runtime våg 5)

| Deno | Kabootar |
|------|----------|
| Async Web Streams | **`stream_read_async`**, **`stream_read_all_async`**, **`stream_pipe_to_async`** |
| Unix sockets | **`unix_connect`**, **`unix_listen`**, **`unix_accept`**, **`unix_read`**, **`unix_write`**, **`unix_close`** |
| `Deno.openKv` | **`open_kv`**, **`Deno_openKv`**, **`kv_get`**, **`kv_set`**, **`kv_delete`**, **`kv_list`**, **`kv_close`** (Kabootar SQL + WAL) |
| Workers | **`worker_new`**, **`worker_start`**, **`worker_post_message`**, **`worker_recv`**, **`worker_terminate`** (in-process på wasm32) |
| FFI | **`ffi_load`**, **`ffi_call`**, **`ffi_close`** |
| npm / TS | **`npm_install`**, **`npm_import`**, **`ts_transpile`**, **`ts_strip_types`** |

### ✅ Deno-paritet (runtime våg 7)

| Deno | Kabootar |
|------|----------|
| `Kv.watch` | **`kv_watch`** → ReadableStream (`{ kind, key, value }`) |
| `Kv.atomic` | **`kv_atomic`** — `set` / `delete` / `get` / `check` i SQL-transaktion |

### ✅ Deno-paritet (runtime våg 8)

| Deno | Kabootar |
|------|----------|
| Delad DB-fil | **`open_kv_db()`** + **`db_open(path)`** delar SQL-motor |
| Versionstamps | **`kv_get_entry`**, **`kv_get_version`**, **`check` + `version`** |
| `Kv.listen` | **`kv_listen`**, **`kv_listen_recv`**, **`kv_listen_close`** |
| Watch buffer | Ringbuffer (64 events) per watch-ström |

### ✅ Deno-paritet (runtime våg 9)

| Deno | Kabootar |
|------|----------|
| Atomic `sum` / `max` / `min` | **`kv_atomic`** ops `sum`, `max`, `min` |
| Queue | **`kv_enqueue`**, **`kv_dequeue`**, atomic `enqueue` |
| List med version | **`kv_list_entries`** |
| Async watch | **`kv_listen_async`**, **`kv_watch_async`** |

### ✅ Deno-paritet (runtime våg 10)

| Deno | Kabootar |
|------|----------|
| Worker isolat | **`worker_start`** kör i egen OS-tråd med separat `Environment` |
| `importScripts` | **`importScripts(path, ...)`** i worker-kontext |
| Worker från fil | **`worker_start_file(worker, path)`** |
| Vänta på worker | **`worker_join(worker)`** |
| Async recv | **`worker_recv_async(worker)`** → Promise |

### ✅ Deno-paritet (runtime våg 11)

| Deno | Kabootar |
|------|----------|
| `onmessage` (worker) | **`onmessage(handler)`** i worker-tråd |
| `postMessage` (worker) | **`postMessage(msg)`** alias för `worker_reply` |
| `worker.onmessage` | **`worker_onmessage(worker, handler)`** — anropas vid `worker_recv` |
| Async poll | **`worker_poll_async(timeout_ms?)`** i worker-tråd |
| Blockerande poll | **`worker_poll_wait(timeout_ms?)`** |
| Meddelandeloop | **`worker_run_message_loop()`** i worker-tråd |

### ✅ Deno-paritet (runtime våg 12)

| Deno | Kabootar |
|------|----------|
| `Deno.readFile` | **`read_file`**, **`Deno_readFile`** → byte-array |
| `Deno.writeFile` | **`write_file`**, **`Deno_writeFile`** |
| `Deno.readDir` | **`read_dir`**, **`Deno_readDir`** |
| `Deno.mkdir` | **`mkdir`**, **`Deno_mkdir`** |
| `Deno.stat` | **`stat`**, **`Deno_stat`** → objekt |
| `Deno.remove` | **`remove`**, **`Deno_remove`** |
| `Deno.exists` | **`exists`**, **`Deno_exists`** |

### ✅ Deno-paritet (runtime våg 13)

| Deno | Kabootar |
|------|----------|
| `Deno.startTls` | **`tcp_start_tls(socket, hostname)`**, **`Deno_startTls`** — rustls-klient |

### ✅ Deno-paritet (runtime våg 14)

| Deno | Kabootar |
|------|----------|
| `npm:` / `jsr:` imports | **`npm_fetch`**, **`jsr_fetch`**, **`npm_resolve`**, **`import "npm:…"`**, **`import "jsr:…"`** |
| npm registry | **`npm_install`** (lokal registry → npm → jsr), cache i **`.kabootar/npm/`** |
| JSR | **`jsr_fetch`**, cache i **`.kabootar/jsr/`** — `npm.jsr.io` |
| Spec parsing | **`npm_parse_spec`**, **`npm_list_cache`** |

Se [DENO.md](DENO.md) för full mappningstabell.

### ✅ Deno-paritet (runtime våg 15)

| Deno | Kabootar |
|------|----------|
| `ReadableStreamDefaultReader` | **`stream_get_reader`**, **`reader_read`**, **`reader_release_lock`**, **`reader_cancel`**, **`reader_read_async`** |
| `WritableStreamDefaultWriter` | **`writable_get_writer`**, **`writer_write`**, **`writer_close`**, **`writer_abort`**, **`writer_release_lock`** |
| `TransformStream` | **`transform_stream_new(transform_fn)`** → `{ readable, writable }` |
| Byte streams / BYOB | **`byte_stream_new`**, **`byte_stream_from_bytes`**, **`byte_stream_read`**, **`byte_stream_byob_read`** |
| `cancel` / `abort` | **`stream_abort`**, **`stream_cancel`**, **`writable_abort`** |
| Stream controller | **`stream_enqueue`**, **`stream_close_readable`**, **`stream_state`** |
| Transferable streams | **`stream_transfer`**, **`stream_from_transfer`**, **`worker_post_message(w, msg, [stream])`** |

### ✅ Deno-paritet (runtime våg 16)

| Deno | Kabootar |
|------|----------|
| `Deno.emit` / TS compile | **`ts_compile`**, **`ts_compile_file`**, **`Deno_emit`** → `{ code, diagnostics }` |
| Type erasure | **`ts_strip_types`**, **`ts_transpile`** — interface/type/enum/generics/modifiers (TS-källa, **inte** native Kabootar) |

### 🚧 Planerat (språk — generics fas 2)

| Feature | Milestone | Doc |
|---------|-----------|-----|
| Inferens från variabler (`let n = 42; id(n)`) | G6 ✅ + `id(Box)` | [GENERICS.md](GENERICS.md#fas-2--g6-planering) |
| Generiska klassmetoder (`fn echo<T>(x) { … }`) | G7 ✅ + två specs | [GENERICS.md](GENERICS.md#fas-2--g6-planering) |
| Generiska klasser (`class Box<T>`) | G8 ✅ + G8.1 `b.echo` + `extends Base<T>` + två specs + fält `T` + `Box<String>`; **kab-only** `echo$Number` / `echo$String` / `Box<String>("hi")` / `Child$Number` | [GENERICS.md](GENERICS.md#fas-2--g6-planering) |
| Generiska enum / `Option<T>` / `Result<T,E>` | G9 ✅ + två specs + `Result$Number_String` | [GENERICS.md](GENERICS.md#fas-2--g6-planering) |
| Self-host generics fas 2 | G10 ✅ | [GENERICS.md](GENERICS.md#fas-2--g6-planering) |
| LSP hover / completion för generics | G11 ✅ | [GENERICS.md](GENERICS.md#fas-2--g6-planering) |

### 🚧 Planerat (språk — generics fas 3)

| Feature | Status | Doc |
|---------|--------|-----|
| `match Option.Some(v)` i bytecode | ✅ + self-host compile-run; **kab-only** `Option.Some(n)` / `Option.Some("x")` / `Option<Number>.None` / `Result.Ok(n)` / `Result$Number_String` | [GENERICS.md](GENERICS.md#fas-3) |
| `class Child<T> extends Base<T>` | ✅ host + self-host compile-run; **kab-only** `Child<Number>().tag()` / `Child$Number` / `super.init` / `Child(42).val` / `super.count = 1` / `super.n += 2` | [GENERICS.md](GENERICS.md#fas-3) |
| `super.method()` | ✅ self-host `get_super_method` (inkl. `Child<T>` kab-only); **kab-only** `let m = super.tag; m()` / **`Child<T>` bound `super.tag`** / `this.run(super.f)` / `(super.f)()`; default = kab-only Kab-VM via kbcb v2 Uint8Array/mmap + tag-dispatch; `kabootar run` packed `.kbcb` | [LANGUAGE.md](LANGUAGE.md) |
| `super.init(...)` / `super.field =` / `super.n +=` | ✅ self-host compile-run; **kab-only** `super.init` / `Child(42).val` / `super.count = 1` / `super.n += 2` | [LANGUAGE.md](LANGUAGE.md) |
| `len(wrap(1))` / `len(pair(x, s))` | ✅ self-host `get_length` (nested call args on locals) | [ROADMAP.md SH3](ROADMAP.md) |
| Self-host `NewInstance` opcode | ✅ | [self_host/README.md](../self_host/README.md) |
| LSP hover member-call med receiver | ✅ | [GENERICS.md](GENERICS.md#fas-3) |

### ✅ Deno-paritet (runtime våg 17)

| Deno / Node | Kabootar |
|-------------|----------|
| `import "node:fs"` | **`readFileSync`**, **`writeFileSync`**, **`mkdirSync`**, **`statSync`**, **`readdirSync`**, … |
| `import "node:fs/promises"` | **`readFile`**, **`writeFile`**, … (sync-shim v17) |
| `import "node:path"` | **`join`**, **`resolve`**, **`dirname`**, **`basename`**, **`extname`**, **`normalize`**, **`sep`**, **`delimiter`** |
| `import "node:process"` | **`cwd`**, **`chdir`**, **`env`**, **`platform`**, **`arch`**, **`argv`** |
| `import "node:os"` | **`platform`**, **`arch`**, **`homedir`**, **`tmpdir`**, **`endianness`**, **`EOL`** |
| `import "node:url"` | **`parse`**, **`format`**, **`fileURLToPath`**, **`pathToFileURL`** |
| `import "node:buffer"` | **`from`**, **`alloc`**, **`isBuffer`** |
| `import "node:crypto"` | **`randomBytes`** |
| Discovery | **`node_resolve`**, **`node_list`**, **`node_import`** |

### ✅ Deno-paritet (runtime våg 18)

| Deno | Kabootar |
|------|----------|
| `SharedArrayBuffer` | **`sab_new`**, **`sab_byte_length`**, **`sab_transfer`**, **`sab_from_transfer`**, **`sab_is_shared`** |
| Typed arrays | **`uint8_array_new/get/set`**, **`int32_array_new/get/set`** |
| `Atomics` | **`atomics_load`**, **`atomics_store`**, **`atomics_add/sub/and/or/xor`**, **`atomics_exchange`**, **`atomics_compare_exchange`**, **`atomics_wait`**, **`atomics_notify`** |
| Worker transfer | **`worker_post_message(w, msg, [sab])`** — delad minne mellan trådar |

### ✅ Deno-paritet (runtime våg B — B1–B8)

| Deno | Kabootar |
|------|----------|
| `Deno.serve` (async stub) | **`serve_dispatch`**, **`serve_async_ready`** → Promise `{ port, ready, http2 }`; **`http2_supported`** → `false` |
| WebSocket ping/pong | Auto-pong i **`ws_recv`** / server framing |
| Stream `tee` cancel | **`stream_cancel`** / **`stream_abort`** propagerar till tee-syskon |
| `Deno.permissions` | **`permissions_query`**, **`permissions_request`**, **`permissions_revoke`**, **`permissions_grant`**, **`Deno_permissions`** |
| `Deno.test` / `Deno.bench` | **`deno_test`**, **`deno_bench`**, **`deno_test_report`**, **`deno_bench_report`**, **`Deno_test`**, **`Deno_bench`** |
| Lockfile | **`lockfile_read`**, **`lockfile_sync`** → `kabootar.lock` |
| `Deno.realPath` / `symlink` / `link` | **`realpath`**, **`symlink`**, **`link`**, **`Deno_realPath`**, **`Deno_symlink`**, **`Deno_link`** |
| `Deno.listenTls` | **`tls_listen`**, **`tls_reload_certs`**, **`tls_accept`**, **`tls_server_read/write/close`**, **`Deno_listenTls`** |
| SharedWorker | **`shared_worker_connect`**, **`shared_worker_post_message`**, **`shared_worker_recv`** (in-process namn→worker) |

### ❌ Medvetet borttaget (problematiska JS-delar)

| JS-beteende | Varför bort i Kabootar |
|-------------|------------------------|
| Implicit typkonvertering (`"1"+2`) | Runtime-fel — förutsägbar typ |
| `==` coercion (`null == undefined`) | `==` är redan strikt per typ |
| `var` och hoisting | Endast block-scoped `let`/`const` |
| Prototyp-arv | Explicita `class` + `this` (C#-stil) |
| `eval()` | Säkerhet och förutsägbarhet |
| `with` | Ogenomskinlig scope |
| Tyst `NaN` i heltalsdivision | Fel eller `NaN` explicit |

---

## Lånade konstruktioner (ska vara kompletta)

### ✅ Finns

| Ursprung | Konstruktion | Status |
|----------|--------------|--------|
| Rust | `match` / mönstermatchning | ✅ tal, variabel, `_`, `Some`/`None`, `Ok`/`Err` |
| Rust | `Option` / `Result` | ✅ literaler + match + **try/catch** |
| Rust | `pub` export | ✅ `pub fn`, `pub let`, `pub const` |
| C# | `class` + fält + metoder | ✅ parsing, `this`, instansiering, **`fn init(...)`**, **`extends`**, **`super`**; **`self`** reserverat för `struct` (Våg R) |
| Rust | `struct` + `&self` / `&mut self` | ✅ host; self-host parse+compile-run (R4) + **`struct Box<T>`**; **kab-only** `Box(42)` / `Box("x")` / `Box$Number` / `Box$String` / `WBox<Shown>` / `WBox$Shown` / `WBox<Nope>` reject |
| C# | `interface` + `implements` | ✅ `interface I { fn m(); }`, `class C implements I`, `is_impl()`; **self-host:** default-metod inject + `type Item;` / **`type Item = Number`** + **`where T: Trait`** + **`trait Show<T>`** / `implements Show<Number>`; **kab-only** `Show$Number` / `type Item = Number` / `where T: Show` / `Box$Shown` / `show_it$Shown` / `Box().show_it<Shown>` / `show_it<Nope>` reject / `Box().show_it<Nope>` reject / `Box<Nope>` reject / `where T: Show, T: Named` / `both_it$Shown` / `both_it<OnlyShow>` reject / `where A: Show, B: Named` / `pair_it$Shown_Labeled` / `pair_it<Shown, Nope>` reject / `PairBox<Shown, Labeled>` / `PairBox$Shown_Labeled` / `PairBox<Shown, Nope>` reject / `Box().join_ab<Shown, Labeled>` / `join_ab$Shown_Labeled` / `Box().join_ab<Shown, Nope>` reject / `Box().both_it<Shown>` / `Box().both_it<OnlyShow>` reject / `BothBox<Shown>` / `BothBox$Shown` / `BothBox<OnlyShow>` reject / `WBox<Shown>` / `WBox$Shown` / `WBox<Nope>` reject / `Thing().id()` / `iface_method_default` / `id() { return 42 }` / `Show<T> default` / `Show<T> default override` |
| Rust | `try`/`catch` på `Result` | ✅ fångar `Err`, unwrapar `Ok` |
| C# | Moduler per fil | ✅ `import "mod"`, `lib/*.kab` |
| Kabootar | `@version` / semver | ✅ `import "mod@1.0"` |

### ✅ Finns (komplett för nuvarande scope)

| Ursprung | Konstruktion | Status |
|----------|--------------|--------|
| Rust | `match` / mönstermatchning | ✅ host: tal, variabel, `_`, `Some`/`None`, `Ok`/`Err`, array/objekt, **`...rest`**, guards, enum. **Self-host:** samma + **`n @ 1..=5 if n != 3`** + or/range/`if let`/`while let` + text-`.kbc` enum-register + Kab-VM payload-ctors + **`match Option.Some(n)`** + **kab-only** **`match 1 { 1 => 2, _ => 0 }`** / **`match [x, y]`** / **`match { p, q }`** / **`match 1..=5`** / **`n @ 1..=5`** / **`1 | 2 | 3`** / **`..5`** / **`5..`** / **`[h, ...t]`** / **`{ k, ...s }`** / **`[h, ...mid, last]`** / **`n @ 1..=5 if n != 3`** / **`Color.Red`** / **`Msg.Move(p)`** / **`xs @ [p, q]`** / **`wrap @ { k, ...s }`** / **`{ k: n @ 1..=5 }`** / **`[n @ 1, ...r]`** / **`Ok(n @ 1..=5)`** / **`Some(n @ 1..=5)`** / **`(1 | 2)`** / **`Option.Some(n)`** / **`Option.Some("x")`** / **`Option<Number>.None`** / **`1.0..=2.0`** / **`Result.Ok(n)`** / **`Result<Number, String>.Err`** / **`n @ 1 | 2`** / **`v @ Msg.Move(x)`** |
| Rust | `enum` (användardefinierad) | ✅ host + self-host unit/payload; **kab-only** `match Color.Red` / `Msg.Move(p)` / `v @ Msg.Move(x)` / `Option.Some(n)` / `Option.Some("x")` / `Option<Number>.None` / `Result.Ok(n)` / `Result<Number, String>.Err` |
| Rust | `if let` / `while let` | ✅ host; self-host socker över `match`; **kab-only** `if let Some(x) = Some(3)` / `while let Ok(v) = r` / `if let 1 | 2` / `while let 1 | 2` / `if let n @ Some(x)` / **`if let 1.. = x`** / **`while let 1.. = r`** / **`if let ..5 = x`** / **`while let ..5 = r`** |
| Rust | fälttyper i klasser | ✅ `x: number` / `status: Color` runtime-check |
| Rust | `Option` / `Result` | ✅ literaler + match + **try/catch** + **`?`-operator** (host + self-host + Kab-VM: unwrap `Ok`, behåll `Err`) |
| Rust | `pub` export | ✅ `pub fn`, `pub let`, `pub const` |
| C# | `class` + fält + metoder | ✅ parsing, `this`, instansiering, **`fn init(...)`**, **`extends`**, **`super`**; **`self`** reserverat för `struct` (Våg R) |
| Rust | `struct` + `&self` / `&mut self` | ✅ host; self-host parse+compile-run (R4) + **`struct Box<T>`**; **kab-only** `Box(42)` / `Box("x")` / `Box$Number` / `Box$String` / `WBox<Shown>` / `WBox$Shown` / `WBox<Nope>` reject |
| C# | `interface` + `implements` | ✅ `interface I { fn m(); }`, `class C implements I`, `is_impl()`; **self-host:** default-metod inject + `type Item;` / **`type Item = Number`** + **`where T: Trait`** + **`trait Show<T>`** / `implements Show<Number>`; **kab-only** `Show$Number` / `type Item = Number` / `where T: Show` / `Box$Shown` / `show_it$Shown` / `Box().show_it<Shown>` / `show_it<Nope>` reject / `Box().show_it<Nope>` reject / `Box<Nope>` reject / `where T: Show, T: Named` / `both_it$Shown` / `both_it<OnlyShow>` reject / `where A: Show, B: Named` / `pair_it$Shown_Labeled` / `pair_it<Shown, Nope>` reject / `PairBox<Shown, Labeled>` / `PairBox$Shown_Labeled` / `PairBox<Shown, Nope>` reject / `Box().join_ab<Shown, Labeled>` / `join_ab$Shown_Labeled` / `Box().join_ab<Shown, Nope>` reject / `Box().both_it<Shown>` / `Box().both_it<OnlyShow>` reject / `BothBox<Shown>` / `BothBox$Shown` / `BothBox<OnlyShow>` reject / `WBox<Shown>` / `WBox$Shown` / `WBox<Nope>` reject / `Thing().id()` / `iface_method_default` / `id() { return 42 }` / `Show<T> default` / `Show<T> default override` |
| Rust | `try`/`catch` på `Result` | ✅ fångar `Err`, unwrapar `Ok` |
| C# | Moduler per fil | ✅ `import "mod"`, `lib/*.kab`, lokalt paketregistry |
| Kabootar | `@version` / semver | ✅ `import "mod@1.0"` |

---

## Runtime (utöver JS)

Kabootar är **fullstack** — detta finns inte i vanlig JavaScript:

- `import "sql"` / `sql()` / **`sql_async()`** — in-process databas
- `http_route`, `http_serve`, **`http_request_async`** (in-process), **`http_fetch_async`** (headers, redirects, timeout), **`http_headers`**, **`http_header`**, **`http_set_timeout`**, **`tls_*`**
- `import "os"` — sandboxat filsystem, **`os_read_async`**, **`os_write_async`**, CFS/`os_mm_*` (fault/mmap/COW), journal/`os_acl_*`
- `import "kbrowser"` / `kbrowser/core` / `kbrowser/mobile_chrome` — cross-platform + mobil shell
- `import "science"` — fysik, kemi, statistik, matriser
- `import "crypto"` — kryptografi
- `import "std"` — JSON, Map/Set, array/string/regex, typkontroller
- KML / Kabootar DOM

---

## Versionshistorik

| Version | Språk |
|---------|-------|
| v1.9 | Array-literaler, filimport, bättre fel |
| **v2.2** | `const`, objekt, index, for-in, template, map/filter, `//`, `%`, `!`, `? :` |
| **v2.3** | destructuring, spread, klassisk `for`, `try`/`catch` på `Result` |
| **v2.4** | pilfunktioner, `async`/`await`, klass-arv (`extends`), `fn init(...)` |
| **v2.5** | `super.init(...)`, `super.method()` — anropar förälderns metoder |
| **v2.6** | microtask-kö, delade Promise, drain vid `await` och slut av script |
| **v2.7** | `sleep_ticks`, `interface`/`implements`, `is_impl()` |
| **v2.8** | `os_read_async`, `http_request_async`, `sql_async`, `await_all` |
| **v2.9** | `http_fetch_async` — riktig HTTP/TCP mot externa `http://`-URL:er |
| **v2.10** | `https://` i `http_fetch_async` — TLS (rustls, system roots) |
| **v2.11** | `tls_add_ca`, `tls_ca_only`, `tls_pin`, `tls_reset`, `tls_cert_sha256` — custom CA och cert pinning |
| **v2.12** | `http_fetch_async` med headers-objekt, `http_headers(res)` — request/response headers |
| **v2.13** | String-nycklar i objekt, `http_header(res, name)`, HTTP redirects |
| **v2.14** | `?` på `Result`, `match`-guards, `instanceof(obj, "Class")` |
| **v2.15** | array-/objekt-mönster i `match` |
| **v2.16** | `http_set_timeout`, `http_reset_timeout`, per-request timeout i `http_fetch_async` |
| **v2.17** | lokalt paketregistry — `kabootar publish/install`, `registry_*`, `.kabootar/packages/` |
| **v2.18** | bytecode-VM, `.kbc`-cache, `bytecode_can_compile`, AST-fallback |
| **v2.19** | bytecode: array, `arr[i]`, `.length`, `while`, tilldelning till namn |
| **v2.20** | bytecode: objekt, index-/medlemsskrivning, `for-in`, klassisk `for` |
| **v2.21** | bytecode: `const`, metodanrop, template literals, `BytecodeFn` i högre ordning |
| **v2.22** | bytecode: spread, destructuring, `try`/`catch`, `break`/`continue` |
| **v2.23** | bytecode: sync-pilfunktioner, `match` (subset + guards) |
| **v2.24** | bytecode: array-/objekt-mönster i `match` (v2.15) |
| **v2.25** | bytecode: `async fn`, async-pilar, `await` |
| **v2.26** | bytecode: klasser, `init`, metoder, `extends` |
| **v2.50** | Python-våg (utan `elif`): `pass`, `raise`, `assert`, `with`, `is`/`is not`, `range`, `counter_*`, `defaultdict_*`, `iterator_chain`/`accumulate`/`pairwise` |
| **v2.58** | Advanced Canvas 2D — paths, gradients, transforms, KDOM compositor (`canvas_*`) |

---

## Bidra

Öppna issue eller PR med etikett `language` om en funktion ska flyttas från 🚧 till ✅. Uppdatera den här filen i samma PR.
