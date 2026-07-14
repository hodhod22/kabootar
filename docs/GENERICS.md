# Kabootar — generics

**Status:** ✅ **v1 fn-generics klart** — Rust (parse, monomorphisering, bytecode, `tests/generics.rs`) och self-host (`lexer.kab` → `parser.kab` → `emit.kab`, `test_parser.kab` / `test_emit.kab`).

**Förutsättningar (self-host):**

1. ~~Milestone 10 — emit~~ ✅
2. ~~Milestone 11 — serialize~~ ✅
3. ~~Milestone 12 — true bootstrap~~ ✅
4. ~~G4 — self-host parser + emit~~ ✅

---

## Mål

- Parametriserade **funktioner** med compile-time typargument: `fn id<T>(x: T) -> T`
- **Monomorphisering** vid compile — varje konkret typ får en egen bytecode-funktion (samma modell som `.kbc` + `functions[]`)
- Tydlig gräns mot **TS type-erasure** (`ts_strip_types`) — separata spår, ingen blandning i v1
- `self` för **klasser** oförändrat (C#-stil); **struct** är **inte** planerat

## Icke-mål (v1)

| Funktion | Status |
|----------|--------|
| Generiska **klasser** | G8 ✅ |
| Generiska **klassmetoder** | G7 ✅ |
| Trait bounds (`T: Display`) | Nej |
| Higher-kinded types | Nej |
| Varianter / `enum Option<T>` | G9 ✅ |
| Runtime typreflektion (`typeid`) | Nej i v1 |
| Inferens över modulgränser | Begränsad — se nedan |

---

## Relation till TypeScript-transpilering

Idag finns **endast** generics i **`ts_compile` / `ts_strip_types`** — typparametrar **raderas** innan Kabootar-källan körs:

```typescript
function id<T>(x: T): T { return x }
// → function id(x) { return x }
```

**Native Kabootar-generics** ska:

- Parsas av `lexer.rs` / `parser.rs` (inte bara TS-strip)
- Lagras i AST med typparametrar
- Monomorphiseras i `bytecode/compiler.rs`
- Serialiseras i `.kbc` som separata specialiserade funktioner

**Regel:** ny `.kab`-kod med `<T>` ska **inte** förlita sig på `ts_transpile` — det är ett annat verktyg.

---

## Syntax (v1)

### Typparametrar på funktioner

```kabootar
fn id<T>(x: T) -> T {
    return x
}

let n = id(42)        // specialisering: id_Number
let s = id("hi")      // specialisering: id_String
```

### Flera parametrar

```kabootar
fn pair<A, B>(a: A, b: B) -> Array {
    return [a, b]
}
```

### Explicit specialisering (valfritt v1, rekommenderat v1.1)

```kabootar
let x = id<Number>(42)
```

Om utelämnat: **inferens från argument** vid anrop (minst för enkel fallback).

### Begränsningar v1

- Typparametrar: **identifierare** (`T`, `K`, `V`) — inga `T extends Foo`
- Returtyper i signatur: **annotering tillåten**, runtime validerar dem **inte** i v1 (samma som class-fält idag)
- Generics i **method** på klass: **fase 2** — börja med top-level `fn` och `pub fn`

---

## Semantik: monomorphisering

Varje anrop `id(expr)` där typen av `expr` är känd vid compile-time skapar (eller återanvänder) en specialisering:

| Källa | Specialiserat namn (internt) |
|-------|------------------------------|
| `id(42)` | `id$Number` |
| `id("x")` | `id$String` |
| `pair(1, "a")` | `pair$Number_String` |

**Bytecode:** varje specialisering är en vanlig post i `module.functions[]` — ingen runtime generic-dispatch i v1.

**Stack / värden:** inget ändras i `Value` — generics är **compile-time only** (som Rust monomorphization, inte Java type erasure med boxing).

### När typen inte är känd

```kabootar
fn callId(f, x) {
    return f(x)   // dynamiskt anrop — ingen generic inferens här
}
```

v1: **ingen** inferens genom högre ordnings funktioner. Kräv explicit specialisering eller konkret callee.

---

## Implementation — Rust (första fasen)

Ordning inom Rust-motorn:

| Steg | Modul | Arbete |
|------|-------|--------|
| 1 | `lexer.rs` | Token `<`, `>`, ev. `,` i type-listor; keyword behövs inte |
| 2 | `ast.rs` | `TypeParam { name }`, `GenericFn { type_params, … }` |
| 3 | `parser.rs` | `parseTypeParams()` efter fn-namn; `parseType()` för enkel ident |
| 4 | `bytecode/compiler.rs` | Specialiserings-tabell; vid `Call` till generic fn → mangla + emit/load rätt `FunctionId` |
| 5 | `bytecode/types.rs` | `.kbc`: optional `generic_instances` eller mangle i `sym` |
| 6 | `tests/` | `tests/generics.rs` — parse, compile, run utan self-host |

### AST (förslag)

```text
FnDecl {
    name: "id",
    type_params: ["T"],
    params: [( "x", Type::Param("T") )],
    return_type: Some(Type::Param("T")),
    body: ...
}
```

### Name mangling (internt)

Mönster: `{name}${T1}_{T2}_…` med runtime-typnamn:

- `Number`, `String`, `Bool`, `Null`, `Array`, `Object`, `ClassName`, …

**Stabil** över serialize/deserialize — `serialize.kab` ska inte behöva förstå generics, bara färdiga `functions[]`.

### Felmeddelanden

| Situation | Meddelande |
|-----------|------------|
| Okänd typparameter | `Unknown type parameter T` |
| Fel antal typargument | `id expects 1 type argument, got 2` |
| Ambiguous inferens | `Cannot infer type for id; use id<Type>(...)` |

---

## Implementation — self-host (andra fasen)

**Efter** Rust-tester gröna **och** M11+M12 klara.

| Fil | Ändring |
|-----|---------|
| `lexer_defs.kab` | Inga nya keywords; `<` `>` finns ofta redan som operatorer — **type context** i parser avgör |
| `ast_defs.kab` | `AST_TYPE_PARAM`, utöka `AST_FN` med `typeParams` |
| `parser.kab` | Efter fn-namn: om `<` → parse type param lista |
| `emit.kab` | Vid generic fn: emit **template** i `functions[]` med metadata ELLER specialisera redan vid emit (enklare: delegera till samma mangling som Rust) |
| `serialize.kab` | Oförändrat om manglade namn redan i IR |

**Self-host-regel (README):** lägg till regel #44+ när implementation påbörjas — t.ex. *spara type params före rekursiv parse som clobbrar peek-state* (samma klass av bugg som `eBxRhs` i emit).

### Self-host test

```kabootar
// tests via Rust CI, inte 3h compile i varje PR:
// self_host_generics_fn_compile_and_run — ignored, ~snabb smoke + compile snippet
```

---

## Exempelprogram (mål för v1-test)

```kabootar
fn swap<T>(a: T, b: T) -> Array {
    return [b, a]
}

fn main() {
    let p = swap(1, 2)      // [2, 1]
    let q = swap("a", "b")  // ["b", "a"]
    return len(p) + len(q)
}
```

Förväntat: två specialiseringar, `main` returnerar `4`.

---

## Klasser och `self`

- **Klasser** behåller `self` som instansreferens ([CLASSES.md](CLASSES.md))
- Generiska **klassmetoder** (`fn map<U>(self, …)`) — **G7** ([GENERICS.md](GENERICS.md#fas-2--g6-planering))
- `interface` / `implements` — oförändrat; ingen generic interface i v1

---

## LSP och IDE (senare)

| Feature | Prioritet |
|---------|-----------|
| Syntax highlight `<T>` | Med parser |
| Hover: specialiserad signatur | Efter inferens |
| Go to definition på `T` | v1.1 |
| Generic completion | v1.2 |

---

## Milstolpar (sammanfattning)

```
M10–M12  self-host bootstrap                    ✅
G1       docs/GENERICS.md                        ✅
G2       Rust: lexer + parser + AST              ✅
G3       Rust: monomorphize + bytecode + tester  ✅ (tests/generics.rs)
G4       Self-host: parser + emit subset         ✅
G5       FEATURES.md + ROADMAP uppdateras        ✅
G6       Inferens v1.1 (variabler, klasser)      ✅
G7       Generiska klassmetoder                  ✅
G8       Generiska klasser                       ✅
G9       Generiska enum / Option<T>              ✅
G10      Self-host G6–G9                         ✅ (parser + emit subset)
G11      LSP / IDE (hover, completion)           ✅
```

---

## Fas 2 — G6+ (planering)

**Förutsättning:** G1–G5 klara (fn-generics Rust + self-host).

**Princip:** samma modell som v1 — **monomorphisering**, inga trait bounds, ingen runtime typreflektion. **Struct planeras inte.**

### G6 — Inferens v1.1 (Rust)

| Steg | Modul | Arbete |
|------|-------|--------|
| 1 | `generics.rs` | Inferera från **lokal/global binding** när typen är känd vid compile (`let n = 42; id(n)`) |
| 2 | `generics.rs` | Stöd fler typnamn i mangling: `Array`, klassnamn från `class_names` |
| 3 | `bytecode/compiler.rs` | Tydligare fel när inferens misslyckas |
| 4 | `tests/generics.rs` | `id(n)` efter `let n = 42`, `pair(arr, s)` med blandade typer |

**Icke-mål G6:** inferens genom HOF / okänd callee (oförändrat från v1).

### G7 — Generiska klassmetoder (Rust)

Syntax (förslag):

```kabootar
class List {
    items: Array;

    fn map<U>(self, f) -> Array {
        // monomorphisera metod per U vid anrop
    }
}
```

| Steg | Modul | Arbete |
|------|-------|--------|
| 1 | `ast.rs` | `type_params` på `ClassMethod` |
| 2 | `parser.rs` | `<T>` efter metodnamn (samma lookahead som fn) |
| 3 | `bytecode/compiler.rs` | Monomorphisera vid `Member`+`Call`; mangla `List$map$Number` |
| 4 | `tests/generics.rs` | Minst ett klassmetod-exempel |

**Begränsning:** inga generiska **fält** på klass i G7 — bara metoder.

### G8 — Generiska klasser (Rust) ✅

Syntax:

```kabootar
class Box<T> {
    value: T;

    fn init(v: T) {
        self.value = v
    }

    fn get(self) -> T {
        return self.value
    }
}

let b = Box(42)           // Box$Number
let s = Box<String>("hi") // Box$String (explicit eller infer från arg)
```

| Steg | Modul | Arbete |
|------|-------|--------|
| 1 | `ast.rs` | `type_params` på klass-deklaration |
| 2 | `parser.rs` | `class Box<T>` efter klassnamn |
| 3 | `bytecode/compiler.rs` | Specialisera `BytecodeClassDef` per typargs; `new Box(42)` → rätt class idx |
| 4 | `bytecode/types.rs` | `.kbc`: manglade klassnamn i `classes[]` (samma modell som fn) |
| 5 | `tests/generics.rs` | `Box<Number>` + `Box<String>` i samma modul |

**Beslut att låsa innan G8:**

1. **Konstruktor:** `Box(42)` infer vs `Box<Number>(42)` explicit — rekommendation: **båda** (samma som fn)
2. **Arv:** `class Child<T> extends Base<T>` — **G8.1**, inte initial G8
3. **Mangling klass:** `Box$Number` (delar `$`-separator med fn)

### G9 — Generiska enum / `Option<T>` (Rust) ✅

| Steg | Innehåll |
|------|----------|
| 1 | `enum Option<T> { Some(T), None }` parse + AST (`type_params` på enum) |
| 2 | Monomorphisera vid `Option.Some(42)` → `Option$Number` i `module.enums[]` |
| 3 | `Option<Number>.None` via member `typeArgs`; `Some`/`None` som keyword i variant/member-namn |

**Begränsning v1:** `match` med `Option.Some(v)` i bytecode-compiler — enum-variant-mönster i `match` är eval-only tills vidare. Tester använder ctor + bytecode-symbolkontroll.

### G10 — Self-host G6–G9 ✅ (subset)

| Del | Status |
|-----|--------|
| **G6** | `emit.kab`: inferens från `let n = 42` → `id(n)` ⇒ `id$Number` |
| **G7** | `parser.kab`: `fn echo<T>(x)` i klass; `emit.kab`: `genericMethodTemplates` + `h.echo(42)` ⇒ `echo$Number` på klassen |
| **G8** | `parser.kab`: `class Box<T>`; `emit.kab`: `Box(42)` ⇒ `classes[]` med `Box$Number`; **G8.1:** `b.echo(1)` ⇒ `echo$Number` på specialiserad klass |
| **G9** | `parser.kab`: `enum Option<T>`, member `typeArgs`; `emit.kab`: `Option.Some(42)` / `Option<Number>.None` ⇒ `enums[]` |
| **Tester** | `test_parser.kab` + `test_emit.kab` utökade (G7–G9) |

**Begränsningar self-host v1:** ctor/enum monomorph registrerar IR; **`Holder()` / `Box(42)` emitter `new_instance`** (fas 3 ✅).

**Regler:** samma som G4 — ingen extra import från `emit.kab`; inline monomorph för fn/klass/enum/metod.

### Fas 3 ✅

| Feature | Rust | Self-host | LSP |
|---------|------|-----------|-----|
| `match Option.Some(v)` i bytecode | ✅ `JumpUnlessEnumVariant` + `UnpackEnumFields` | — | — |
| `class Child<T> extends Base<T>` | ✅ `extends_type_args` + parent auto-specialize | — | — |
| `NewInstance` för klass-konstruktor | ✅ (fanns) | ✅ `OP_NEW_INSTANCE` + serialize `classes[]` | — |
| Hover på `b.echo` med infererad receiver | — | — | ✅ `hover_member_at` |

**Tester:** `cargo test --test generics` (24), `cargo test --lib language::` (22), `test_emit.kab` + `test_serialize.kab`.

### G11 — LSP / IDE ✅

| Feature | Status |
|---------|--------|
| Syntax highlight `<T>` | ✅ `kabootar.tmLanguage.json` — type params + type args |
| Hover: generic signatur | ✅ `hover_at()` — `fn id<T>(x: T) -> T`, `class Box<T>` |
| Hover: specialiserad signatur | ✅ demangle `id$Number` → specialization note |
| Go to definition på `T` | ✅ `SymbolKind::TypeParam` i parser |
| Generic completion | ✅ concrete types efter `<`; mallar `Box`, `id<T>` |
| LSP trigger `<` | ✅ `kabootar-lsp` completion trigger |

---

## Traits (planerat — Våg G5)

Traits gör generics **användbara** utan JavaScript-prototyper:

```kabootar
trait Show<T> {
    fn show(self) -> String
}

class Box<T> implements Show<T> {
    fn show(self) -> String {
        return "Box"
    }
}

fn print<T>(x: T) where T: Show<T> {
    log(x.show())
}
```

| Beslut | Rekommendation |
|--------|----------------|
| Syntax | `trait Name<T>` + `implements` på klass |
| Bounds | `fn f<T>() where T: Show<T>` efter G11 |
| Monomorph | Trait-metoder specialiseras som klassmetoder |
| Self-host | Efter G10 emit parity |

**Icke-mål v1:** HKT, associated types, dyn dispatch runtime.

---

## Beslut att låsa innan G2

1. **Mangling-separator:** `$` vs `__` (rekommendation: `$` — sällan i Kabootar-identifiers)
2. **Inferens:** ja på enkel `id(expr)` / nej tills explicit `id<T>(expr)` (rekommendation: **ja** för literals och variabler med känd bindingstyp)
3. **Generiska klasser:** skjuts till G6+ (ej struct)

---

## Referenser i repo

| Dokument | Koppling |
|----------|----------|
| [CLASSES.md](CLASSES.md) | Referenstyper, `self`, `fn init` |
| [TYPES.md](TYPES.md) | Runtime-värden (generics ändrar inte `Value`) |
| [FEATURES.md](FEATURES.md) | Statusmatris |
| [self_host/README.md](../self_host/README.md) | Bootstrap-milstolpar |
| `src/runtime/ts_compile.rs` | TS generics strip (ej native) |
