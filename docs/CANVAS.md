# Kabootar Canvas 2D (advanced)

HTML Canvas-liknande 2D-rendering integrerad i Kabootar compositor (`kb_paint`).

## Snabbstart

```kabootar
let c = kdom_create("canvas");
kdom_set_attr(c, "width", "400");
kdom_set_attr(c, "height", "300");
let ctx = canvas_get_context(c, "2d");

canvas_set_fill_style(ctx, "#3366cc");
canvas_fill_rect(ctx, 0, 0, 400, 300);

canvas_set_fill_style(ctx, "#ffffff");
canvas_fill_text(ctx, "Kabootar", 20, 40);

kb_mount(page_with_canvas);
kb_viewport(800, 600);
kb_paint();
```

Offscreen utan DOM:

```kabootar
let ctx = canvas_create(128, 128);
canvas_fill_rect(ctx, 0, 0, 128, 128);
let pixels = canvas_to_pixels(ctx);
```

## API

| Kategori | Natives |
|----------|---------|
| Skapa | `canvas_create(w,h)`, `canvas_bind(node)`, `canvas_get_context(node\|id, "2d")` |
| Rektanglar | `canvas_fill_rect`, `canvas_stroke_rect`, `canvas_clear_rect` |
| Stil | `canvas_set_fill_style`, `canvas_set_stroke_style`, `canvas_set_global_alpha`, `canvas_set_line_width`, `canvas_set_font` |
| Transform | `canvas_save`, `canvas_restore`, `canvas_translate`, `canvas_scale`, `canvas_rotate`, `canvas_set_transform` |
| Paths | `canvas_begin_path`, `canvas_move_to`, `canvas_line_to`, `canvas_arc`, `canvas_rect`, `canvas_close_path`, `canvas_fill`, `canvas_stroke` |
| Text | `canvas_fill_text`, `canvas_measure_text` |
| Gradient | `canvas_create_linear_gradient`, `canvas_gradient_add_color_stop` |
| Bitmap | `canvas_draw_image`, `canvas_get_image_data`, `canvas_put_image_data`, `canvas_to_pixels` |
| Info | `canvas_info()` |

Kontextobjekt: `{ id, width, height, kind: "2d", layer, dom_id? }` — alla `canvas_*`-anrop accepterar id eller kontextobjekt.

## Två vägar

### 1 — Native motor + JS-syntax

`canvas_create` / `canvas_get_context` ger ett kontextobjekt med metoder som i browser-API:

```kabootar
let ctx = canvas_create(64, 64);
ctx.fillStyle = "#ff0000";
ctx.fillRect(0, 0, 32, 32);
```

Stödda properties: `fillStyle`, `strokeStyle`, `globalAlpha`, `lineWidth`, `font` (synkas till native motor och host WASM-kontext).

Flat API (`canvas_fill_rect(ctx, …)`) fungerar fortfarande parallellt.

### 2 — Host WASM-bridge

`document.createElement("canvas")` skapar ett host-canvas (riktig `HtmlCanvasElement` på WASM, native fallback annars):

```kabootar
let canvas = document.createElement("canvas");
let ctx = canvas.getContext("2d");
ctx.fillStyle = "#3366cc";
ctx.fillRect(0, 0, 80, 60);
```

Host-kontext speglar till native `canvas2d` (compositor + tester). På WASM körs även `web_sys::CanvasRenderingContext2d` parallellt.

`bp_info()["host_canvas"]` rapporterar backend (`web_sys` / `native-fallback`).

## Compositor

- `<canvas width="…" height="…">` i KDOM får layoutstorlek från attribut.
- Vid `kb_paint()` blits canvas-backing store in i frame (`raster.rs`).
- Skala: canvas-storlek → layout-box (som HTML canvas).

## Skillnad mot WebGL

| | Canvas 2D | WebGL |
|--|-----------|-------|
| API | 2D paths, text, gradients | Shaders, buffers, triangles |
| Output | Compositor-blitt | Hela `frame_buffer` |
| JS-syntax | `ctx.fillRect`, `ctx.fillStyle = …` | `gl.drawElements`, `gl.clearColor`, … |

### WebGL via `getContext`

```kabootar
let c = kdom_create("canvas");
kdom_set_attr(c, "width", "64");
let gl = canvas_get_context(c, "webgl2");
gl.clearColor(10, 20, 30, 255);
let vbo = gl.createBuffer("array", [-0.8, -0.8, 0.8, -0.8, 0.0, 0.8]);
let ibo = gl.createIndexBuffer([0, 1, 2]);
gl.bindBuffer(vbo);
gl.bindBuffer(ibo);
gl.uniform4f(0, 1.0, 0.5, 0.0, 1.0);
gl.drawElements(3, 0);
```

Host-vägen:

```kabootar
let gl = document.createElement("canvas").getContext("webgl");
gl.drawArrays(3);
```

Flat API (`webgl_create`, `webgl_draw_elements`, …) fungerar parallellt.

### Texturer

```kabootar
let tex = gl.createTexture();
gl.texImage2D(tex, 16, 16, rgba_bytes);
// eller från 2D-canvas:
gl.texImage2D(tex, canvas_ctx);
gl.bindTexture(tex);
let vbo = gl.createBuffer("array", [-1,-1,0,0, 1,-1,1,0, 0,1,0.5,1]);
gl.bindBuffer(vbo);
gl.drawArrays(3);
```

Se [RENDERING.md](RENDERING.md) och [BROWSER_V2.md](BROWSER_V2.md).
