# Kabootar — dubbel plattform

Kabootar bygger på **två separata lager**. Samma språk, två världar — du väljer (eller blandar) per app.

## Arkitektur

```
┌─────────────────────────────────────────────────────────────────┐
│                     Kabootar-källkod                             │
└────────────────────────────┬────────────────────────────────────┘
                             │
              ┌──────────────▼──────────────┐
              │   platform_layer()           │
              │   "host" | "kabootar" | "hybrid" │
              └──────────────┬──────────────┘
                             │
         ┌───────────────────┴───────────────────┐
         │                                       │
┌────────▼─────────┐                 ┌──────────▼──────────┐
│  LAGER 1: HOST   │                 │  LAGER 2: KABOOTAR    │
│  (befintlig värld)│                 │  (egen stack)         │
├──────────────────┤                 ├───────────────────────┤
│ OS: Windows/Linux│                 │ OS: kabootar-kernel   │
│     macOS, WASM  │                 │     VFS, processer,   │
│ DOM: document    │                 │     fönsterhanterare  │
│      window      │                 │ DOM: kdom + KML       │
│      navigator   │                 │ Browser: kbrowser     │
│ Browser: Chrome  │                 │     flikar, history   │
│     Safari, etc. │                 │     viewport          │
└──────────────────┘                 └───────────────────────┘
```

## Lager 1 — Host (befintlig OS/DOM/webbläsare)

Använd när Kabootar körs **i** en riktig miljö (WASM i Chrome, native CLI mot riktigt filsystem).

| API | Beskrivning |
|-----|-------------|
| `document` | Chrome-lik DOM: `querySelector`, `createElement`, `getElementById` |
| `window` | `location`, `innerWidth`, `fetch` |
| `navigator` | `userAgent`, `platform`, `language` |
| `host_layer` | `"host"` |

```kabootar
println(navigator.userAgent);
let btn = document.createElement("button");
```

På WASM mappas detta mot värdwebbläsaren (Chrome m.fl.) när bindningar är aktiva.

## Lager 2 — Kabootar-native

Egen stack som **speglar** host-API:erna men är helt skriven i/ för Kabootar — ingen webbläsare krävs.

| API | Beskrivning |
|-----|-------------|
| `os` | Kernel, VFS, `os_spawn`, `os_window_create` |
| `kdom` / `kml` | Egen DOM + KML-markup |
| `kbrowser` | Chrome-inspirerad flikar, navigation, rendering |
| `kdom_listen` | Event-lyssnare på träd (muterar på plats) |
| `kabootar shell` | Native desktop-fönster med pixel-compositor |
| `kdom_layer` | `"kabootar"` |

```kabootar
os_window_create("Kabootar App", 1280, 720);
let ui = kml("<div class=\"app\"><h1>Hej</h1></div>");
kb_mount(ui);
println(kb_render());
```

## Plattforms-API

```kabootar
platform_info();           // objekt med båda lager
platform_layer();          // "hybrid" (standard)
platform_use("kabootar");  // föredra native stack
platform_use("host");      // föredra värd-stack
platform_use("hybrid");    // båda tillgängliga
```

## När ska jag använda vilket lager?

| Scenario | Lager |
|----------|-------|
| Webbapp i Chrome/Safari | Host (`document`, `window`) |
| Native desktop utan browser | Kabootar (`kdom`, `kbrowser`, `os`) |
| Server / SSR | Kabootar (`kdom_render`, `sql`, `http_*`) |
| Fullstack samma kodbas | Hybrid — UI i `kdom`, deploy WASM med host |

## Implementering

| Modul | Sökväg |
|-------|--------|
| Platform | `src/runtime/platform/mod.rs` |
| Host DOM | `src/runtime/browser_dom.rs` |
| Kabootar DOM | `src/runtime/kabootar_dom.rs` |
| Kabootar Browser | `src/runtime/kabootar_browser/mod.rs` |
| Kabootar OS | `src/runtime/os/` |

Se även [BROWSER.md](BROWSER.md), [OS.md](OS.md), [KML.md](KML.md), [RUNTIME.md](RUNTIME.md).
