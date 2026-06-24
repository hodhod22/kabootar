# Kabootar Rendering Engine

Kabootar's native stack (layer 2) includes a full **render pipeline** independent of host browsers.

## Pipeline

```
KML / kdom_create
       ↓
  DomNode tree (with IDs + event listeners)
       ↓
  KSS stylesheet (kstyle_parse) — Chrome-dark theme default
       ↓
  LayoutEngine — flexbox-inspired box layout + text measurement (word wrap, line-height)
       ↓
  Paint — styled HTML compositor frame + rasterized text spans
       ↓
  Raster — CPU pixel buffer (RGBA) via rasterize_tree()
       ↓
  Events — hit-test layers, kb_click / kb_poll_events
       ↓
  frame_buffer → host (WASM web_sys) or native shell (winit+softbuffer)
```

## API

### Styles (KSS)

```kabootar
kstyle_parse("
  body { display: flex; flex-direction: column; padding: 24px; background: #292a2d; }
  h1 { font-size: 32px; color: #8ab4f8; }
  .card { background: #35363a; padding: 16px; border-radius: 12px; }
");
```

Supports: `display`, `color`, `background`, `font-size`, `font-weight`, `line-height`, `white-space`, `padding`, `margin`, `width`, `height`, `border-radius`, `flex-direction`, `gap`, plus inline `style=""` attributes.

### Text layout

Kabootar measures and wraps text with proportional glyph widths (not fixed `char × 8px`).

```kabootar
let layout = ktext_layout("Long paragraph that wraps inside a box", 200, 16);
// layout = { width, height, lines, line_height }

let size = ktext_measure("iii WWW", 16);   // [width, height]
```

CSS on text nodes:

```kabootar
kstyle_parse("
  p { font-size: 16px; line-height: 1.5; white-space: normal; }
  pre { white-space: pre-wrap; }
  .label { white-space: nowrap; }
");
```

| Property | Values |
|----------|--------|
| `line-height` | `normal` (1.25×), `24px`, `150%`, or unitless `1.5` |
| `white-space` | `normal` (wrap), `nowrap`, `pre-wrap` |

**TTF rendering:** set `KABOOTAR_FONT=/path/to/Roboto-Regular.ttf` for fontdue glyph rasterization. Without it, Kabootar uses a built-in proportional metrics engine (layout + paint still work). Replace `assets/fonts/KabootarUI.ttf` with a real font (>500 bytes) to load automatically at build time.

### Paint single node

```kabootar
let ui = kml("<html><body><h1>App</h1></body></html>");
let frame = kdom_paint(ui, 1280, 720);
// frame = { width, height, html, text, nodes, layers }
```

### Pixel raster frame

```kabootar
kb_paint();
let px = kb_pixels();   // { width, height, bytes, backend, gpu? }
```

### GPU backend (wgpu)

```kabootar
kb_set_backend("gpu");   // or "cpu"
println(kb_backend());
println(kb_gpu_info());  // { available, device, backend, uploads }
```

When `gpu` feature is enabled and a GPU adapter is available, frames are uploaded to wgpu textures (`backend: "gpu"`). Otherwise Kabootar falls back to CPU raster automatically.

Native shell with GPU presentation:

```bash
cargo run --no-default-features --features docai,codai,shell,gpu -- shell
```

### Events

```kabootar
let btn = kdom_query(ui, "button");
kdom_on(btn, "click", "on_click");
kb_paint();
kb_click(120, 40);
let events = kb_poll_events();   // [{ node, type, handler, x, y }]
```

### Browser compositor

```kabootar
kb_mount(ui);
kb_viewport(1280, 720);
kb_theme("h1 { color: #8ab4f8; }");
let frame = kb_paint();
let scene = kb_composite();   // frame + OS windows + tab info
kb_host_sync();               // publish to host bridge
host_paint();                 // read compositor HTML
```

### OS ↔ Browser binding

```kabootar
let win = os_window_create("My App", 1280, 800);
os_window_bind(win, 1);       // link OS window to browser tab 1
```

### VFS pages

Navigate to KML files in Kabootar OS virtual filesystem:

```kabootar
os_write("/apps/page.kml", "<html><body><h1>From VFS</h1></body></html>");
kb_navigate("kabootar://vfs/apps/page.kml");
```

## Chrome-like shell

| Mode | How |
|------|-----|
| WASM + host DOM | Open `kabootar-shell.html` — Chrome UI mounts compositor HTML via `kb_run_ui()` / `host_mount()` |
| Native pixels | `cargo run --features shell -- shell` — winit window + softbuffer |

## Modules

| Module | Path |
|--------|------|
| KSS CSS | `src/runtime/kstyle.rs` |
| Layout | `src/runtime/render/layout.rs` |
| Text layout | `src/runtime/render/text.rs` |
| Paint | `src/runtime/render/paint.rs` |
| Raster | `src/runtime/render/raster.rs` |
| GPU (wgpu) | `src/runtime/render/gpu.rs` |
| Backend | `src/runtime/render/backend.rs` |
| Compositor | `src/runtime/render/mod.rs` |
| Events | `src/runtime/events/mod.rs` |
| Frame buffer | `src/runtime/frame_buffer.rs` |
| Browser | `src/runtime/kabootar_browser/mod.rs` |
| Desktop shell | `src/shell/mod.rs` |

See [PLATFORM.md](PLATFORM.md) and [BROWSER.md](BROWSER.md).
