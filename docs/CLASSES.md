# Kabootar — klasser (C#-inspirerat)

## Designmål

Kabootar-klasser ska kännas som **C#**, inte som JavaScript-prototyper:

- Explicita **fält** med valfria defaultvärden
- **Metoder** bundna till instansen via `self`
- **Konstruktor** — `fn init(...)` (konvention)
- **Arv** — `class Dog extends Animal` (v2.4)
- **Interfaces** (planerat)

## Syntax

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
p.sum()
```

### Arv

```kabootar
class Animal {
    name: string;

    fn init(n) {
        self.name = n
    }

    fn label() {
        return self.name
    }
}

class Dog extends Animal {
    breed: string;

    fn init(n, b) {
        self.name = n
        self.breed = b
    }

    fn label() {
        return self.name + " (" + self.breed + ")"
    }
}

let d = Dog("Rex", "lab")
d.label()
```

Barnklasser **ärver fält och metoder** från föräldern. Barnets `init` och metoder **överskriver** förälderns med samma namn.

### super

```kabootar
class Dog extends Animal {
    breed: string;

    fn init(n, b) {
        super.init(n)
        self.breed = b
    }

    fn greet() {
        return super.greet() + "!"
    }
}
```

`super.method()` anropar förälderns version — även om barnet har en egen metod med samma namn. `super` fungerar bara inuti metoder (där `self` finns).

### interface / implements (v2.7)

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

is_impl(p, "Greeter")   // runtime-check (inte keyword — `implements` är reserverat)
```

Ärvda metoder räknas mot interface-kravet.

## Implementation

Modulen `src/class/` innehåller:

- `ClassDef` — statisk klassdefinition (`extends: Option<String>`)
- `ClassInstance` — runtime-instans med fält och metodtabell
- `ClassRegistry` — globalt register

Evaluering (`src/evaluator.rs`):

- `materialize_class()` slår ihop förälder + barn (fält, metoder)
- `instantiate_class()` skapar instans och anropar `fn init(...)` med konstruktorargument
- `self.x = v` i metoder muterar instansen via `assign_member_value`

Fälttyper (`string`, `number`) är annoteringar för nu — runtime validerar dem inte ännu.

## Skillnad mot JavaScript

| | JavaScript | Kabootar |
|---|------------|----------|
| Modell | Prototyper | Klassdefinitioner |
| `this` (JS) | Implicit, bunden dynamiskt | **`self`** — explicit i metoder (Rust/Swift-stil) |
| Fält | Vilken property som helst | Deklarerade fält |
| Konstruktor | `constructor` | `fn init(...)` |
| Arv | Prototypkedja | `extends` — fält/metoder kopieras vid materialisering |

Se [ROADMAP.md](ROADMAP.md) och [JAVASCRIPT.md](JAVASCRIPT.md) för JS-jämförelse.

## Generiska klasser (G7–G8) ✅

Fn-generics v1 och klass-generics (Rust) enligt [GENERICS.md](GENERICS.md#fas-2--g6-planering) och roadmap **Våg F**:

| Milestone | Innehåll |
|-----------|----------|
| **G7** ✅ | Generiska **metoder** — `fn echo<T>(x) { … }` |
| **G8** ✅ | Generiska **klasser** — `class Box<T> { … }`, monomorphisering som fn (`Box$Number`) |

**Struct** planeras inte. Trait bounds och generiska interfaces ingår inte i Våg F.
