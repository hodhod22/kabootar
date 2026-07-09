# Kabootar — språket

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

## Kontrollflöde

- `if` / `else`
- `while` / `break` / `continue`
- `for … in` / `for … of`
- `match` (Rust-liknande)
- Python-lån: `pass`, `raise` (alias `throw`), `assert`, `with x as name { }`, `is` / `is not`, `not x`

## Funktioner

```kabootar
fn add(a, b) {
    return a + b
}
```

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
| 2 | Comptime 3.0 | 🔶 | `comptime { }`, `comptime_assert` |
| 3 | Aktörer | 🔶 | `actor Name { }`, `actor_spawn` |
| 4 | Hot reload | 🔶 | `kabootar serve --watch` + kbc-invalidate |
| 5 | Auto-SIMD | 🔶 | `@simd` (dokumentation) |
| 6 | Valfri minne | 🔶 | `os_mem_*`, `@gc`/`@manual` |
| 7 | Web-native | 🔶 | `html! { }` → Kv8 |
| 8 | Verktygskedja | 🔶 | `compile`, `fmt`, `registry_*` |
| 9 | Statisk binär | 🔶 | `cargo build --release` |
| 10 | Match + guards | ✅ | `match x if cond =>` |
| 10b | User `enum` | ✅ | `enum Color { Red, Green }`, `Color.Red`, `match` |
| 10c | `if let` / `while let` | ✅ | `if let Some(x) = opt { }`, `while let Ok(v) = r { }` |
| 10d | Class field types | ✅ | `age: number` valideras vid tilldelning |
| 11 | Effect system | 🔶 | `@pure` `@io` `@disk` (strippas) |
| 12 | Benchmark | 🔶 | `lang_benchmark`, `@benchmark` |
| 13 | Doc-exempel | 🔶 | `@example` planerat |
| 14 | Kanaler | ✅ | `channel_new/send/recv` |
| 15 | Cache-layout | 🔶 | `@packed` (dokumentation) |
| 16 | Post-quantum | 🔶 | `crypto_kyber_encapsulate`, `crypto_dilithium_sign` |
| 17 | Persistens | 🔶 | `@persist`, `persist_save/load` |
| 18 | GPU/shader | 🔶 | `shader_compile`, `webgl_*` |
| 19 | Resumable fel | 🔶 | `try/catch` returnerar resume-värde |
| 20 | Självhostande | 🔶 | Kompilator i Rust idag |

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
