# kbrowser — roadmap

kbrowser är **webläsaren**, inte operativsystemet. Chrome/nav/flik skrivs i Kabootar under `lib/kbrowser/`. Rust = fönster, pixlar, input tills det tunnas.

Läsaren är **allmän**: sidor och appar i **vilket språk som helst** som targetar **Kv8 + kDOM + kstyle**. Inte en Kabootar-only-browser — Kabootar är ett gästspråk; motorn renderar kDOM, stylar med kstyle, kör script i Kv8.

kOS är separat: [lib/kos/ROADMAP.md](../kos/ROADMAP.md). **Bygg kOS först**; kbrowser fördjupas mot samma `kabootar://vfs`.

Docs: [README.md](README.md). kOS: [../kos/README.md](../kos/README.md). Web-API v2: [docs/BROWSER_V2.md](../../docs/BROWSER_V2.md).

## Layout

```
lib/kbrowser.kab          — import "kbrowser" → core
lib/kbrowser/
  core.kab, nav.kab, history.kab, load_policy.kab
  bookmarks.kab, theme.kab
  desktop_chrome.kab, mobile_chrome.kab
```

| Import | Roll |
|--------|------|
| `kbrowser` / `kbrowser/core` | mount/render/paint-orchestration |
| `kbrowser/nav` | back/forward/tabs (Kab; inga `kb_back`-natives) |
| `kbrowser/history` | sessionshistorik |
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

- [ ] Bookmarks + load_policy som enda lastväg (ingen Rust-URL-policy)
- [ ] Flikar/session persist i VFS (`/session/tabs`) via `kos/vfs`
- [ ] Delete-gate: öppna → navigera → back/forward → stäng flik utan native chrome

---

## Cross-platform (tidigare G11)

`kbrowser` ska vara förstaklass på kOS **och** varje host där motorn byggs.

| Mål | Renderingsväg | Smoke | Status |
|-----|---------------|-------|--------|
| **kOS** | VFS (`kabootar://`), compositor | `kbrowser_kos_smoke` | ✅ subset |
| **Windows/Linux/macOS** | Native shell / pixels | `kbrowser_native_smoke` | ✅ subset |
| **WASM** | `kabootar-shell.html` + canvas | `kbrowser_wasm_smoke` | ✅ subset |

Krav (landat subset):

- [x] `lib/kbrowser/` + aggregator; Rust som host-bindning
- [x] `kb_sync_platform()` → `{mode,layer,host_os,schemes}`
- [x] Enhetlig yta: `kb_mount` → `kb_render` → `kb_paint`
- [x] CI-smokes native / kos / wasm

**Nästa:** AppKit/X11/Wayland-bridge (thin host, ingen produktlogik i Rust).

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

---

## Rust → noll (läsar-delen av H6)

| Fas | Mål | Status |
|-----|-----|--------|
| **H6c** | Chrome = `.kab`; Rust = window/pixels/input | ✅ |
| **KB-H1** | Ingen ny `kb_*` produkt-API i Rust | pågående |
| **KB-H2** | Navigate/load-policy 100 % Kab | `load_policy` deepen |

---

## Beroenden

kDOM/Kv8/kss (Våg K2/G6–G10), layout/canvas (Våg C), **kOS VFS** (`kos/vfs`, `kos/async`).

## Checkpoint

Smokes: `examples/kbrowser_*.kab`, `examples/h6c_browser_chrome_smoke.kab`, `k4_kbrowser_tabs_smoke`.
