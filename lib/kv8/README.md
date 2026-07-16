# Kv8 in Kabootar (`lib/kv8`)

Self-hosted Kv8 JS-subset: lexer → parser → eval → dom. Import chain:

```
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
7. **ASCII-only in `.kab` comments** - em-dash can trip `kstyle_preprocess` UTF-8 scan.

## Fas 2 VM notes

`src/bytecode/vm.rs`: LoadLocal prefers frame `local_vals`; StoreLocal mirrors to `env` for `__oid` writeback; after Call only object locals refresh by oid.

## Eval subset (Fas 1.3+)

Literals, ident, member, call, let/var/assign, if/else, while, for, try/catch, throw, function, binary: `+ - * / == === != !== < > <= >= && || ??` (short-circuit for `&&`/`||`/`??`).

## Tests

| Suite | Command | Time |
|-------|---------|------|
| Fast (lexer/parser) | `cargo test --test kv8_lib` | ~1–2 min |
| Slow (eval/dom) | `cargo test --test kv8_lib_slow -- --test-threads=1` | ~5–10 min |

## Cache

- `.kabootar/cache/*.kbc` — bytecode on disk; invalidated when source mtime is newer.
- Rust module export cache — repeated `import "kv8/eval"` in one process reuses exports (see `src/modules/mod.rs`).

If while-loops hang after editing `eval.kab`, delete stale cache:

```bash
rm .kabootar/cache/eval.kab.kbc
```
