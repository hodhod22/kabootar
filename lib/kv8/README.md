# Kv8 in Kabootar (`lib/kv8`)

Self-hosted Kv8 JS-subset: lexer → parser → eval → dom. Import chain:

```
kv8/react → kv8/host → kdom/events
kv8/dom  → kv8/eval → kv8/parser → kv8/lexer
                      kv8/host
                      kv8/defs
```

## Module rules (Kabootar bytecode)

1. **Module stacks for recurse** - push `op`/`right`/`lhs` on `evBin*Stack` (and member/call stacks) before nested `evExpr`; fn locals are not re-entrant in `.kbc`. `&&`/`||`/`??` short-circuit (skip RHS).
2. **Bracket access for AST keys** - `node["sym"]`, not `.sym` where names collide.
3. **`evRunBlock` for nested bodies** - if/block/`k8fn` bodies. Pub `evalSource*` keep an inline program loop (delegating the whole entry to a helper hung on Windows module-init with a taller mutual-rec call graph).
4. **Unique loop index names** per fn (`si`, `bi`, `ei`, ...).
5. **Sym pool in parser** - `k8pPoolSym` / `k8pSymCopy`; AST field `"sym"`.
6. **<=~8 top-level fn** where possible - import of `eval.kab` with 9+ mutual-rec pub helpers has hung on Windows.
7. **ASCII-only in `.kab` comments** - em-dash can trip older `kstyle_preprocess`; scan is now UTF-8 safe.
8. **Parser body/cond stacks** - nested if/for/while/try push `k8pBlockBody`/`k8pCond` so nested stmts do not clobber the outer node.

## Fas 2 VM notes

`src/bytecode/vm.rs`: LoadLocal prefers frame `local_vals`; StoreLocal mirrors to `env` for `__oid` writeback; after Call only object locals refresh by oid.

## Eval subset (Fas 1.3+ / K1c–K1f)

**Gates:** `evalSource(source)` / `evalSourceKab(source)` — Kabootar interpreter (`evalSourceWith`); `evalSourceRust(source)` — Rust `kv8_eval_source` fallback. **H4 product path:** prefer Kab (`preferKabEval()`); Rust only for gaps.

Literals, ident (incl. `this`), member (incl. `?.`), index (`a[i]`), array literals, unary `!` / `typeof`, ternary (`? :`), template literals (`` `a=${n}` `` / `` `${a + b}` ``), call, `new` (`K_NEW`), object literals, let/var/assign (incl. member `this.n = n`), if/else, switch/case/default, while, for, for-in, for-of, break/continue, try/catch/finally, throw, function, `class` + constructor/`this` + `extends` (parent method merge), `await` (unwrap `__k8promise` / sync `.then`), async function (return `{__k8promise,value,then}`), `Promise.resolve` + `.then` microtask queue (`drainMicrotasks`), binary: `+ - * / == === != !== < > <= >= && || ??` (short-circuit for `&&`/`||`/`??`).

## React stub (G10)

`import "kv8/react"`: `createElement`, `useState(hooks, initial)`, `useEffect(hooks, setup, deps?, cleanup?)`, `setState` / `render` → `{ frame, hasClick, patched }`.

- Hook state on `fiber["$hooks"]`; components get `props["$hooks"]`.
- **Live Dom patch:** `hooks["nid"]` + `hooks["ntag"]` + `hooks["cnid0"…]` / `ncn` for nested fiber children; `setTextById` / `setAttrById` / multi-text / nested remount via `appendById`; parent live-registry sync so `paint(parent)` sees child text; `onById` + `dispatchById` for click without remount. Never store `KabootarDom` in `.kab` lets; avoid local name `id`.
- Stack pop in `kv8/eval` uses native `pop` (C2). **`evalSource` / `evalSourceKab` → `evalSourceWith(source, null)`** (K1c–K1g Kabootar path: literals/ops/control + class/`new`/`this`/`extends` + async/`await`/`Promise.resolve` + `.then` microtasks). **`evalSourceRust` → `kv8_eval_source`** fallback (H4: gaps only). `preferKabEval()` keeps the Kab flag on. `evalSourceWith` stays self-host for `extraEnv`. Do not reintroduce Kabootar `evPopStack` rebuild loops.
- Self-host parser: `class` / `new` / `async function` / `await` / `this` AST (`K_CLASS` / `K_NEW` / `K_FN.async` / `K_AWAIT`).
- `useEffect(hooks, setup, deps?, cleanup?)` — skip when `deps[0]` unchanged; optional `cleanup` runs on deps change (never stored — fn-on-hooks hangs). Bumps `hooks["c"+n]` on each run.
- Keep `react.kab` at ~7 top-level fns.

## Tests / DX

| Suite | Command | Time (typical) |
|-------|---------|----------------|
| Fast (lexer/parser) | `cargo test --test kv8_lib -- --test-threads=1` | ~1–2 min |
| Slow (eval/dom/react) | `cargo test --test kv8_lib_slow -- --test-threads=1` | first `kv8/eval` import dominates; later cases reuse shared env |

Notes:

- Always use `--test-threads=1` on Windows; parallel `kv8_lib_slow` fights over `.kbc` / linker.
- After editing `lib/kv8/*.kab`, set `KABOOTAR_KV8_INVALIDATE=1` (or `rm .kabootar/cache/*.kbc`) so tests refresh bytecode.
- If `LNK1104` / locked `kv8_lib_slow*.exe`, kill hung `kabootar.exe` / test processes before rebuild.
- Eval cases share one process-local env (`with_kv8_eval`) so `import "kv8/eval"` runs once; `warm_kv8_module_exports()` + disk `.kbc` warm in `kv8_lib_slow` reuse imports across react smokes.
- `react.kab`: keep ≤~7 top-level fns; never rebind hook bags — mutate and assign back (`fiber["$hooks"] = hooks`).

## Cache

- `.kabootar/cache/*.kbc` — bytecode on disk; invalidated when source mtime is newer.
- Rust module export cache — repeated `import "kv8/eval"` in one process reuses exports (see `src/modules/mod.rs`).

If while-loops hang after editing `eval.kab`, delete stale cache:

```bash
rm .kabootar/cache/eval.kab.kbc
```
