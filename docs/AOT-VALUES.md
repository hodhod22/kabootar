# AOT value representation — design record

**Status: implemented (v1 + `aot-tv1` tagged+heap domain).** This is the
`nollAotReady` criterion-2 design decision: how dynamic Kab `Value`s are
represented in emitted native code. `aot-tv1` now runs the tagged-word
representation on the real CPU (`sh28_aot_tagged_smoke`,
`sh28_aot_heap_smoke`, `sh28_aot_dyn_smoke`).

## v1 — `aot-i64v1` (shipped in `aot_fn_x64.kab`)

Every stack slot, local, argument and result is a raw **untagged signed
i64 qword**. No heap, no objects, no strings — the domain is "typed i64
functions". Anything outside refuses at emit/lower, never silently.

The host `Value` enum (`src/value.rs`) is much wider: `Number(i64)`,
`Float(f64)`, `BigInt`, `String`, `Bool`, `Symbol`, `Array`, `Object`,
`Option`, `Result`, `Function`, `Promise`, `BytecodeFn`,
`ClassInstance`, … — all currently live outside the native domain.

## v2 candidates

| Repr | Stack slot | Int arith cost | Range | GC |
|---|---|---|---|---|
| untagged i64 (v1) | qword | none | full i64 | n/a |
| **tagged word (SMI)** | qword | tag-check + shift (~3 insn) | i62 | ptr = odd qword |
| NaN-boxing | f64/qword | reinterpreting | f64 / ptr48 | ptr in NaN space |
| boxed ptr (uniform) | qword ptr | alloc+load | full | everything GC'd |

### Decision: **tagged word** (SMI + heap pointer), not NaN-boxing, not uniform boxing

- Kab integers are already `i64` — SMI tagging (`v<<1|0` → 62-bit) keeps
  integer arith fast and honest; floats/boxes go through the heap.
- NaN-boxing optimizes for `f64` payloads; Kab's integer-first semantics
  would pay double-bounce for the common case and cap addresses at 48
  bits for no real win.
- Uniform boxing (every value a pointer) forces an alloc+GC-hit per
  number — wrong direction for a systems language.

### Layout

```
qword bit0 = 0  → small integer, value = qword >> 1     (i62)
qword bit0 = 1  → heap pointer, addr = qword & ~1       (aligned)

heap cell: [tag8 | flags8 | len_or_hdr16 | payload…]
  tags: Float f64, Str (len+bytes), Array (len+slots), Object, Option,
        Result, BigInt, Symbol(u64 id), Function(fn index + env cell),
        BytecodeFn, ClassInstance, BoundMethod, Promise…
  immediates (no heap): null, undefined, true, false → reserved small
  tag patterns in the low bits (e.g. 0b011/0b111 forms) so Bool/null
  never allocate.
```

### Implemented op surface (`aot-tv1`)

- **Tagged scalars**: `tconst` (SMI literal, |v|≤2³⁰−1 — tagged fits
  sext imm32), `tag`/`untag`, `is_smi`/`jsmi`/`jnsmi` dispatch,
  `tadd`/`tsub`/`tmul`/`tdiv`/`tmod`/`tneg`, tagged immediates
  (`tnull`/`ttrue`/`tfalse`).
- **Kab heap**: `hbase` (heap start past the globals slots on the data
  page), `halloc` (bump pointer lives in a global slot; returns tagged
  ptr `addr|1`), `hload`/`hstore` (fixed byte offset — record fields),
  `hloadx`/`hstorex` (runtime qword index — array elements),
  `hloadb`/`hstoreb` (byte granularity — string payloads at cell+8),
  `hlen`/`htag` (header introspection → tagged results for `eq`-checks
  and tag dispatch).
- Cells are `[hdr qword | payload…]` where `hdr = tag<<16 | len`; the
  bump allocator never frees — the Kab GC takes over when it lands.
- Emit-side lesson: `[r12+disp]` addressing needs `REX.B` on the SIB
  base AND `REX.R` must stay clear for an `rax` dest — `4D`/`4C` are
  both wrong for `lea rax,[r12+X]`; the correct prefix is `49`.
- Mirror: `aotFnMirrorH` in `aot_fn_exec.kab` executes the same op
  arrays over a pre-sized heap array — native rax and mirror result are
  asserted equal by every tv1 smoke.

### Consequences for the emitter

- `aot-i64v1` ops keep their meaning as the **SMI fast path**: `add` on
  tagged SMIs is `lea`/`add`+overflow-check into the slow path — emitted
  as inline fast path + out-of-line helper later.
- The op set gains `tag_*`/`untag`/`type_check`/`heap_load`/`heap_store`
  ops; the lowerer chooses the fast or generic path per op.
- The globals page (r12 base, v1) generalizes to tagged slots — same
  addressing, wider payload semantics.
- GC: the Kab-side GC (`lib/kab/gc*`) becomes the native heap manager;
  native frames expose stack maps or the GC scans conservatively.
- `div`/`mod` keep the zero-check trap; tagged operands untag first.
- Foreign values (fn values, closures, natives) get heap cells — the
  current "fn value used as data" lowerer rejection becomes a heap
  allocation once v2 lands.

### Rejected alternatives

- **Untagged i64 only** — can't express dynamic Kab; fine for v1, not a
  platform.
- **NaN-boxing** — f64-centric; Kab is integer-first; saves nothing here.
- **Uniform boxing** — alloc-per-scalar is a non-starter for a
  self-hosting systems language.

## Honesty guardrails

- v1 emitters keep refusing dynamic ops loudly — no silent truncation.
- `runModule` parity legs keep comparing native vs the Kab VM on the
  same compiled module — any repr divergence shows up as a wrong rax.
- The transition adds ops; it does not weaken `aot-i64v1` proofs.
