# Kabootar standard library

Kabootar har **tre lager** — samma modell som Rust (`core`/`std`) och JavaScript (globals + npm):

| Lager | Var | Användning |
|-------|-----|------------|
| **Natives** | `src/runtime/stdlib/*.rs` | Snabb runtime — registreras som globala (`map`, `floor`, `object_keys`, …) |
| **Inbyggda moduler** | `import "math"`, `import "strings"`, … | Stub-källa + LSP; se `src/modules/mod.rs` |
| **Fil-moduler** | `lib/**/*.kab` | Projekt-specifik std — `import "std/array"` |

Kör `std_info()` för aktuell capability-lista.

---

## JS-paritet — sammanfattning (2026-07)

### ✅ Finns (native)

| Område | Kabootar | JS-motsvarighet |
|--------|----------|-----------------|
| **Array** | `map`, `filter`, `reduce`, `for_each`, `slice`, `concat`, `flat`, `flat_map`, `includes`, `index_of`, `find`, `find_index`, `find_last`, `sort`, `reverse`, `join`, `push`, `pop`, `shift`, `unshift`, `splice`, `at`, `fill`, `copy_within`, `to_spliced`, `array_from`, `array_of`, `values`, `entries` | `Array.prototype.*` |
| **String** | `trim`, `split`, `replace`, `pad_start`, `char_at`, `str_includes`, `str_slice`, `str_normalize`, `char_code_at`, **`str_match`**, **`str_search`**, **`str_locale_compare`** | `String.prototype.*` |
| **Object** | `object_keys`, `object_values`, `object_entries`, `object_assign`, `object_has_own`, `object_is`, `object_group_by`, `structured_clone`, freeze/seal | `Object.*` |
| **Math** | `floor`, `ceil`, `round`, `abs`, `min`, `max`, `sqrt`, `pow`, `random`, trig + **`sinh`/`cosh`/`tanh`/`asinh`/`acosh`/`atanh`**, `hypot`, `imul`, `clz32`, … | `Math.*` |
| **Loop** | `for x of arr`, `for await x of gen()` | `for…of` / `for await…of` |
| **Iterator** | `Iterator.from`, `.map`, `.filter`, `.take`, `.flatMap`, … | ES2023 Iterator helpers |

### 🚧 Planerat (Våg G — se [ROADMAP.md](ROADMAP.md))

| Gap | Prioritet |
|-----|-----------|
| `Array.prototype.toLocaleString` / `toString` på värden | G2 ✅ |
| `String.prototype.matchAll` (global regex iterator) | G2 ✅ |
| `String.prototype.localeCompare` med `Intl` locales | G3 ✅ subset (`sensitivity`) |
| `Math.f16round`, `Math.sumPrecise` | G4 |
| Method-syntax `arr.push(x)` på icke-variabel receiver (bytecode) | G2 ✅ |
| `import "std"` som enda entry (aggregator-modul) | G3 ✅ (`lib/std.kab` + builtin) |
| **Traits** för generics (se [GENERICS.md#traits](GENERICS.md#traits)) | G5 |

### ❌ Medvetet borttaget

Prototypkedja, implicit coercion, `eval`, `var` — se [JAVASCRIPT.md](JAVASCRIPT.md).

---

## Fil-moduler (`lib/std/`)

```
lib/std/
  array.kab    — sum, first, last, fromIterable
  string.kab   — isEmpty, capitalize, lines, words
  math.kab     — degToRad, lerp, isFiniteNumber
  object.kab   — isEmpty, pick, mapValues
```

```kabootar
import "std/array"
let total = sum([1, 2, 3])
```

### kDOM + KSS (`lib/kdom/`, `lib/kstyle/`)

Kabootar-språkliga wrappers — natives (`kdom_*`, `kstyle_*`) är syscall-lager tills logiken porteras hit.

```
lib/kdom/
  document.kab  — el, text, attach, attr, query, domExtra, paint
  events.kab    — on, listen, dispatch, id, mutations
lib/kstyle/
  parse.kab     — parseSheet, parseAndApply, sheetToCss (Kabootar KSS parser)
  selectors.kab — matches tag/class/id (Kabootar)
  parser.kab    — facade: parse, ruleCount, rules
  theme.kab     — reset/addRule/commit, applyDark(), applyCss()
examples/
  kdom_smoke.kab — cargo run --bin kabootar -- examples/kdom_smoke.kab
  kstyle_parse_smoke.kab — KSS parser smoke
lib/kv8/
  defs.kab      — JS keyword map + token + AST constants
  lexer.kab     — tokenize, tokenType, tokenValue (G9)
  parser.kab    — parseSource, parseTokens (G9)
  eval.kab      — evalSource, evalProgram (G9)
  host.kab      — hostCall (k8host -> kDOM/KSS)
  dom.kab       — makeEnv, evalUi (Kv8 + kDOM; use sequential JS calls)
examples/
  kv8_lexer_smoke.kab
  kv8_parser_smoke.kab — cargo run --bin kabootar -- examples/kv8_parser_smoke.kab
  kv8_eval_smoke.kab — cargo run --bin kabootar -- examples/kv8_eval_smoke.kab
  kv8_dom_smoke.kab — cargo run --bin kabootar -- examples/kv8_dom_smoke.kab
```

```kabootar
import "kdom/document"
import "kstyle/theme"
applyDark()
let root = attach(el("div"), text("Hello"), true)
paint(root, 1280, 720, "")
```

Tester: `cargo test --test kdom_lib`

### kOS (`lib/os/`)

Kabootar-språkliga wrappers over sandboxed `os_*` natives.

```
lib/os/
  vfs.kab     — read, write, exists, list, mkdir, remove, stat
  mount.kab   — mount, unmount, mounts
  process.kab — spawn, list
  kernel.kab  — info, caps
  async.kab   — readAsync, writeAsync, readPromise, writePromise, awaitAll
  mem.kab          — @manual MemBox: alloc/read/write/free (owned_*)
  display_buf.kab  — @manual framebuffer helper over os/mem
examples/
  os_smoke.kab — cargo run --bin kabootar -- examples/os_smoke.kab
  os_async_smoke.kab — cargo run --bin kabootar -- examples/os_async_smoke.kab
```

Tester: `cargo test --test os_lib`; ownership: `cargo test --test ownership_manual`

---

## Inbyggda import-moduler

| Modul | Innehåll |
|-------|----------|
| `import "std"` | JSON parse/stringify + `std_info()` |
| `import "math"` | Exempel-wrappers (natives finns globalt) |
| `import "strings"` | `clean`, `parts`, `has_prefix` |
| `import "collections"` | `map_new`, `set_new`, `from_pairs` |
| `import "json"` | `parse`, `dump` |
| `import "science"` | Komplexa tal, matriser, statistik |
| `import "kv8"` | Kv8 JS-motor (DOM/React-bridge) |

---

## kDOM / Kv8 / ramverk (Våg C + G)

| Komponent | Status | Doc |
|-----------|--------|-----|
| **kDOM** | 🚧 querySelector, events; **`lib/kdom/`** Kabootar wrappers ✅ | [RUNTIME.md](RUNTIME.md) |
| **Kv8** | 🚧 **`lib/kv8/lexer.kab`** Kabootar JS lexer (G9 start); eval prestanda | [KV8.md](KV8.md) |
| **kss** (Kabootar Stylesheets) | 🚧 **`lib/kstyle/`** Kabootar wrappers ✅ | [ROADMAP.md](ROADMAP.md) |
| **Next-lik routing** | 🚧 `http_route` + filbaserad pages | Våg G6 |

Mål: **Kv8 skrivet i Kabootar** när self-host bootstrap når lexer/parser/emit full compile (Våg E ✅ subset).

---

## Plattformar

| Mål | Väg |
|-----|-----|
| **kOS** | `lib/os/*`, `kbrowser`, `kabootar://` VFS (referensstack); Windows-lik shell, modern compositor ([OS.md](OS.md#desktop--utseende)) |
| **Windows / Linux / macOS** | Native binary + `kbrowser` desktop shell (G11) |
| **WASM** | `wasm-pack` + `kabootar-shell.html`, `kb_host_sync()` |
| **Android** | WASM WebView/PWA + touch ([G7](ROADMAP.md)); Kabootar Shell-app |
| **iPhone / iOS** | WASM WKWebView/PWA + safe area ([G7](ROADMAP.md)); Shell-app |
| Server | `kabootar serve`, bytecode `.kbc` |

**kbrowser cross-platform (G11 + G7):** samma `kb_*`-API på kOS, desktop-värd-OS **och mobil (Android, iPhone)**. Detaljer: [BROWSER.md#plattformsmål](BROWSER.md#plattformsmål), [ROADMAP.md](ROADMAP.md).

---

## Referenser

- [JAVASCRIPT.md](JAVASCRIPT.md) — porting-guide
- [FEATURES.md](FEATURES.md) — statusmatris
- [VSCODE_TESTS.md](VSCODE_TESTS.md) — hur du kör tester i IDE
- [COMPILE.md](COMPILE.md) — snabb kompilering
