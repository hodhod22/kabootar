# Kv8 — Kabootar's own engine (DOM + CSS + JS-subset)

Kv8 is Kabootar's **native runtime engine** — not Google V8. It embeds:

- **JS-subset** interpreter (lexer + parser + evaluator + bytecode bridge)
- **DOM** via KDOM (`DomNode`, `document.createElement`, …)
- **CSS** via KSS (`kv8_css`, computed styles, paint pipeline)
- **JIT** for hot `for` loops (bytecode compilation after 8 iterations)
- **VFS `.kv8` modules** — KML + CSS + script bundles for browser apps

## Quick start

```kabootar
let ctx = kv8_create();
kv8_css(ctx, "#app { color: #e8eaed; padding: 16px; }");
kv8_run_ui(ctx, "<div id='app'><h1>Kabootar</h1></div>", "#app { background: #1a1a2e; }");
kv8_paint(ctx, 1280, 720);
```

## JavaScript-subset in Kv8

```kabootar
kv8_eval(ctx, "
  function greet(name) { return 'Hello ' + name; }
  let add = (a, b) => a + b;
  let btn = document.createElement('button');
  btn.textContent = greet('Kv8');
  document.appendChild(btn);
  if (add(1, 2) == 3) { btn.style.color = '#00ff88'; }
  for (let i = 0; i < 3; i = i + 1) { /* hot loops JIT to bytecode */ }
");
let tree = kv8_dom(ctx);
```

Supported syntax: `let`, `return`, `if`/`else`, `for`, `function`, arrow `=>` (expr + block), comparisons (`==`, `!=`, `<`, `<=`, `>`, `>=`), `&&`/`||`/`!`, strings, numbers, `+`/`-`, member access, calls.

Arrow expressions compile to Kabootar bytecode on first call via `bytecode_bridge`.

## KSS in Kabootar syntax (`kstyle { }`)

Write CSS without raw strings — preprocessed before compile:

```kabootar
kstyle {
  .card { color: #e8eaed; padding: 12px; }
  #title { font-size: 24px; }
}
let css = __kstyle;
```

Expansion calls `kstyle_reset()`, `kstyle_rule(...)`, `kstyle_commit()` (see `kstyle_lang.rs`).

## `.kv8` VFS modules

Bundle format for browser + Kv8:

```
---kml---
<div id="app"><h1>App</h1></div>
---css---
#app { color: #00ff88; }
---script---
let h = document.querySelector('h1');
h.textContent = 'Live';
```

Load from OS VFS:

```kabootar
os_write("/apps/home.kv8", "...bundle...");
let ctx = kv8_create();
let root = kv8_load_vfs(ctx, "/apps/home.kv8");
```

Browser integration:

```kabootar
kb_navigate("kabootar://vfs/apps/home.kv8");
kb_run_kv8();  // runs ---script---, merges ---css--- into paint
```

## API

| Function | Description |
|----------|-------------|
| `kv8_info()` | Engine metadata (`jit`, `vfs_kv8`, `arrow_bytecode`) |
| `kv8_create()` | New isolate/context handle |
| `kv8_eval(ctx, script)` | Run JS-subset |
| `kv8_css(ctx, css)` | Load KSS stylesheet |
| `kv8_dom(ctx)` | Export DOM as `KabootarDom` |
| `kv8_paint(ctx, w, h)` | Layout + raster + publish frame |
| `kv8_computed_style(ctx, node)` | KSS computed style object |
| `kv8_run_ui(ctx, kml, css)` | Parse KML + CSS and paint |
| `kv8_jit_info(ctx)` | `{ compiled_loops, loop_hits }` |
| `kv8_load_vfs(ctx, path)` | Load `.kv8` from OS VFS |
| `kb_run_kv8()` | Run script from active browser tab |

Also available via `import "kv8";` (re-registers natives).

## Optimizations (Kv8 vs V8)

Kv8 is **not** a drop-in V8. The **Rust host** Kv8 runtime avoids a tracing GC (values live in Rust). Self-hosted Kv8 under `lib/kv8` runs as ordinary Kabootar and uses the **default GC** heap. Systems ownership (`@manual`, `owned_*`, `os/mem`) is for kOS / low-level Kabootar — not for web/Kv8 app code.

Safe wins implemented in `opt.rs`:

| Technique | Kv8 status | Notes |
|-----------|------------|-------|
| Inline caching / hot-path predictor | ✅ | Monomorphic `document.*` / `console.*` call sites |
| Zero-copy DOM | ✅ partial | `document`/`console` singletons + DOM id index (no full Arc yet) |
| Parallel CSS + GPU | ✅ partial | KSS style cache; GPU upload after CPU paint (not GPU selector matching) |
| OS-specific machine code JIT | ⏭️ skipped | Kabootar bytecode bridge instead (portable, sandboxed) |
| Lazy parse + precompile | ✅ | `program_cache` + `arrow_cache` on context |
| Hidden classes | ⏭️ skipped | HashMap model; predictor covers hot natives |
| Predictive GC | ✅ N/A | Host Kv8: Rust lifetimes; Kabootar apps: GC default |
| WASM → native JIT | ⏭️ separate | Guest WASM via wasmi (`browser_platform`); Kv8 JIT → bytecode |
| Speculative execution / deopt | ⏭️ skipped | Threshold JIT only |
| OS integration | ✅ | VFS `.kv8`, journal, no extra syscall layer in hot path |

### Kabootar advantages (by design)

- **Web/default = GC** — no ownership noise for app and Kv8-in-Kabootar code
- **Systems = `@manual`** — MemBox / move+drop for kOS buffers (`import "os/mem"`)
- **Browser = OS** — `kabootar://vfs`, compositor, kernel capabilities
- **Host Kv8** — no tracing GC pauses in the Rust Kv8 engine
- **Predictive JIT** — hot `for` loops compile after 8 iterations with scope bridge

```kabootar
let info = kv8_opt_info(ctx);
// program_cache, arrow_cache, style_cache, dom_nodes_indexed, hot_members, compiled_loops
```

## Architecture

```
kv8/
  ast.rs            — Stmt / Expr AST
  lexer.rs          — JS-subset tokenizer
  eval.rs           — parser + interpreter + JIT hooks
  bytecode_bridge.rs — arrow/loop → Kabootar bytecode
  jit.rs            — hot-loop threshold + cache
  opt.rs            — hot-path predictor, DOM index, parse/arrow/style caches
  vfs_module.rs     — .kv8 bundle parser + loader
  context.rs        — Kv8Context (document + stylesheet + scope + jit)
  register.rs       — natives wired to KDOM + KSS + render pipeline
```

Pipeline: **Kv8 script → KDOM tree → KSS → RenderEngine → frame_buffer**

JIT: **`for` loop body** → after 8 iterations → `try_compile_loop_body` → bytecode VM.

## OS integration

Kv8 runs in the same session as Kabootar OS:

- `os_write` → journal + page cache + AI prefetch
- `os_spawn` → thread pool + AI habits
- `os_recovery_restore` → golden VFS partition (2s)
- Kernel capabilities: `kv8-engine`, `kv8-jit`, `kv8-vfs-modules`, `kstyle-lang`

Use `kb_navigate("kabootar://vfs/...")` with `kb_run_kv8()` for full apps.

See [OS.md](OS.md), [KML.md](KML.md), [RENDERING.md](RENDERING.md).
