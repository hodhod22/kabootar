# DX tooling (`cli`, `log`, `validate`, `auth`, `test`, registry)

Kab-first developer experience modules and CLI commands for formatting, docs, tests, logging, validation, auth, and the local package registry UI.

## Kab modules

| Import | Role |
|--------|------|
| `cli` | `parseArgs` / `hasFlag` / `flag` / `positional` |
| `log` | leveled logger (`create`, `info`, `warn`, …) |
| `validate` | schema helpers + re-exports `validation` |
| `auth` | sessions + MAC tokens on `crypto` |
| `test` | `assertEq` / `assertTrue` / `summary` |
| `test/mock` | `mockFn` / `returns` / `call` / `calledTimes` |

## CLI

```text
kabootar repl
kabootar fmt [--check] <file.kab>
kabootar doc [path] [--out FILE]
kabootar test [path] [--coverage]
kabootar registry web [--port 8787]
kabootar registry list
```

- **doc** — extracts `///` above `pub fn` / `fn` into Markdown  
- **test** — runs `*_test.kab` / `*.test.kab`; success if result is `true` or `{ ok: true }`  
- **coverage** — module-hit report (import mention heuristic over `lib/`)  
- **registry web** — HTML + `/api/packages` for `.kabootar/registry/`

## Files

- `lib/{cli,log,validate,auth,test}.kab`, `lib/test/mock.kab`
- `src/cli/{doc,test_runner,registry_web}.rs`
- `tests/dx_smoke_test.kab`, `tests/dx_tooling.rs`

Roadmap: [ROADMAP.md](ROADMAP.md) **Våg DX-TOOL**.
