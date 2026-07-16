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

## Eval subset (Fas 1.3+)

Literals, ident, member, index (`a[i]`), array literals, unary `!` / `typeof`, ternary (`? :`), call, object literals, let/var/assign, if/else, switch/case/default, while, for, for-in, for-of, break/continue, try/catch/finally, throw, function, binary: `+ - * / == === != !== < > <= >= && || ??` (short-circuit for `&&`/`||`/`??`).

## React stub (G10)

`import "kv8/react"`: `createElement`, `useState(hooks, initial)`, `useEffect(hooks, setup)`, `setState(fiber, index, next)`, remount `render`.

- Hook state lives on `fiber["$hooks"]` (plain object). Components receive it via `props["$hooks"]`.
- `useEffect(hooks, setup)` stub runs `setup` each remount/render and returns a 1-based count (assign onto `hooks` in the component if needed — bytecode may copy call-arg objects). No deps / cleanup yet.
- Remount-only: `setState` remounts the tree — no DOM reconcile / fiber diff yet.
- Bytecode may copy objects on read — mutations are written back onto the fiber after the component runs.
- `onClick` (fn prop) sets `hasClick` on the render result; remount via `setState(tree, 0, next)`.
- Function components: non-string `type` is invoked (bytecode fns often are not `typeof "function"`).

Keep `react.kab` at ~7 top-level fns (Windows `.kbc` import hangs/OOM above that).

## Tests / DX

| Suite | Command | Time (typical) |
|-------|---------|----------------|
| Fast (lexer/parser) | `cargo test --test kv8_lib -- --test-threads=1` | ~1–2 min |
| Slow (eval/dom/react) | `cargo test --test kv8_lib_slow -- --test-threads=1` | first `kv8/eval` import dominates; later cases reuse shared env |

Notes:

- Always use `--test-threads=1` on Windows; parallel `kv8_lib_slow` fights over `.kbc` / linker.
- After editing `lib/kv8/*.kab`, set `KABOOTAR_KV8_INVALIDATE=1` (or `rm .kabootar/cache/*.kbc`) so tests refresh bytecode.
- If `LNK1104` / locked `kv8_lib_slow*.exe`, kill hung `kabootar.exe` / test processes before rebuild.
- Eval cases share one process-local env (`with_kv8_eval`) so `import "kv8/eval"` runs once.
- `react.kab`: keep ≤~7 top-level fns; never rebind hook bags — mutate and assign back (`fiber["$hooks"] = hooks`).

## Cache

- `.kabootar/cache/*.kbc` — bytecode on disk; invalidated when source mtime is newer.
- Rust module export cache — repeated `import "kv8/eval"` in one process reuses exports (see `src/modules/mod.rs`).

If while-loops hang after editing `eval.kab`, delete stale cache:

```bash
rm .kabootar/cache/eval.kab.kbc
```
