# Browser Platform v2 — efter Kv8 v1.0

Kabootar Browser v2 bygger på **Kv8** (DOM/CSS/JS) och lägger till sex webbplattforms-API:er som native moduler i `src/runtime/browser_platform/`.

## Översikt

| Modul | Status | Version |
|-------|--------|---------|
| **WebAssembly** | wasmi gäst-körning, `wasm_run` | v2.51 |
| **WebRTC** | SDP, ICE (STUN/TURN), RTP tracks | v2.56 |
| **DevTools** | Console, inspector, Elements, Network, Profiler, live edit | C9 |
| **Extensions** | Manifest, content scripts vid navigation | v2.55 |
| **PWA** | Manifest → OS VFS, service worker, offline cache | v2.55 |
| **WebGL** | Buffers, draw_elements, shader pipeline | v2.57 |
| **Canvas 2D** | Paths, gradients, transforms, KDOM compositor | v2.58 |

Plattformsversion: **2.58.0** (`bp_info().version`). Se [CANVAS.md](CANVAS.md).

## Snabbstart

```kabootar
let bp = bp_info();
// { platform, version, wasm, webgl, webrtc, devtools, extensions, pwa }

// WebAssembly — ladda .wasm från OS VFS och kör export
os_write("/apps/add.wasm", wasm_bytes);
let mod = wasm_load("/apps/add.wasm", "add");
wasm_run(mod["id"], "add", [5, 7]);  // → 12

// WebGL — shader + rita till compositor
let gl = webgl_create(1280, 720);
let prog = webgl_shader(vertex_src, fragment_src);
webgl_use_program(gl["id"], prog);
webgl_clear(gl["id"], 26, 26, 46, 255);
webgl_draw(gl["id"], 3);

// WebRTC — peer, ICE, tracks
let peer = webrtc_create_peer();
let offer = webrtc_create_offer(peer);
webrtc_set_remote(peer, answer_sdp);
webrtc_gather_ice(peer);
webrtc_add_track(peer, "video");
webrtc_stats(peer);

// DevTools — Kv8 console.log går hit automatiskt
devtools_log("info", "Hello from Kabootar");
let logs = devtools_dump();
devtools_breakpoint("/apps/demo.kv8", 12);
devtools_source_map("bundle.js", "/apps/demo.kv8");

// Extensions — content scripts injiceras vid kb_run_kv8 / navigation
ext_install('{"name":"DarkMode","version":"1.0","content_scripts":["document.body.classList.add(\"dark\");"]}');
ext_list();

// PWA — installera som native OS-app + offline cache
let url = pwa_install('{"name":"My App","short_name":"myapp","start_url":"/"}');
pwa_register_worker("/", sw_script);
pwa_fetch_cached("/");
kb_navigate(url);
```

## Arkitektur

```
Kabootar Browser (kbrowser)
    ├── Kv8 (JS/DOM/CSS)
    ├── browser_platform/
    │   ├── wasm_guest.rs   — wasmi gäst-WASM + env.log host-import
    │   ├── webgl.rs        — WebGL → frame_buffer + wgpu
    │   ├── webrtc.rs       — SDP/ICE/tracks
    │   ├── devtools.rs     — Console + Inspector + debugger
    │   ├── extensions.rs   — Tilläggsmanifest + content scripts
    │   └── pwa.rs          — manifest.webmanifest → OS VFS + SW cache
    └── kabootar_browser/   — flikar, navigation, compositor
```

## Faser

### v2.50 — Foundation
- [x] `bp_info()` + per-modul `*_info()`
- [x] WASM magic-validering + VFS-laddning
- [x] WebGL-kontexthantering + GPU-info
- [x] WebRTC peer + SDP-stub
- [x] DevTools console + DOM-inspector
- [x] Extension manifest-parser
- [x] PWA manifest → `/apps/<slug>/app.kv8`

### v2.51 — WebAssembly full
- [x] wasmi-integration (gäst `.wasm` från C/Rust)
- [x] `wasm_run(id, export, args[])` — typed i32-anrop
- [x] Host-import: `env.log` → DevTools console

### v2.52 — WebGL 3D
- [x] Shader-kompilering (`webgl_shader`, `webgl_use_program`)
- [x] Clear + draw → `frame_buffer::publish_pixels` + wgpu upload
- [x] `webgl_draw(ctx, count)`

### v2.53 — WebRTC streaming
- [x] ICE host + STUN-kandidater (`webrtc_gather_ice`)
- [x] Audio/video tracks (`webrtc_add_track`)
- [x] `webrtc_stats(peer)`

### v2.54 — DevTools IDE
- [x] Console → Kv8 `console.log` hook
- [x] Breakpoints (`devtools_breakpoint`)
- [x] Source maps (`devtools_source_map`)
- [x] Elements-panel i `kabootar-shell.html` (UI + `kb_devtools_json`)

### v2.55 — Extensions + PWA
- [x] Content scripts injiceras vid Kv8-navigation
- [x] Service worker-registrering (`pwa_register_worker`)
- [x] Offline cache (`pwa_fetch_cached`)
- [x] `pwa_install()` → OS app-launcher

### v2.56 — WebRTC STUN/TURN + RTP
- [x] `webrtc_configure_ice(json)` — STUN/TURN-servrar
- [x] Riktig STUN binding (UDP) på native, fallback srflx
- [x] TURN relay-kandidater
- [x] `webrtc_send_rtp` / `webrtc_recv_rtp` media pipeline

### C7 — WebRTC DTLS-SRTP
- [x] SDP med `ice-ufrag` / `ice-pwd` / `fingerprint:sha-256` / `setup`
- [x] `webrtc_create_answer` + `webrtc_add_ice_candidate`
- [x] `webrtc_connect_peers` — lokal DTLS/SRTP-brygga
- [x] SRTP protect/unprotect på RTP-payload

### C8 — PWA fetch events + extension permissions
- [x] `pwa_dispatch_fetch(url)` — FetchEvent till längsta matchande SW-scope
- [x] `pwa_on_fetch(scope, strategy)` — `cache-first` / `offline-only` / `network-stub`
- [x] Auto-detektera `addEventListener('fetch', …)` i SW-script
- [x] `ext_has_permission` / `ext_request_permission` / `ext_revoke_permission`
- [x] Permission-gated `ext_storage_*` + `ext_tabs_query`

### C9 — DevTools network / profiler / live edit
- [x] `devtools_network_record` / `devtools_network_dump` / `devtools_network_clear`
- [x] `devtools_profile_start` / `mark` / `measure` / `stop` / `dump`
- [x] `devtools_live_edit` (text/attr) + `devtools_live_eval`
- [x] Shell snapshot inkluderar `network` + `profiler`

### v2.57 — WebGL buffers
- [x] `webgl_create_buffer` / `webgl_create_index_buffer`
- [x] `webgl_bind_buffer` + `webgl_draw_elements`
- [x] CPU triangle rasterizer + wgpu VBO-spårning

## Tester

```bash
cargo test --test kabootar_browser_v2 --features docai,codai,hw
cargo test --test kabootar_kv8 --features docai,codai,hw
cargo test --test kabootar_kv8_extended --features docai,codai,hw
```

## Kernel-kapabiliteter

`browser-wasm`, `browser-webgl`, `browser-webrtc`, `browser-devtools`, `browser-extensions`, `browser-pwa`

## Se även

- [KV8.md](KV8.md) — JS/DOM/CSS-motor
- [BROWSER.md](BROWSER.md) — kbrowser API
- [RENDERING.md](RENDERING.md) — compositor + GPU
