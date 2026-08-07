# Kabootar — Ownership (Våg O)

**Status:** O1–O4 ✅ (compile-time affine + borrow + leak-lint i `@manual`). R2 ✅ (struct-typer affine i `@manual`). O5 = self-host subset. L5/G10b = runtime MemBox.

## Två minnesmodeller

| Läge | När | Semantik |
|------|-----|----------|
| **GC (default)** | Web, Kv8, UI, de flesta `.kab` | Referenser + GC. Ingen ownership-syntax. |
| **`@manual`** | kOS, buffertar, netstack | Affine **Owned** + borrow. Compile-time check + runtime safety net. |

## Science tensors (buffer ownership ≠ `@manual`)

Science keeps a **hybrid** memory model (not full `@manual` affine on every array):

| Piece | Model | API |
|-------|--------|-----|
| **Tensor / nd buffer** (`__buf`, `NdShared`) | Ownership-steered when unique | `Tensor(...)`, `ensureOwned`, `take` / `nd_take`, `isOwner` |
| **Views** | Shared Rc (GC handles) | `slice` / `viewOf` — cannot dangle while any handle lives |
| **Lazy graphs, models, `meta`** | GC objects | `science/lazy`, `tensor.meta` |

`take` moves a unique buffer and marks the source `__moved` (use-after-move errors). Shared views must be dropped or `ensureOwned` first (view creation is recorded so `take` rejects even when sibling locals are outside the callee env).

For OS/netstacks use `@manual` + `Owned` (MemBox). Science tensors stay on the GC product surface with explicit buffer ownership.

## Host `Value::Array` / `Object` sharing (P6b)

Default GC values use **`Rc`** for array/object heap so `LoadGlobal` is O(1) clone (self-host emit). Leak-safety in the host VM:

1. **COW mutations** — always `Value::array_make_mut` / `object_make_mut` (`Rc::make_mut`). Shared aliases keep a stable snapshot.
2. **No direct self-cycles** — `reject_direct_container_cycle` blocks storing a container into itself (`Rc::ptr_eq`).
3. **Object parent chains** — existing `Object.setParent` cycle check + WeakRef / frame GC sweep.
4. **Deeper A→B→A graphs** — use WeakRef (or avoid); plain `Rc` cannot collect those alone.

Compile-time AST paths are trees and do not create cycles.

## Typer (O2–O3)

```kabootar
@manual

fn take(b: Owned) { drop(b) }
fn peek(b: &Owned) { owned_read(b, 0, 1) }
fn poke(b: &mut Owned) { owned_write(b, 0, [1]) }
```

- `Owned` — unik buffert (MemBox)
- `&Owned` — shared borrow (flera OK, ingen move)
- `&mut Owned` — exclusive borrow (en i taget)

## Move (O1–O2)

I `@manual` flyttas `Owned` vid:

- `let y = x` / `y = x` (hela värdet)
- call-argument som är en Owned-bindning (om param inte är `&`/`&mut`)
- `owned_move(x)` / `move(x)` / `drop(x)`

Efter move: **compile error** vid användning (O1), runtime `use after move` som backup.

**Peek-API** (flyttar inte): `owned_read`, `owned_write`, och `os/mem` `read`/`write`.

## Borrow (O3)

```kabootar
let b = owned_alloc(8, "t")
peek(&b)          // shared — b lever kvar
poke(&mut b)      // exclusive
take(b)           // move
// owned_read(b, …)  // fel: use after move
```

## Runtime (L5, kvar)

`Value::Owned` / `OwnedBuf` med `take_move` / `take_drop`. Scope-overwrite droppar unique handles. Det är **säkerhetsnät**, inte substitut för O1–O3.

## Faser

| Fas | Mål |
|-----|-----|
| O1 | Affine analysis + use-after-move vid compile |
| O2 | `Owned` i param/retur |
| O3 | `&` / `&mut` expr + typer |
| O4 | Leak-lint — Owned som lämnar scope utan `drop`/`move` ❌ compile |
| O5 | Same checker i `self_host/` |
| R2 | Struct-typer är affine i `@manual` (samma Place som MemBox Owned) |

### O4 — leak-lint

I `@manual`-moduler: om en `Owned`-plats fortfarande är Owned när ett block/fn/program avslutas (utan `drop`/`move`), ger checkern:

`ownership: Owned 'name' dropped out of scope without move/drop (leak-lint)`

Runtime drop oförändrad — detta är compile-time endast.

### R2 — struct affine ownership

I `@manual` är **struct**-typer affine på samma sätt som `Owned` (MemBox):

```kabootar
@manual
struct Point {
    x: number;
    fn init(n) { self.x = n }
}
let a = Point(1)
let b = a      // move
// use a       // use after move
drop(b)        // konsumera — annars leak-lint
```

- `Point(...)` producerar Owned
- `fn take(p: Point)` — Named-typ som matchar en struct räknas som Owned-plats
- Class-metoder har fortfarande otypade params (skip)

Tester: `cargo test --test ownership_check` + `cargo test --test ownership_manual`.

## Done / non-goals (Våg O)

| Landat ✅ | Icke-mål ❌ |
|-----------|-------------|
| O1–O4 affine + borrow + leak-lint i `@manual` | Lifetimes à la Rust |
| O5 self-host ownership subset | Borrow över `async` |
| R2 struct-typer affine i `@manual` | GC-moduler med ownership-syntax |
| Science buffer `take` / `ensureOwned` (hybrid) | HKT / `dyn Trait` |

Default-GC-kod behöver **ingen** ownership-syntax. Se [ROADMAP.md](ROADMAP.md) Våg O.
