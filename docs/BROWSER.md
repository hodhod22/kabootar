# Kabootar Browser (`kbrowser`)

Kabootar Browser är **lager 2** — en Chrome-inspirerad motor som renderar **Kabootar DOM** (KML/kdom), oberoende av värdwebbläsaren.

Host-Chrome använder `document`/`window`; Kabootar-appar använder `kbrowser` + `kb_*`.

## Jämförelse med Chrome

| Chrome (host) | Kabootar Browser |
|---------------|------------------|
| `window.location.href` | `kb_location()` / `kbrowser.location` |
| `history.back()` | `kb_back()` |
| `history.forward()` | `kb_forward()` |
| Ny flik | `kb_tab_open(url)` |
| `document.body.innerHTML` | `kb_mount(kdom_node)` + `kb_render()` |
| `navigator.userAgent` | `kb_user_agent()` |

## API

```kabootar
// Navigation + VFS-sidor
os_mkdir("/apps");
os_write("/apps/page.kml", "<html><body><h1>App</h1></body></html>");
kb_navigate("kabootar://vfs/apps/page.kml");

// Compositor (layout + paint)
kb_mount(page);
kb_viewport(1280, 720);
// Mobil (G7): kb_viewport(390, 844, 3, "portrait"); kb_safe_area(47, 0, 34, 0); kb_touch_at(x, y, "start");
// import "kbrowser/mobile_chrome" → mountChrome / goBack / listTabs
// G11: import "kbrowser" / "kbrowser/core"; kb_sync_platform() → {mode,host_os,schemes}
kb_theme("h1 { color: #8ab4f8; }");
let frame = kb_paint();        // { html, text, layers, nodes, width, height }
let scene = kb_composite();    // frame + OS-fönster + aktiv flik

// Host-brygga (WASM → Chrome shell)
kb_host_sync();
host_paint();

// Historik + flikar
kb_back();
kb_forward();
kb_tab_open("kabootar://settings");
kb_tabs();
```

## Integration med Kabootar OS och värd-OS

Webbläsaren stödjer **flera OS-lager** via `kb_set_os_mode` / `kb_sync_platform`:

| Läge | URL-exempel |
|------|-------------|
| `kabootar` | `kabootar://vfs/apps/page.kml` |
| `host` | `file:///C:/apps/page.kml`, `host://./page.kml` |
| `auto` | Väljer scheme automatiskt |
| HTTP (native) | `http://localhost:8080/` |

```kabootar
platform_use("hybrid");
kb_sync_platform();              // hybrid → auto
kb_set_os_mode("host");          // Windows/Linux/macOS-filer
kb_navigate("file:///tmp/app.kml");
kb_set_os_mode("kabootar");
os_write("/apps/home.kml", "<div>Welcome</div>");
kb_navigate("kabootar://vfs/apps/home.kml");
kb_os_info();                    // mode, host_os, mounts
```

Montera värdmapp i Kabootar VFS för delad åtkomst:

```kabootar
os_mount("/host", "C:/Users/dev/projects");
kb_navigate("kabootar://vfs/host/app/page.kml");
```

Framtida steg: OS-fönster kopplas direkt till `kbrowser`-viewport (compositor).

## Plattformsmål

`kbrowser` ska fungera **överallt Kabootar körs** — inte bara som WASM-demo i Chrome. Sju målklasser:

| # | Plattform | Lager | Primär URL / I/O |
|---|-----------|-------|------------------|
| 1 | **kOS** | Kabootar-native | `kabootar://vfs/…`, `os_*`, compositor |
| 2 | **Windows** | Host + hybrid | `file:///`, native desktop shell |
| 3 | **Linux** | Host + hybrid | `file://`, X11/Wayland bridge |
| 4 | **macOS** | Host + hybrid | `file://`, AppKit bridge |
| 5 | **WASM** (desktop web) | Host (web) | `kabootar-shell.html`, canvas + `kb_host_sync()` |
| 6 | **Android** | Mobil host | WebView / PWA + touch; framtida Kabootar Shell-app |
| 7 | **iPhone / iOS** | Mobil host | WKWebView / PWA + touch + safe area; framtida Shell-app |

### Mobil (Android & iPhone)

Samma **`kb_*`-API** som desktop — anpassad **input**, **viewport** och **shell**, inte ett separat “mini-browser”-språk.

| Krav | Android | iPhone / iOS |
|------|---------|----------------|
| Motor | WASM i WebView / Chrome | WASM i WKWebView / Mobile Safari |
| Touch | `touchstart` / `touchmove` / `touchend` → kDOM hit-test | samma + scroll-rubberband policy |
| Viewport | `kb_viewport(w, h)`, DPR, orientering | + **safe area** (notch, home indicator) |
| Navigation | Tillbaka-gest, adressfält / flikar (mobil layout) | swipe-back, iOS safe-area padding |
| Distribution | PWA (`kabootar-shell.html`) → Play Store Shell | PWA → App Store Shell (WKWebView) |
| kOS / seamless | `os_seamless_*` — clipboard/handoff mot desktop | samma ([OS.md](OS.md)) |

Mobil implementeras under **[ROADMAP G7](ROADMAP.md)** och delar compositor med **G11**.

Principer (planerat — [ROADMAP.md — Våg G11](ROADMAP.md)):

1. **Samma Kabootar-API** — `kb_navigate`, `kb_mount`, `kb_render`, `kb_paint`, flikar/historik på **desktop och mobil**.
2. **Plattformsadapter** — `kb_sync_platform()` / `platform_use("kabootar"|"host"|"hybrid")` väljer scheme, fil-I/O och paint-backend.
3. **kOS som referens** — VFS-sidor och OS-fönster är canonical; host-OS speglar beteendet via mount (`os_mount("/host", …)`). **Utseende:** Windows-lik familiaritet (taskbar, Start, Explorer) med modern Kabootar-rendering — se [OS.md#desktop--utseende](OS.md#desktop--utseende) och [ROADMAP G12](ROADMAP.md).
4. **Små smoke per klass** — native (win/linux/mac), wasm, kos, **mobil (Android/iOS)**; inga megabundles i standard-`cargo test`.

Implementeringsordning (förslag):

```
lib/os + lib/kdom + lib/kv8  →  lib/kbrowser/*.kab  →  host-bindningar per OS  →  mobil touch/viewport (G7)  →  CI-smokes
```

Se även [PLATFORM.md](PLATFORM.md) (dual-layer) och [STDLIB.md](STDLIB.md) (plattformstabell).

**v2.50+:** WebAssembly, WebGL, WebRTC, DevTools, Extensions, PWA — se [BROWSER_V2.md](BROWSER_V2.md).

## Arkitektur

```
kb_navigate(url)
    → BrowserTab.history
    → parse KML / load document
kb_mount(DomNode)
    → active tab.document
kb_render()
    → KML HTML serializer → viewport string
```

Implementering: `src/runtime/kabootar_browser/mod.rs`

## Chrome-like shell

Öppna `kabootar-shell.html` efter WASM-build — Chrome-inspirerat skal som kör Kabootar compositor i iframe via `kb_run_ui()`.

```bash
wasm-pack build --target web --no-default-features --features docai,codai
# Öppna kabootar-shell.html i Chrome
```

## Host-lager

När appen körs i riktig Chrome, använd **lager 1**:

```kabootar
window.fetch("https://api.example.com/data");
document.querySelector("#app");
```

Se [PLATFORM.md](PLATFORM.md) för dual-layer-modellen.
