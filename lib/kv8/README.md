# Kv8 in Kabootar (`lib/kv8`)

Self-hosted Kv8 JS-subset: lexer → parser → eval → dom. Import chain:

```
kv8/dom  → kv8/eval → kv8/parser → kv8/lexer
                      kv8/host
                      kv8/defs
```

## Module rules (Kabootar bytecode)

Learned from self-host; same VM limits apply here.

1. **No re-entrant `let` in recursive fn** — save AST fields in module globals (`evBxL`, `evBindSym`, …) before calling yourself.
2. **Bracket access for AST keys** — `node["sym"]`, not `.sym` where names collide with locals.
3. **`while` + assign in module fn** — the cond loop and `let bi = 0` body loop must live in the **same pub entry fn** as the caller expects. No `pub fn a() { return b() }` if `b` contains the while; no private `evRunBlock()` helper between entry and the loop.
4. **`evalSource` and `evalSourceWith` each inline the program loop** — do not delegate to each other.
5. **Unique loop index names** per fn (`si`, `bi`, `ei`, …); never duplicate `let i = 0` in the same fn.
6. **Sym pool in parser** — `k8pPoolSym` / `k8pSymCopy`; AST field `"sym"`, not `"name"`.
7. **≤~7 top-level fn per module** where possible — large modules slow compile and can OOM on Windows.

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
