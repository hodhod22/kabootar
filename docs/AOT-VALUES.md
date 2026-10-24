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
- **Tagged bitwise (SH31)**: `bit_and`/`bit_or`/`bit_xor`/`shl`/`shr`/
  `ushr`/`bit_not` lower behind an `is_smi` pair-guard to `tband`/`tbor`/
  `tbxor`/`tshl`/`tshr`/`tushr`/`tbnot` — detag (`sar 1`), 32-bit op
  (matches `to_int32` truncation), `movsxd` sign-extend, `shl 1` retag;
  shift counts are masked `&0x1f` and `ushr` mirrors the host's
  `as i32` wrap (`(-1) >>> 0` → -1, not 4294967295). Float operands
  trap honestly — no f→i32 op exists. Mirror uses Kab's own
  `& | ^ ~ << >> >>>` on detagged values.
- **Tagged pow (SH32)**: `pow` lowers behind the same SMI pair-guard
  to `tpow` — detag, then an `imul` loop (`jno`/`ud2` on overflow),
  `exp<0` traps, and the `shl 1` retag is OF-guarded so results past
  ±2⁶² trap. The host folds `powf` to Float on those paths; with no
  `fpow` op the AOT domain traps honestly. Covers the self-host VM's
  own `a ** b` (`vm_ops_arith_a`) — the last opcode self_host sources
  emit.
- **Module linking (SH33)**: `kab/aot/aot_link.aotLinkModules` merges
  several `compileIr` records into one image the tv1 lowerer consumes
  directly. Each module's top-level ops become a `$main::<name>` fn
  invoked in dependency order from a synthesized entry; `pub` names
  alias the exporter's global slot (live-binding semantics — the
  importer reads the cell the exporter's main stored into); own fn
  syms mangle to `mod::sym` so `load_global`+`call` fusion picks the
  right fn and private-name collisions stay isolated; builtin names
  (`println` …) keep their plain spelling for builtin lookup.
  Constants pools and global indices are rebased per module. Verified
  on pub-fn calls, pub-let data, indirect fn values (callv),
  transitive imports and private collisions — native == mirror on
  every case.
- **SH33 capstone — real self-host DAG emitted and executed**: the
  `self_host/vm_ops_arith_a` import tree (10 modules) links to one
  compileIr record, lowers tv1 → 210 fns, 274 globals, 339,870 lowered
  ops → 3,396,431 bytes of x64 → `os_native_exec` returns the merged
  main's `tundef` (11) natively in ~1 s. Emission is **staged and
  resumable** (`examples/sh33/_sh33_sA/sB/sE/sD.kab`): stage A
  validates+measures each fn independently (`_stageA_<i>.json`), stage
  B aggregates `fnOffs` prefix sums (`_stageB.json`), stage E emits per
  fn to `_stageE_<i>.json` (chunked `i0`/`i1` ranges for the 22k-op
  fns), stage D concatenates, writes `_emit_cache.json` and execs —
  a shutdown loses only the in-flight fn. `aotFnEmitOps` carries an
  absolute base `bOff` so rel32 calls/jumps/GC disp32 stay in image
  coordinates when `b` is a per-fn buffer; monolithic emit passes 0.
- **Under-arg call padding**: VM call semantics pad missing args with
  `undefined` — `callK` emission now pads the missing register slots
  with tagged-`undef` movs (`fa` = per-fn arity table threaded through
  `aotFnOpLen`/`aotFnOpOffsets`/`aotFnEmitOps`), which makes the op
  length variable — the first real callee-arity consumer.
- **Data-page sizing**: `os_native_exec` gained a 4th arg — the RW
  mapping size. The payload array only carries initial bytes; the aot
  heap bound (`hlimit` address, baked into the image) must fit inside
  the mapping or the first heap write past it faults. Anonymous maps
  zero-fill past the payload, so an 8 MiB data page is a mapping size,
  not an 8M-element Kab array. `MAX_EXEC_BYTES` rose 1 MiB → 64 MiB
  for real capstone images; `MAX_EXEC_DATA` stays 16 MiB.
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
  a `MAX_EXEC_BYTES` sanity guard (now 64 MiB for real capstone
  images), data payloads get a separate 16 MiB `MAX_EXEC_DATA`, so the
  RW mapping can span many pages and `hlimit` can sit past the old
  one-page ceiling (smoke writes/reads at byte 4800). The bound check
  uses `jbe` over `ud2` — an inverted `ja` would skip the trap exactly
  when it should fire (emit-side lesson).
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
- **Runtime numeric dispatch** (lowerer tv1 mode, `aotLowerNumPair`):
  `add/sub/mul/div/mod` and `lt/le/gt/ge/eq/ne` emit a four-way type
  dispatch — both-SMI → `t*`/`raw` fast path; both-Float-cell → `f*`;
  mixed SMI/Float (either order) → the SMI side is promoted by `fcast`
  (untag `sar 1` → `cvtsi2sd` → fresh Float cell), then `f*`. A
  non-Float heap operand traps (`ud2`). `mod` on Floats is `fmod` —
  truncating remainder `l − trunc(l/r)·r` via `roundsd` imm 0x0B,
  matching Kab `%`. `neg` does `tneg`/`fmk(0)−x`.
- **`fcast`** (`["fcast", bump, limit]`, 57B): pops a tagged SMI,
  `sar rax,1` → `cvtsi2sd xmm0,rax` → bound-checked 2-qword Float cell
  (header 65537, `movq` payload) → tagged pointer pushed.
- **`fmod`** (`["fmod", bump, limit]`, 89B): `divsd` → `roundsd` trunc
  → `mulsd` → `subsd` on the two payloads, allocates the result cell.
- **`tconst64`** (`["tconst64", v]`, 11B): `movabs rax,imm64`+push —
  integer constants outside the signed-i31 `tconst` window. The
  lowerer accepts exact integers within ±2^52 (`tconst` inside
  [-1073741824, 1073741823], `tconst64` beyond); beyond that it
  rejects honestly.
- **Float constants through `aotF64Halves`**: Kab cannot reinterpret
  an f64's bits in-language, so `aotF64Halves(v)` derives the
  IEEE-754 low/high 32-bit halves arithmetically (sign/exponent/
  mantissa split). The lowerer emits `["fmk", bump, limit, 0, v]` and
  the emitter assembles the imm64 from the halves — `op[3]` no longer
  needs to carry a pre-computed (possibly >2^63) bit pattern.
- **Record cells** (tag 10) + **str-key cells** (tag 4) + **slots
  cells** (tag 11): a record is a *shell* `[hdr=10<<16|1 | slotsPtr]`
  pointing at a *slots* cell `[hdr=11<<16|2n | kid0 v0 …]`. The shell
  indirection preserves alias semantics — `member_set` growth allocates
  a new slots cell and re-points the shell, so `let p = o; p.y = 8`
  makes `o.y` visible (VM `vMemberSet` mutates in place). Member keys
  are **content-hashed to small ints (kid < 2²⁰)**; string cells carry
  their kid in header bits 32+ (`hdr = kid<<32 | 4<<16 | nbytes`,
  payload = packed LE bytes — the cell doubles as the tag-4 string
  value). The kid is a content hash (`k*33+byte` mod 2²⁰, +1), not a
  per-module intern counter — a runtime-built key like `"a"+"b"`
  therefore matches the static literal `"ab"`. Static `smk` cells carry
  the hash in the header; runtime-built strings (concat/itoa results)
  carry kid 0, and dynamic-key paths run `aotKidSeq` — `hkid` first, a
  byte-loop rehash on kid 0 — so lookups stay exact tagged-SMI
  compares. Two new emitter ops only: `smk` (`["smk", bump,
  limit, text, kid]` — bound-checked alloc `1+ceil(len/8)` qwords,
  per-byte stores since packed qwords exceed f64 precision) and `hkid`
  (`["hkid"]`, 16B — `hdr>>32<<1`, the tagged kid). Everything else —
  `make_object`, `get_member`, `member_set`, `merge_object`,
  `object_rest`, and dynamic `index_get`/`index_set` with str keys — is
  **lowered into the existing heap primitives** (`halloc`/`hload`/
  `hstore`/`hloadx`/`hstorex`/`hlen`/`htag`/`mod`+labels), so no new
  native ops were needed beyond `smk`/`hkid`. `merge_object` two-passes
  (count right-only keys → exact alloc → copy left → merge right →
  re-point left's shell); `object_rest` allocates the exclude-kid cell
  and copies kept pairs into a fresh record. Missing members → `tundef`
  (VM missing-member semantics); member_set on non-records traps.
- **String values** (tag 4, the same cell `smk` builds): `add`
  dispatches on string operands *before* the numeric pair — str+str and
  str±SMI all concatenate (the SMI side is coerced by `aotItoaSeq`:
  untag → sign → digit count → bound-checked `halloc` → `-` + digits
  via `hstoreb`, `mod`/`div` by 10); non-number non-string operands
  still trap. `aotConcatSeq` reads both lengths from the headers,
  allocates `1+ceil((la+lb)/8)` qwords, writes a kid-0 tag-4 header and
  copies bytes through `hloadb`/`hstoreb`. `eq`/`ne` on two str cells
  compare **contents** (length first, then byte loop) — not pointers.
  `s[i]` on a str cell bounds-checks the index against the header
  length and returns a fresh 1-char cell (`halloc` 2 qwords) or
  `tundef` out-of-bounds. `.length` reads the header's low 16 bits.
  All of it is lowered out of the existing primitives — no new
  emitter ops.
- **Seq-splice lesson**: lowerer helpers that return op sequences with
  internal `@labels` (`aotIsStrSeq`, `aotItoaSeq`, `aotIsFalsySeq`…)
  collide when spliced twice into one `aotLowerAsm` call — the label
  table keeps the last definition, so every earlier jump targets the
  wrong site and produces silently-wrong results rather than a crash.
  `aotSeqTag(seq, sfx)` rewrites defs+targets with a per-splice suffix;
  every repeated splice inside one op's dispatch must be tagged.
- **`aotF64Halves(0)` fix**: the `1/v` sign test for −0 evaluated
  `1/0` → integer division by zero in the Kab VM. The −0/+0 split is
  unobservable in Kab f64 anyway, so the function now only tests
  `v < 0` and returns `[0, sgn]` for `a == 0` before the exponent loop.
- **Option/Result cells** (tag 12): a "sum" cell `[hdr | v]` where
  `hdr = 12<<16 | variant` — 0 none / 1 some / 2 ok / 3 err; `none`'s
  payload is `tundef`. `make_some`/`make_none`/`make_ok`/`make_err`
  allocate via `halloc`+`hstore`; `jump_unless_option_some/none` and
  `jump_unless_result_ok/err` keep the VM's *peek* semantics by
  store→load-back around an `aotSumIsSeq` variant check (is_smi →
  mod-8 pointer test → `hload 0 == hdr`); `unwrap_*` pops, checks the
  variant and pushes `hload 8`, trapping on the wrong kind like the
  VM's `throw`; `result_question` (`expr?`) unwraps Ok's payload and
  keeps the Err cell on the stack — the `?`-propagation itself stays
  the caller's business; `match_fail` traps honestly.
- **`dup`/`swap` ops**: the match machinery emits `dup` (peek at the
  scrutinee while pattern-testing) and `swap`+`pop` (rebind the
  scrutinee global). `dup` emits `mov rax,[rsp]; push` (5B); `swap`
  emits `pop rax; pop rdx; push rax; push rdx` (4B) — write the push
  order down wrong and it silently becomes an identity (that bug made
  `a[i]=v`-style match writeback return the scrutinee cell instead of
  the bound value; mirror caught it instantly).
- **Match machinery** (`aot-tv1`): every `jump_unless_*` match op is a
  peek — the lowerer stores the scrutinee into a scratch slot, pushes
  a raw flag via the existing tag/kid seq helpers, and emits a `jz/jnz`
  fixup. `jump_unless_object`/`jump_unless_array` are tag checks
  (record 10 / array 5); `jump_unless_object_empty` reads the slots
  cell's `hlen`; `jump_unless_has_member` runs the record kid scan;
  `jump_unless_const_eq` materializes the const via `aotConstTv1Seq`
  and compares with the same `aotLowerEqTv1` used by `eq` (NaN patterns
  take the `feq` self-compare route — unordered → no match);
  `jump_unless_enum_variant` content-compares the record's `variant`
  member and accepts `typeName` equality OR the `Foo$T` generic-prefix
  form (`aotStrPrefDollarSeq`). `unpack_enum_fields` verifies the
  fields member is a tag-5 array of the declared arity and `aget`s the
  payload reversed onto the stack. `match_fail` traps honestly.
- **Enum values are plain records** (`{vmEnum, typeName, variant,
  fields}`) — enum namespaces are *module-init data*, not ops: the tv1
  module lowerer prepends an init sequence (`aotEnumNsSeq`) that builds
  `{vmEnumNs:1, typeName, variants:[{name, fields}]}` and `gstore`s it
  into the enum's global slot before user ops run — the same shape the
  VM session installs. `E.V` lowers as `get_member` whose miss path
  falls back to `aotEnumResolveSeq` (only when `V` is a declared
  variant name — `aotEnumHasVariant` gates it); arity>0 variants
  resolve to `{vmEnumCtor:1,…,arity}` records, and `call` on one builds
  the real enum value in `aotLowerCallvTv1` (checked against `arity`).
- **initOps jump shift**: `aotLowerAsm` resolves `@labels` in the
  seq-local index space — init ops spliced into `mainOps` after the
  4-op heap prologue must have their resolved jump targets shifted by
  the prologue length (`+4`), or control lands inside the seq body.
- **Optional chaining** (`o?.x`, `o?[k]`, `o?.f(a)`) compiles to
  `load_global "__opt_member"/__opt_index/__opt_call"; call n` — host
  natives, so the tv1 lowerer intercepts the fused pair: no `gload`,
  no callee `pop` — it splices a nullish gate (`== tnull || ==
  tundef` → `tundef`) plus the ordinary member/index lookup or the
  dynamic `callv` path.
- **Array/slot ops** (`aot-tv1`): `take_local`/`take_global` move the
  slot value onto the stack and write `tundef` back (the
  `a = push(a, v)` desugar relies on it); `array_push_local`/
  `array_push_global`/`array_push` pop [arr, item] and run
  `aotArrPushSeq` — allocate a fresh tag-5 cell, copy elements with
  `aotArrCopySeq`, append the item, store back, push the tagged `hlen`.
  Always-copy matches the VM's `Rc::make_mut` copy-on-write exactly:
  an unshared push mutates in place in the VM, but no other reference
  exists to observe the difference — `let b = a; a = push(a, 9)` must
  leave `b` at the old length either way. `array_pop_*` shrink to a new
  cell (`max(n-1,0)` — `pop([])` returns `[]` like the host builtin)
  and push `tnull`. `len_local`/`len_global` load the slot then share
  `aotLenBodySeq` with `get_length` (array → element count, str cell →
  byte count, record → field count via slots/2). `concat_array` (spread
  literals `[...a, ...b]` and spread call args) allocates a fresh cell
  — an in-place append would alias-mutate `left`. `array_slice_from`
  clones `arr[arg..]`; `call_from_array` (`f(...xs)`) dispatches arg
  counts 0..3 onto the static `callv` registers and generates
  high-arity `callv` branches (scratch-spilled extras) for 4..15,
  trapping beyond. Const-ness is not modeled:
  the host VM refuses `take_*`/store ops on `immutable_locals`, but
  AOT slots carry no mutability flag (same gap as regular assigns).
- **throw/try-catch** (`aot-tv1`): an in-region `throw` lowers to a
  bare `jmp` to catchStart — the reason stays on the operand stack and
  the region's first op (`store_local errLocal`) binds it, matching
  vCatchThrow's pop+push net-zero. An uncaught `throw` wraps the
  reason in a **tag-13 marker cell** `[hdr=13<<16|1 | reason]` and
  `ret`s it: every callN/callv/call_from_array site appends a marker
  check — SMI/immediate/other-tag → keep the value; tag 13 + region →
  `hload` the reason + `jmp` catchStart; tag 13 + no region → `ret`
  the marker to keep unwinding (wrappers pass it through for free —
  they `ret` the call result verbatim). The uncaught marker survives
  out of main as the program result — observable as a ≡1-mod-8 cell
  pointer (mirror-side: readable tag-13 header + payload in hp).
  Region lookup replicates vFindTry exactly: hit iff ip==catchStart or
  bodyStart<=ip<catchStart, innermost by max bodyStart with the `>=`
  tie-break (so a nested try whose body IS the outer body loses to the
  outer region — the inner catch is dead code, matching the VM), and
  vCatchThrow's ip/ip-1/ip-2 fallback. Jumps targeting `len(ops)`
  (fall-through past a trailing catch) map to a `tundef;ret` landing
  pad appended to every lowered fn — the VM's implicit `return
  undefined`; this also guarantees the emitter's "last op is ret"
  rule. 11 source cases green native=mirror: same-frame, cross-fn and
  multi-frame propagation, nested ties, finally, uncaught marker.
- **Builtin-call trampoline** (`aot-tv1`): `load_global <name>; call n`
  with no module-fn target looks `name` up in `aotBuiltinSeq` and lowers
  it inline — the callee global never materialises, the args on the
  operand stack are consumed and exactly one tagged result pushed
  (same convention as the `__opt_*` intercepts). Covered v1: `len`,
  `push`, `pop`, `slice`, `concat` (variadic stack fold), `typeof`
  (smk literals), `str_slice`/`substring`, `char_at`, `char_code_at`,
  `str_at`, `from_char_code`/`from_code_point`, `parse_int` (trim,
  radix 2..36, 0x/0b/0 prefixes, no-sign digits — matches host), `abs`,
  `min`, `max`, `floor`/`ceil`/`round` (SMI domain; floats trap — the
  emitter has no SSE rounding), the `is_*` family (`is_null`,
  `is_undefined`, `is_bool`, `is_number`, `is_string`, `is_array`,
  `is_object`, `is_function`, `is_nan` — the last via `feq`
  self-compare, NaN≠NaN), `object_has_own`/`has_key` (record kid-scan +
  array bounds), and `object_array_push`/`object_array_pop`
  (read → copy-op → writeback into the record's slots cell). NaN
  results (`char_code_at` OOB, `parse_int` no-digits) are built as
  `fmk 0.0; fmk 0.0; fdiv` — real IEEE NaN native, Kab `0.0/0.0` in the
  mirror. Known divergences: `object_array_push/pop` only update
  `obj[key]` — the host mutates the shared vec in place, so an alias
  captured before the call won't observe the append (cell layout has
  no capacity field); string ops are ASCII-domain (byte idx == char
  idx); `from_char_code` traps for codes > 127. Uncovered names —
  `println`, `os_*`, `map`/`filter`, `join`, `keys` (kid→name
  reconstruction needs the intern table) — fall through to
  `gload`+`callv` and trap honestly on the uninitialised slot. First-
  class builtin values (`let f = len; f(x)`) hit the same trap — no
  builtin materialises into a callable cell yet.
- **Class support** (`aot-tv1`): instances are tag-10 record shells
  `{vmC: classIdx, vmI: uniqueId}` — exactly the Kab VM's `vNewInstance`
  layout — allocated by `aotInstShellSeq` with a bump-counter `vmI`.
  Declared field defaults are NOT materialised (matches the VM: probing
  `class C { x = 5 }` shows `Undefined` until `init` runs). Method
  bodies lower into the image as `Class$method` fns (arity = 1+np,
  local0 = `this`) plus `Class$method$clo` wrappers so a method value
  dispatches through the normal `callv` convention with the bound
  receiver as capture 0. `get_member`'s record-miss path consults
  compile-time class metadata (`mmeth`): compare `vmC` → build a bound
  closure → return it; inherited methods found by walking `extends`.
  `new_instance`/`new_instance_from_array` resolve the initializer via
  `aotClsInitImg` (inherited `init` first, then `constructor`) and emit
  a direct `callN` — only when the arg count equals `len(init.params)`;
  any other count emits an honest `trap` (the emitter validates callN
  arity statically, so unreachable branches can't hide mismatches).
  `get_super_method` resolves the parent method at lower time and
  binds the current `this`. `instanceof` lowers in `aotBuiltinSeq` —
  reads `vmC` and compares the requested class name against the
  extends chain, pushing a tagged bool. 16 source cases green
  native=mirror (construction, init/inherited-init, method calls incl.
  `this` mutation, super, from_array, instanceof on exact/parent/
  unrelated/non-instance). Known divergences: calling a bound method
  with the wrong arg count flows garbage regs into missing params
  (same limitation as arrows — callv carries no argc); `is`/deep-equal
  on instances is identity-only; `implements`/associated types are
  compile-time metadata with no runtime check.
- **High-arity calls** (`aot-tv1`): the private call ABI passes the
  first four ordinary args in `rcx/rdx/r8/r9`; calls with more than
  four spill the extras onto the machine stack. Direct calls emit
  `calln` (`["calln", fi, n, scr]` — n≤20): the caller stashes args
  ≥5 in scratch locals, pops them into the machine-stack arg area in
  order, calls, then `add rsp, 8*(n-4)`. Indirect `callv` (closure
  cell hidden in rcx) supports ≤19 explicit args the same way
  (`add rsp, 8*(n-3)` after the call). Callee prologues seed local
  `i>=4` from `[rbp+24+8*(i-4)]` — `rbp+24` is the first stack arg
  because every fn pushes `r12` before `rbp` (push + mov = 16 bytes
  above rbp; +8 for the return address). Wrappers, methods, bound
  methods, and ctors go through `aotCallOpN` (n≤4 → `callN`, else
  `calln`); `call_from_array` generates per-count `callv` branches;
  `new_instance_from_array` traps honestly when argc ≠ init arity
  and for init arities > 19 (receiver occupies one total slot).
  Mirror: `aotFnRunG` takes an extra `xs` array — caller extras ride
  it and seed callee locals 4+. Two emitter bugs found by the smoke:
  `calln`'s rel32 base subtracted the call size itself (target
  landed 5 bytes inside the callee prologue → privileged
  instruction) and `callv`'s declared length double-counted the
  ≤3-arg reg pops for `na>3` (+ a missing imm32 cleanup form for
  `na=19` → 128-byte spill). `call_from_array`'s generated dispatch
  also only checked counts 0..3, so argc ≥4 fell into the trap —
  it now loops the `jnz @c<k>` chain over 1..19. 16 source cases
  green native=mirror (call5/9/20, arg order across the spill,
  callv 6/8/12/19, spread 6/10, ctor/method/bound/from_array).
- **Emitter nested-index fix** (`self_host/emit_expr_leaf.kab`):
  `emitExpr_index` stored the object node in the shared `E["eBxL"]`
  before recursively compiling the index expression — a nested
  `a[b[i]]` or `a[b.c]` then clobbered it, so `locals[op[1]]`
  emitted `index_get_local` on `op`'s slot (`op[op[1]]`). Fixed by
  capturing the sym before the recursive `eCallExpr`. Regression:
  `tests/nested_index_test.kab`. The full `E[".."]`-field audit
  plugged the same clobber class in `emitExpr_call_h5_emit`
  (`eMemObj`/`eMethodMangled`), `emitExpr_callSpread` (`eCallee`)
  and `emitStmt_expr` (`eExprStmt`); stack-saved fields
  (`eThen`/`eElse`/`eMemberFld`/`eArgs`/`eCallArg*`) and
  local-captured reads (`eNode`/`binRight`/`saveIn`) verified safe,
  and `emitNestedNamedFn` already restores the enclosing fn frame.
- **Decl-in-fn reject** (`self_host/emit_stmt_body.kab`): `class`,
  `enum` and `interface` declarations inside fn/method bodies used
  to silently corrupt the enclosing function — the class emitter
  overwrites `eFnSave`/`eInFn`/`eFnOps*`/`eFnLocals` without save,
  so statements after the decl leaked to top level (`let c` became
  a global; a trailing `return` fired early). Rust's `compile_stmt`
  rejects these via `_ => Err(CompileError)`; the self-host emitter
  now throws `"... declaration only supported at top level"`.
  Regression: `tests/decl_in_fn_reject_test.kab`.
- **Host compiler stack-leak fix** (`src/bytecode/compiler.rs`): an
  `else <expr>` branch (every `else if` level) used to leave its value
  on the operand stack unbalanced — the enclosing `Expr::If` then
  pushed its own null epilogue per nesting level, so a dispatch chain
  of N arms leaked up to N−1 slots per evaluation. `aotLowerOps` (52
  arms) overflowed `MAX_BYTECODE_STACK` at 8192 during big lowering
  runs. Fixed: a non-block else expression *is* the if's result — the
  then-side pushes the fallback null itself, and the shared epilogue is
  skipped (`else_valued` split). `compile_block_or_expr` also pops
  non-block branches when the caller doesn't want a value.
- Kab-VM emit quirk: **`push(st, st[len(st)-1])` inside an `if`-arm
  can miscompile** — the single-expression form produced a `len()`
  failure at runtime while the two-statement form (`let t = st[len-1];
  st = push(st, t)`) is fine. Keep the split form in the mirror.
- **Tagged-immediate guard**: `tnull`/`tundef`/`tfalse`/`ttrue` (3/11/
  19/27, all ≡3 mod 8) are immediates, not pointers — `htag`/`hload` on
  one dereferences address ≤26 → segfault native / garbage mirror.
  Every heap-tag dispatch therefore starts `is_smi` → `v%8==1` ptr
  check → `htag`. This guards numeric dispatch, `neg`, truthiness,
  member/index access, and record ops uniformly.
- **Truthiness**: `jump_if_false`/`jump_if_true` in tv1 lower through an
  `isfalsy` seq — falsy = tagged 0 (SMI 0), 3 (null), 11 (undefined),
  19 (false), or a Float cell with payload == 0.0; everything else
  truthy (raw `jz` would have made `false` truthy — 19 is nonzero).
- **Tagged bools**: user-facing compares (`eq/ne/lt/le/gt/ge`) produce
  `v*8+19` — tfalse=19 / ttrue=27 — never raw 0/1. A raw 1 on the
  operand stack is indistinguishable from a heap pointer under the
  mod-8 check (`1 % 8 == 1`), so `let b = x < y; b == true` would have
  crashed on `htag`; the tagged result makes bools first-class values
  (`b == true` → 27==27, `if b` → falsy-test hits the 19 arm). `not`
  lowers to isfalsy + the same `*8+19` wrap; `and`/`or` keep the VM's
  value semantics — `a && b` pushes `b` when `a` is truthy else `a`,
  `a || b` pushes `a` when truthy else `b` (raw bitwise would corrupt
  tagged bools: `27 | 19 = 31` is not a value).
- **Writeback stays tagged**: `aset` re-ORs the container (`or rdx,1`)
  before pushing it — `a[i] = v` leaves `[ptr, val]` on the stack and
  compileIr's `pop; store_*` writeback must store the *tagged* pointer
  back, not the `-2`-masked base address.
- **`undefined` const**: compileIr stores `undefined` in the const pool
  as the sentinel string `"__kab_const_undefined__"` — the tv1 const
  branch recognizes it and emits `tundef` before the string path; other
  string consts become `smk` cells (so `o["a"]`/`o.k` share interned
  kids), `true`/`false`/`null` become `ttrue`/`tfalse`/`tnull`.
- **Equality**: `feq`/`fne` numeric compare for Float/Float and mixed
  pairs; SMI vs non-Float heap → unequal; other heap cells → pointer
  identity, as documented.
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
- Emit-side lesson II: **`aotFnOpLen` must equal the emitted byte count
  exactly** — it feeds `aotFnOpOffsets`, so a 3-byte undercount sends
  every later `jmp`/`jz` target into the middle of an instruction
  (that's how the first source-level fmod segfaulted). `aotFnEmitOps`
  now audits per-op `len(b)-offs[i]` against `aotFnOpLen` and throws
  `aot-x64 op length mismatch` on divergence — the audit caught the
  `fcast`(57B)/`fmod`(89B) miscounts before any native run.
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
