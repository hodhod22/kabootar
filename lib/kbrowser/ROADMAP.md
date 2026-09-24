# kbrowser — roadmap

kbrowser är **webläsaren**, inte operativsystemet. Chrome/nav/flik skrivs i Kabootar under `lib/kbrowser/`. Rust = fönster, pixlar, input tills det tunnas.

Läsaren är **allmän**: sidor och appar i **vilket språk som helst** som targetar **Kv8 + kDOM + kstyle**. Inte en Kabootar-only-browser — Kabootar är ett gästspråk; motorn renderar kDOM, stylar med kstyle, kör script i Kv8.

kOS är separat: [lib/kos/ROADMAP.md](../kos/ROADMAP.md). **Bygg kOS först**; kbrowser fördjupas mot samma `kabootar://vfs`.

Docs: [README.md](README.md). kOS: [../kos/README.md](../kos/README.md). Web-API v2: [docs/BROWSER_V2.md](../../docs/BROWSER_V2.md).

## Layout

```
lib/kbrowser.kab          — import "kbrowser" → core
lib/kbrowser/
  core.kab, nav.kab, history.kab, session.kab, load_policy.kab
  bookmarks.kab, theme.kab
  desktop_chrome.kab, mobile_chrome.kab
```

| Import | Roll |
|--------|------|
| `kbrowser` / `kbrowser/core` | mount/render/paint-orchestration |
| `kbrowser/nav` | back/forward/tabs (Kab; inga `kb_back`-natives) |
| `kbrowser/history` | sessionshistorik |
| `kbrowser/session` | session persist → `/session/tabs` via `kos/vfs` |
| `kbrowser/mobile_chrome` | adressfält, tillbaka, flikar (mobil) |
| `kbrowser/desktop_chrome` | desktop-chrome |

---

## Regel

- Ny läsarlogik bara i `.kab` här (själva motorn).
- kOS-fönster/Start/Explorer hör hemma i `lib/kos/`.
- Samma `kb_*`-yta på kOS, Windows/Linux/macOS, WASM, Android, iPhone.
- Gästinnehåll = kDOM + kstyle + Kv8. Inget krav att sidan är skriven i Kabootar.

## Gäster (alla språk)

```
källspråk (Kabootar, JS/Kv8, andra som emitterar Kv8/kDOM)
        ↓
kDOM-träd + kstyle + Kv8-script
        ↓
kb_mount / kb_navigate / kb_paint
```

Samma pipeline på kOS-skrivbord och på host-OS. Chrome (flikar, historik, PWA) är språkagnostisk.

---

## Kärna (tidigare K4 / H6c)

| Fas | Innehåll | Status |
|-----|----------|--------|
| **KB-K1** | Tabs + VFS navigate + paint | ✅ subset (`k4_kbrowser_tabs_smoke`) |
| **KB-K2** | Chrome/nav i Kab; Rust `BrowserTab.history` bort | ✅ (`kbrowser/nav`; `kb_back`/`kb_tab_*` natives bort) |
| **KB-K3** | Tab/history-session i `.kab` | ✅ subset (`kbrowser/history`) |
| **KB-K4** | Load/paint via `kb_navigate` | ✅ subset |

**Nästa (kärna):**

- [x] Bookmarks + load_policy som enda lastväg (ingen Rust-URL-policy) — `core.kab`-delegationen var bruten (`bookmarksAdd`/`bookmarksRemove`/`bookmarksList` saknades → `Undefined variable`); core exponerar nu `bookmarks*` som delegerar till `bookmarks.kab`s `addBookmark`/`removeBookmark`/`listBookmarks`
- [x] Flikar/session persist i VFS (`/session/tabs`) via `kos/vfs` — `kbrowser/session` (`saveSession`/`loadSession`/`resumeSession`/`clearSession`/`sessionPersistOk`, JSON-roundtrip); `nav.kab` write-through på varje mutation + `bootNav` återställer (`sh27_session_persist_smoke`, `sh27_session_persist_in_kab`)
- [x] Delete-gate-prep: öppna → navigera → back/forward → stäng flik utan native chrome — hela rundan via `kbrowser/nav` + `history` + `session` (`kb_navigate` = bara load/paint); `sh27_delete_gate_smoke` + `sh27_delete_gate_in_kab`. `uiHostDeleteOk()` fortfarande `false` — host-chrome tas bort först när Kab-ytan faktiskt ersätter den.

**Host-inventering (`src/runtime/kabootar_browser/`) efter prep — vad som återstår innan `uiHostDeleteOk` kan flippas:**

| Rust-host äger | Kab-motsvarighet | Status |
|---|---|---|
| `tabs`/`active`/`next_id` i `BrowserInner` | `history.kab`/`nav.kab`-session + `/session/tabs` | ✅ bort — `BrowserInner.tab` = en render-slot; tabell/indexering Kab-ägd |
| `default_home_document` | `theme.kab::homePage` via `kb_set_home_provider` | ✅ — `home_fallback` anropar providern (live-resolve); Rust-DOM = bootstrap utan Kab |
| `title_from_url`/`tab.title` | `load_policy.kab::titleFromUrl` via `kb_set_title_provider`; `history.kab` spårar `tab["title"]` | ✅ — döda `tab.title`-fältet bort; fallback-h1 går via provider |
| `kbrowser`-handle per env | — | ✅ — `SHARED_BROWSER` thread-local: modul-env och main-env delar samma render-slot (lagade split: `kb_theme` i modul-env vs `kb_paint` i main-env) |
| `default_chrome_theme_css`/`set_theme_css` | `theme.kab::applyBrowserTheme` → `kb_theme(css)` | ✅ bridgat — Kab-tema når nu `inner.stylesheet`/paint |
| `stylesheet`-merge i `paint()` | `kstyle_*`/`__kstyle` | via `kb_theme`; kDOM-paint läser `__kstyle` direkt |
| `load_page`/`host_nav` (VFS/file/http I/O) | — | host-capability, stannar |
| `paint`/RenderEngine/`frame_buffer` | — | host-capability, stannar |
| viewport/safe_area/user_agent | `mobile_chrome`/`desktop_chrome` policynivå | host device-state, stannar |
| `os_mode`/`effective_mode` mirror | `load_policy.kab::effectiveMode` → `nav.navApplyMode` → `kb_set_os_mode` | ✅ Kab beslutar per-URL på produktvägen; Rust-mirror = fallback i `load_page` |
| `/session/tabs` endast in-memory VFS | `session_disk.kab` mountar `/session` → host-katalog (`os_mount`) | ✅ — writes streamar till äkta disk, överlever reboot (`mountSessionDisk`/`unmountSessionDisk`/`sessionDiskPersistOk`; `SHARED_OS` = samma env-split-fix som `SHARED_BROWSER`) |
| `click_at`/`touch_at`/input | — | host-capability, stannar |
| `run_kv8_script` | — | host-capability, stannar |

---

## Cross-platform (tidigare G11)

`kbrowser` ska vara förstaklass på kOS **och** varje host där motorn byggs.

| Mål | Renderingsväg | Smoke | Status |
|-----|---------------|-------|--------|
| **kOS** | VFS (`kabootar://`), compositor | `kbrowser_kos_smoke` | ✅ subset — via `kbrowser/core`+`nav` (navApplyMode auto väljer mode) |
| **Windows/Linux/macOS** | Native shell / pixels | `kbrowser_native_smoke` | ✅ subset — via `kbrowser/core` (`sync`/`info`/`paint`) |
| **WASM** | `kabootar-shell.html` + canvas | `kbrowser_wasm_smoke` | ✅ subset — via `kbrowser/core` |
| **Mobil** | viewport/safe-area/touch | `kbrowser_mobile_smoke`, `kbrowser_mobile_shell_smoke` | ✅ subset — via `kbrowser/mobile_chrome` |

Krav (landat subset):

- [x] `lib/kbrowser/` + aggregator; Rust som host-bindning
- [x] `kb_sync_platform()` → `{mode,layer,host_os,schemes}`
- [x] Enhetlig yta: `kb_mount` → `kb_render` → `kb_paint`
- [x] CI-smokes native / kos / wasm

**Nästa:** AppKit/X11/Wayland-bridge (thin host, ingen produktlogik i Rust).
Bridge-policyn är landad i Kab: `kbrowser/platform` mappar `host_os` →
`{shell, present, input, status}` (`win32`/`appkit`/`x11-wayland`/`webcanvas`/
`wkwebview`/`webview`/`kos-compositor`, alla ärligt `stub` utom kos). Device-CI
gaten `platformBridgeOk` är självanpassande — varje CI-host assertar sin egen
shell (`sh27_platform_bridge_in_kab`, `sh27_platform_bridge_smoke`).

---

## Mobil (tidigare G7)

Samma `kb_*` på Android och iPhone.

| Mål | Väg | Status |
|-----|-----|--------|
| Touch | `kb_touch_at` + hit-test | ✅ subset |
| Viewport | `kb_viewport(w, h, dpr?, orientation?)` | ✅ subset |
| iOS safe area | `kb_safe_area(…)` stub | ✅ subset |
| Mobil chrome | `kbrowser/mobile_chrome` | ✅ subset |
| PWA | SW + manifest ([BROWSER_V2](../../docs/BROWSER_V2.md)) | ✅ subset |
| Smokes | `kbrowser_mobile_smoke`, `kbrowser_mobile_shell_smoke` | ✅ subset |

**Nästa:** device-CI (WebView/WKWebView), Play/App Store-wrapper (host-skal, UI i Kab).
Bridge-tabellen täcker redan `ios`→`wkwebview` och `android`→`webview` som
ärliga `stub`-mål; `platformBridgeOk` failar om en framtida host_os inte
mappar — det är device-CI-förberedelsen.

---

## Rust → noll (läsar-delen av H6)

| Fas | Mål | Status |
|-----|-----|--------|
| **H6c** | Chrome = `.kab`; Rust = window/pixels/input | ✅ |
| **KB-H1** | Ingen ny `kb_*` produkt-API i Rust | ✅ gate `sh27_kb_h1_frozen_native_surface` — `mod.rs` registreringen fryst till den auditerade 28-namns-ytan (host-capability + provider-hooks); ny `kb_*` i Rust failar testet |
| **KB-H2** | Navigate/load-policy 100 % Kab | ✅ deepen — `nav.navApplyMode` sätter `kb_set_os_mode(effectiveMode(url, navModePref))` före varje `kb_navigate` (auto per-URL eller pin via `navSetMode`); `loadPlan(url)` buntar mode+kind+title+virtualHome (`sh27_load_policy_smoke`, `sh27_load_policy_in_kab`) |
| **KB-H3** | Markup→DOM i Kab | ✅ `kdom/markup.parseMarkup` (element/attr/text/comments/void/entiteter, bottom-up-append mot live-registret) installerad via `kb_set_document_provider` i `nav.kab` — navigerade sidor byggs utan Rust `parse_kml` (`sh27_markup_parse_in_kab`, `sh27_markup_parse_smoke`) |
| **KB-H4** | kv8-modul-parse i Kab | ✅ `kv8/module.kv8Sections` splittar `---kml---`/`---css---`/`---script---` i Kab (script hittas även utan css-markör — Rust-referensen lämnade det i kml-text); `kv8Page` returnerar `{doc, css, script}` via det utökade provider-kontraktet — inget nytt `kb_*` (KB-H1 orörd). Script-eval är fortfarande host (`kb_run_kv8`) tills `lib/kv8`-eval tar över produktvägen (`sh27_kv8_module_parse_in_kab`, `sh27_kv8_module_smoke`); kvar i Rust = fetch/paint/input/kv8-eval-capability |

---

## Beroenden

kDOM/Kv8/kss (Våg K2/G6–G10), layout/canvas (Våg C), **kOS VFS** (`kos/vfs`, `kos/async`).

## Checkpoint

Smokes: `examples/kbrowser_*.kab`, `examples/h6c_browser_chrome_smoke.kab`, `k4_kbrowser_tabs_smoke`.
