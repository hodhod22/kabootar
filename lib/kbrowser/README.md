# kbrowser

kbrowser är **webläsaren** (inte operativsystemet). Den renderar **kDOM**, stylar med **kstyle** och kör script i **Kv8**, oberoende av host-Chrome.

Motorn **skrivs** i Kabootar (`lib/kbrowser/`). **Innehåll** är språkagnostiskt — valfritt språk som targetar Kv8 + kDOM + kstyle. Kabootar är ett gästspråk bland flera.

Plan: [ROADMAP.md](ROADMAP.md). Framtida web-API (PWA, WebGL, …): [BROWSER_V2.md](../../docs/BROWSER_V2.md). OS: [../kos/README.md](../kos/README.md).

## Kontrakt

```
Källspråk  →  kDOM + kstyle + Kv8
                 ↓
         kb_navigate / kb_mount / kb_paint
                 ↓
         kOS-fönster  eller  host (native / WASM / mobil)
```

Host-Chrome: `document` / `window`. kbrowser: `kb_*` + `import "kbrowser/…"`.

## Mot Chrome

| Chrome (host) | kbrowser |
|---------------|----------|
| `window.location.href` | `kb_location()` |
| `history.back()` / `forward()` | `import "kbrowser/nav"` → `navBack()` / `navForward()` |
| Ny flik | `navOpenTab(url)` / `navTabs()` |
| `document.body` | `kb_mount(kdom_node)` + `kb_render()` |
| `navigator.userAgent` | `kb_user_agent()` |

## Moduler

```kabootar
import "kbrowser"                 // core
import "kbrowser/core"
import "kbrowser/nav"             // back, forward, tabs (Kab — inga kb_back-natives)
import "kbrowser/history"         // session
import "kbrowser/load_policy"
import "kbrowser/bookmarks"
import "kbrowser/theme"
import "kbrowser/desktop_chrome"
import "kbrowser/mobile_chrome"   // adressfält, tillbaka, flikar
```

## API

```kabootar
os_mkdir("/apps")
os_write("/apps/page.kml", "<html><body><h1>App</h1></body></html>")
kb_navigate("kabootar://vfs/apps/page.kml")

kb_mount(page)
kb_viewport(1280, 720)
kb_theme("h1 { color: #8ab4f8; }")
let frame = kb_paint()      // { html, text, layers, nodes, width, height }
let scene = kb_composite()

kb_host_sync()
host_paint()

import "kbrowser/nav"
navBack()
navForward()
navOpenTab("kabootar://settings")
navTabs()
```

Mobil:

```kabootar
kb_viewport(390, 844, 3, "portrait")
kb_safe_area(47, 0, 34, 0)
kb_touch_at(x, y, "start")
import "kbrowser/mobile_chrome"
```

## OS-lägen

`kb_set_os_mode` / `kb_sync_platform`:

| Läge | URL |
|------|-----|
| `kabootar` | `kabootar://vfs/apps/page.kml` |
| `host` | `file:///…`, `host://./page.kml` |
| `auto` | scheme väljs |
| HTTP | `http://localhost:8080/` |

```kabootar
platform_use("hybrid")
kb_sync_platform()
kb_set_os_mode("host")
kb_navigate("file:///tmp/app.kml")
kb_set_os_mode("kabootar")
kb_navigate("kabootar://vfs/apps/home.kml")
kb_os_info()
```

Mount av host-katalog via kOS: `os_mount("/host", "C:/…")` sedan `kabootar://vfs/host/…`.

## Plattformar

Samma `kb_*` överallt.

| Plattform | Lager | I/O |
|-----------|-------|-----|
| **kOS** | native VFS + compositor | `kabootar://vfs` |
| **Windows / Linux / macOS** | host + hybrid | `file://` |
| **WASM** | `kabootar-shell.html` + canvas | `kb_host_sync()` |
| **Android** | WebView / PWA + touch | samma API |
| **iPhone** | WKWebView / PWA + safe area | samma API |

`kb_sync_platform()` → `{mode, layer, host_os, schemes}`. Kedja: `kb_mount` → `kb_render` → `kb_paint`.

WASM-skal:

```bash
wasm-pack build --target web --no-default-features --features docai,codai
# öppna kabootar-shell.html
```

## Tester

`examples/kbrowser_native_smoke.kab`, `kbrowser_kos_smoke.kab`, `kbrowser_wasm_smoke.kab`, `kbrowser_mobile_smoke.kab`, `h6c_browser_chrome_smoke.kab`.
