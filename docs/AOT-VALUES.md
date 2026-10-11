# AOT value representation — design record

**Status: implemented (v1 + `aot-tv1` tagged+heap domain).** This is the
`nollAotReady` criterion-2 design decision: how dynamic Kab `Value`s are
represented in emitted native code. `aot-tv1` runs the tagged-word
representation on the real CPU (`sh28_aot_tagged_smoke`,
`sh28_aot_heap_smoke`, `sh28_aot_dyn_smoke`, `sh28_aot_float_smoke`) and
the lowerer reaches it from real `.kab` source via `compileIr`
(`sh28_aot_tv1_src_smoke` — arrays, indexing, `.length`, `if`-jumps).

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
  page), `hlimit` (heap bound — pushes the limit address/count into a
  second global slot), `halloc bump limit` (bound-checked bump: newtop
  over the limit slot traps `ud2` native / `throw`s in the mirror —
  an unchecked overrun would scribble past the data mapping; returns
  tagged ptr `addr|1`), `hload`/`hstore` (fixed byte offset — record
  fields), `hloadx`/`hstorex` (runtime qword index — array elements),
  `hloadb`/`hstoreb` (byte granularity — string payloads at cell+8),
  `hlen`/`htag` (header introspection → tagged results for `eq`-checks
  and tag dispatch).
- **Heap growth**: `exec_bytes_data` splits the size caps — code keeps
  the 4 KiB `MAX_EXEC_BYTES` guard, data payloads get a separate 16 MiB
  `MAX_EXEC_DATA`, so the RW mapping can span many pages and `hlimit`
  can sit past the old one-page ceiling (smoke writes/reads at byte
  4800). The bound check uses `jbe` over `ud2` — an inverted `ja` would
  skip the trap exactly when it should fire (emit-side lesson).
- Cells are `[hdr qword | payload…]` where `hdr = tag<<16 | len`; the
  bump allocator never frees — see GC status below.
- **GC status (both backends collect)**: `lib/kab/aot/aot_gc.kab`
  runs real mark/walk passes over the tagged heap — tag-aware payload
  scanning (float `1` and string `4` payloads are raw bits/bytes and
  never traced; closure `6` skips the raw `fnidx` slot; strings size
  their cell as `1+ceil(len/8)` qwords), worklist reachability, and a
  cell-by-cell bump-region live/dead audit. The collector itself is
  **synthesized in the aot op language** (`aotGcSynthFn` → the hidden
  `__gc_collect` fn, arity 4: top index, heap base `lo`, root end
  `rc`, data base `dbase`): mark bit 23 in the header, forwarding in
  header bits 24+, sliding compaction preserving cell order, then
  root + payload rewriting and header cleanup. The same op array runs
  in the mirror interpreter and is emitted as ~4.7 KiB of x64 when a
  program uses collect — one algorithm, two backends, proven equal.
- **Root protocol**: the data page is `[globals | spill | heap]`.
  `gcollect` (`["gcollect", bump, lo, rb, nspill]`) spills all `nl`
  locals and the top `nspill` operand-stack values into data slots
  `[rb, rb+nl+nspill)` — the collector then scans the unified range
  `[0, rc)` (globals + spilled roots) because the native operand stack
  is unreadable. Contract: `rb + nl + nspill <= lo`. After collect the
  rewritten roots are restored and the bump slot gets the new top
  (index→address converted through r12 on native, plain index in the
  mirror).
- **`hcalloc` collect-on-overflow** (`["hcalloc", bump, limit, lo,
  rb, nspill]`): bound-checks like `halloc`; on overflow it stashes
  the byte count in the scratch slot `rb+nl+nspill` (needs one slot
  more, so `rb+nl+nspill+1 <= lo`), spills roots, calls
  `__gc_collect`, reloads roots, and retries the same bound check —
  `ud2` only when the heap is STILL full (mirror throws "over heap
  limit after gc"). Note `os_native_exec` cannot surface a hardware
  trap through the transmuted call — the same bytes via `exec-image`
  exit SIGILL, which is the honest trap
  (`sh28_aot_gc_smoke`: audit 3 live / 2 dead, mid-program `gcollect`
  slides a record cell and rewrites a spilled stack root + locals +
  globals + A's payload pointer — `native 5` = mirror `5`; `hcalloc`
  reclaims 12 dead qwords mid-alloc — `native 1` = mirror `1`; the
  still-full path throws in the mirror and its emission contains the
  ud2 at the retry site).
- **Float cells** (tag 1): `fmk` bound-checked bump-allocates 2 qwords
  (`["fmk", bump, limit, bits, fval]`) and stores the IEEE-754 f64 bit
  pattern in the payload (`movabs` imm64 — `const64` for full-width
  constants). `fadd`/`fsub`/`fmul`/`fdiv` (`["f<op>", bump, limit]`)
  run real SSE `addsd`/`subsd`/`mulsd`/`divsd` on payloads and
  bound-checked-allocate the result; `feq`/`fne`/`flt`/`fle`/`fgt`/`fge`
  compare via `ucomisd` with unordered (NaN) handling (`jp` + `setcc`).
  `fmk` carries both the native bits and the Kab float because Kab
  cannot reinterpret f64→bits in-language — the mirror uses `fval`,
  native uses `bits`; parity is semantic, not bit-derivation.
- **Runtime numeric dispatch** (lowerer tv1 mode): `add/sub/mul/div`
  emit both-SMI→`t*` fast path, both-Float-cell→`f*` slow path, else
  `trap` (`ud2`). `mod` is SMI-only → non-SMI traps. `neg` does
  `tneg`/`fmk(0)−x`. `eq`/`ne` do raw cmp / distinct→const / `feq`/`fne`
  / pointer-identity for non-Float cells. `lt/le/gt/ge` do raw cmp /
  `f<cmp>` / trap — **no SMI→Float promotion yet** (mixed numeric
  compares trap honestly).
- `trap` = `ud2` (0F 0B) — an honest native fault for out-of-domain
  operands; the mirror `throw`s.
- **Closure cells** (tag 6): `mkclo` (`["mkclo", bump, limit, fi, ncap]`)
  bound-checked bump-allocates `[hdr | fnidx(raw) | cap0…capN]` and pops
  `ncap` tagged captures into the payload; `callv`
  (`["callv", nargs, [fi…]]`) pops the callee cell **off the top**
  (compileIr pushes args first, callee last) into `rcx` as a hidden
  arg0, then args into rdx/r8/r9, rejects a SMI callee via
  `test cl,1`/`ud2`, loads `fnidx` from `cell+8` and dispatches through a
  `cmp r10,imm8`/`call rel32` chain to the appended image fns — `ud2`
  when the index matches no candidate. The lowerer maps each arrow's
  `params…,this,caps…` local layout onto the cell convention (local0 =
  the tagged cell; `this` and captures seeded via `hload`) and rewrites
  dynamic `call`s to `callv`; a `load_global` of a module fn used as
  data becomes a 0-cap `mkclo` onto a synthesized wrapper fn that
  re-invokes the real target with the static `callN`.
- ABI lesson: **never use `rsi`/`rdi` as scratch inside image code** —
  both are callee-saved on win64 and the Rust host caller keeps live
  state there (clobbering `rsi` turned a correct image into a host
  stack-overflow). `r10`/`r11` are the volatile scratch regs.
- Kab-VM lesson II: **arrays copy at every call boundary**, so the
  mirror's `aotFnRunG` returns `{v, gl, hp}` and each nested call site
  adopts the returned state — callee heap/global mutations otherwise
  vanish silently (the cell written by `mk` was invisible to `main`'s
  `callv` until the return record carried it back).
- Emit-side lesson: `[r12+disp]` addressing needs `REX.B` on the SIB
  base AND `REX.R` must stay clear for an `rax` dest — `4D`/`4C` are
  both wrong for `lea rax,[r12+X]`; the correct prefix is `49`. Tag a
  heap pointer **after** writing its cells — `or rdx,1` before the
  stores writes header/payload at `base+1`. Bound checks branch **over**
  the trap on success (`jbe`/`jna`), never `ja` — and hardcoded op-index
  jump targets in hand-written smokes must be re-shifted whenever the
  prologue gains ops.
- Kab-VM lesson: record field access returns a **deep copy** —
  `mainR["ops"]` fetched twice gives two unrelated arrays. Lowerer code
  that mutates fetched arrays (jump-target shifts) must keep ONE fetch.
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
