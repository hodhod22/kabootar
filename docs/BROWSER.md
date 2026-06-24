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
