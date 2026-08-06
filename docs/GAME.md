# Kabootar Game Runtime (v2.59)

Spel-loop, input och enhetlig canvas-yta för 2D-spel i Kabootar Browser och host-WASM.

## Snabbstart

```kabootar
platform_use("kabootar")
let surf = game_surface_create(800, 600)
let ctx = surf["ctx"]
let player_x = 100

fn game_loop(dt) {
    if input_is_down("ArrowLeft") {
        player_x = player_x - 5
    }
    ctx.fillStyle = "#000033"
    ctx.fillRect(0, 0, 800, 600)
    ctx.fillStyle = "#00ff00"
    ctx.fillRect(player_x, 300, 40, 40)
    surf.present()
    requestAnimationFrame(game_loop)
}

requestAnimationFrame(game_loop)
```

Synkron test (en frame):

```kabootar
fn boot() {
    fn on_frame(dt) {
        ctx.fillStyle = "#ff0000"
        ctx.fillRect(0, 0, 40, 40)
        surf.present()
    }
    requestAnimationFrame(on_frame)
    game_tick()
}
boot()
```

**Tips:** deklarera delad spelstate (`let player_x`, `let state = { ... }`) på skript-topnivå så att callbacks i nästlade `fn` kan läsa den.

## API

| Funktion | Beskrivning |
|----------|-------------|
| `requestAnimationFrame(cb)` | Köa callback `fn(dt)` för nästa frame |
| `cancelAnimationFrame(id)` | Avbryt schemalagd frame |
| `kb_on_frame(cb)` | Alias för `requestAnimationFrame` |
| `game_tick()` | Kör en frame, returnerar `{ delta_ms, frame, time_ms }` |
| `game_run(max)` | Kör frames tills kön är tom eller `max` nåtts |
| `game_surface_create(w, h)` | Enhetlig canvas + compositor (`kabootar` eller `host` på WASM) |
| `game_surface_create_3d(w, h)` | WebGL-yta + `frame_buffer`-present |
| `surf.present()` | `kb_paint` / pixel-publish |
| `input_key_down/up(key)` | Simulera tangent (test + shell) |
| `input_poll()` | `{ pressed, released, down, pointer }` |
| `input_is_down(key)` | Hålls tangent ned? |
| `game_info()` | Runtime-info |

## Plattform

| Miljö | Surface | Loop |
|-------|---------|------|
| Kabootar shell | `layer: "kabootar"` → KDOM + `kb_paint` | winit ~60 FPS + `game_tick` |
| Native test | `game_tick` / `game_run` | Manuell |
| WASM + `platform_use("host")` | `layer: "host"` → web_sys + pixel mirror | Host `requestAnimationFrame` (planerat) |

## Nästa steg (3D)

Implementerat i **v2.60-3D**:
- **vec3/vec5** vertex-buffers (position + UV)
- **MVP-matriser**: `gl.perspective`, `gl.lookAt`, `gl.rotateModelY`, `gl.uniformMatrix4fv`
- **Z-buffer** med depth test (närmare trianglar vinner)
- `game_surface_create_3d` med perspektivkamera

```kabootar
let gl = webgl_create(64, 64);
gl.lookAt(0, 0, 3, 0, 0, 0, 0, 1, 0);
gl.rotateModelY(45);
let vbo = gl.createBuffer("array", [-0.5,-0.5,0.5, 0.5,-0.5,0.5, 0.0,0.5,0.5]);
gl.bindBuffer(vbo);
gl.drawArrays(3);
```

**GPU 3D** (med `--features gpu`): vec3-ritning och **texturerade vec5** (xyz+uv) går via wgpu WGSL-pipeline (`gpu3d: wgpu-pipeline` / `wgpu-pipeline+msaa4` i `webgl_info()`). MSAA×4 när adaptern stödjer det (resolve → readback); shell-present mappar DisplayServer-vsync (`fifo`/`immediate`) till wgpu `PresentMode`; `surf.present()` på 3D-yta undviker redundant compositor-publish om pixels redan finns. CPU-raster används som fallback (ingen adapter, `depth_test` av, eller tom textur).

**GP0b material uniforms / bind groups:**
- **Group 0 (frame):** `view_proj` mat4
- **Group 1 (material):** `model` mat4 + `color` vec4 + `uv_xform` vec4 (xy=scale, zw=offset; default `1,1,0,0`) + texture/sampler när texturerat
- `gl.uniform4f(0, …)` → draw color; `gl.uniform4f(1, sx, sy, ox, oy)` → UV-transform
- `gl.uniformMatrix4fv(matrix)` / loc 0 → explicit MVP; `gl.uniformMatrix4fv(1, modelMatrix)` → model utan att tvinga explicit MVP
- `gl.rotateModelY` sätter fortfarande model-matrisen (CPU/GPU)

## Samples

```bash
kabootar run examples/game_2d_smoke.kab
kabootar run examples/game_playable_2d.kab
kabootar run examples/game_3d_triangle.kab
kabootar mod init game    # 2D loop + physics scaffold
kabootar mod init game3d  # 3D mesh + shaders/solid.wgsl
```

## Kab-spel-lib (GP1–GP3 subset)

Tunna `.kab`-moduler under `lib/game/` (Kab CoW: mutatorer returnerar noden — `root = setLocal(root, …)`):

| Import | API |
|--------|-----|
| `import "game/scene"` | `createNode`, `addChild`, `setLocal`, `setLayer`, `worldPos` (Parent-walk) |
| `import "game/render"` | `createMesh`, `createIndexedMesh`, `setColor`, `drawMesh`, `drawIndexedMesh`, `drawMeshInstanced`, `drawIndexedMeshInstanced` |
| `import "game/input"` | `createActions`, `actionPressed` |
| `import "game/time"` | `dtSec`, `createFixed`, `fixedTick` |
| `import "game/gltf"` | `loadGltfJson` → `{ floats, indices?, color, animations }` (glTF 2.0 JSON subset) |
| `import "game/atlas"` | `bakeAtlas(images)` row-pack → `{ width, height, rgba, uvs }` |
| `import "game/batch"` | `buildSpriteQuads`, `createSpriteBatch`, `drawSpriteBatch`, `buildTilemapSprites` |
| `import "game/physics"` | `aabbOverlap`, `circleOverlap`, `resolveAabb`, `rayAabb`, `characterStep` |
| `import "game/ecs"` | `createWorld`, `spawn`, `add`, `get`, `has`, `query` |
| `import "game/audio"` | `createBus`, `setBusVolume`, `playPcm`, `makeTone`, `playTone`, spatial/group/duck/stream (GP6h) |
| `import "game/terrain"` | heightmap, LOD mesh, splat, async streaming poll (`beginAsyncLoad`/`pollStreamingLoads`) (GP6d) |
| `import "game/procgen"` | seeded RNG, `noise2d`/`fbm2d`, dungeon, scatter, heightmap fill (GP6l) |
| `import "game/i18n"` | catalogs, `t` / `tn`, locale switch (GP6j) |
| `import "game/stats"` | counters, achievements, save/load (GP6k) |
| `import "game/debug"` | `drawLine2d`, `drawAabb`, `drawCircleApprox` |
| `import "game/assets"` | `createDb`, `registerVfs`/`registerHost`, `loadText`/`loadBytes`/`loadPng`/`loadGltf`, `watchAll`/`pollReload` |
| `import "game/nav"` | `createGrid`, `setBlocked`, `astar`, `pathCost` |
| `import "game/net"` | session/snapshot, relay + HTTP hub, remote session server (`createRemoteHttpTransport`) |
| `import "game/xr"` | XR session, compositor process/IPC, immersive loop (`xrRunImmersiveFrames`), HMD present |
| `import "game/editor"` | `createEditor`, `buildHierarchy`, `buildInspector`, `buildSceneView` (+ **GPU viewport**), `selectNode`, `refresh` |
| `import "game/sandbox"` | **Spelbyggare** — session/canvas, Play↔Edit↔Learn, level pack (`ksandbox`), `applyLiveParams`/`setLearnParam` |
| `import "sim"` / `sim/robot` | **Sim/robotik** — `createWorld`/`createHinge`/`createSlider`/`step`, 3-DOF `createArm3`, encoders/IMU, `worldToEditor` |
| `import "game/profiler"` | `createProfiler`, `beginFrame`/`endFrame`, `sample`, `drawOverlay` |
| `import "game/host"` | `useHost`, `createHostSurface`, `presentOnce` |
| `import "game/hot"` | `watch(path)`, `poll()` → changed paths (mtime) |
| `import "game/shader"` | `loadSolid`/`loadTextured`/`loadSolidFromFile`/`loadTexturedFromFile`, `info`, `pollReload` |

Natives: `gltf_load_json`, `image_decode_png`, `asset_watch`, `asset_poll`, `host_read_bytes`, `gpu3d_load_wgsl`, `gpu3d_load_wgsl_from_file`, `gpu3d_shader_info`. Fixtures: `fixtures/game/triangle.gltf`, `fixtures/game/px.png`, `fixtures/game/solid.wgsl`.

`createBuffer` accepterar **Float32Array** (bulk) utöver Array-of-numbers. Frame-smoke: `tests/perf_p0_smoke.rs` (P9: `delta_ms` < 100). GC-frame: `gc_frame_stats` / `gc_set_frame_budget` (P3). Playable: `examples/game_playable_2d.kab`. **Spelbyggare / sandlåda:** `import "game/sandbox"` + `science/mechanics` — **session** (`createSession`/`stepSession`/`runSessionFrames`: canvas + pointer), Play↔Edit↔Learn (`enterEdit`/`applyEditorAndPlay`/`enterLearnMode`/`setLearnParam`), multi-level (`defaultLevelPack`/`nextLevel`/`saveLevelPack`). Exempel `examples/sandbox_force_puzzle.kab`, test `tests/game_sandbox.rs`. **Sim/robot twin:** `examples/sim_robot_arm.kab`, `tests/sim_robot.rs`.

**WGSL (GP0e):** `gpu3d_load_wgsl("solid"|"textured", source)` bygger om wgpu-pipeline vid hash-ändring; fil-load registreras för hot reload via `asset_poll` (`.wgsl`). GLSL `compileShader*` lagras fortfarande (CPU/legacy); GPU-path använder WGSL.

**Roadmap (produktion):** [ROADMAP.md](ROADMAP.md) **Våg GP** — GP0–GP5 ✅; **GP6** (`anim`…`xr` ✅ subset); **GP7** editor a–g ✅ subset. Ship: [SHIP.md](SHIP.md).

Editor: `import "game/game_editor"` — `bootEditor`, save/load `.kscene`, multi-select, shortcuts. Tester: `tests/game_editor.rs`, `tests/game_gp6_gp7.rs`.

Se [CANVAS.md](CANVAS.md) och [BROWSER_V2.md](BROWSER_V2.md).
