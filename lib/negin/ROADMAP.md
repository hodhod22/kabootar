# Negin — UI-lager för Kabootar

## Vision
Negin är Kabootars UI-lager. Det tar Reacts styrkor som är värda att behålla, men det är inte en React-klon i JavaScript. Eftersom Negin och Kabootar är samma ekosystem kan Negin angripa problem React måste leva med på grund av sin historik och JS-miljön.

## Reacts styrkor som är värda att behålla
- **Komponentbaserad arkitektur** — UI byggs av isolerade enheter med tydliga gränser
- **Deklarativ UI** — beskriv vad som ska synas, inte hur DOM:en muteras steg för steg
- **Composability** — små ytor sätts ihop till större utan att läcka implementationsdetaljer
- **Återanvändbara komponenter** — samma byggsten i flera skärmar och appar
- **Tydlig data → UI-modell** — given state ska UI:t vara en funktion av den datan
- **Ekosystemtänk** — hooks, context, suspense, routing, forms som samspelande lager, inte ett enda monster-API
- **Server/client-modeller** — möjlighet att rendera på server och hydrera/köra på klient, samma komponentmodell

Virtual DOM och “React-community” är *medel*, inte mål. Negin behåller den deklarativa komponentmodellen; diffing kan vara fiber, signals eller kdom-patch så länge data → UI håller.

## Sådant Negin kan försöka göra bättre
React bär JS-historik: mutable DOM, closures, dependency arrays, separat bundler, separat SSR-runtime. Negin kör i Kabootar (GC default, sql/http/kdom, self-host bytecode) och kan därför sikta på:

- **Onödig rendering** — signal-tracking och keyed fiber så bara beroende noder uppdateras; ingen “render hela trädet för att en räknare ändrades”
- **Komplex state management** — inbyggd store/signals i språket, inte Redux-lager ovanpå hooks
- **Dependency-array-problematik** — effekter spårar beroenden automatiskt (`seen[eid]`), inga `[count, fn]`-fällor
- **Hydration/SSR-komplexitet** — samma runtime och kdom/host-adapter; mindre “två världar” (Node vs browser)
- **Bundle/runtime-overhead** — `.kab` → `.kbc`, modulär import, ingen React-reconciler som separat JS-payload
- **Async state** — suspense/sql/http i samma språk; loading/error som vanliga värden, inte ett extra React-ekosystem
- **Memory lifetime** — GC default för UI; `@manual` bara där Kabootar redan har det (kOS/net/buffers), inte i komponentkärnan
- **GC-relaterade kostnader** — färre temporära VNode-allocs per tick; O(1) subscriber-index; frame-budget via runtime när det behövs
- **Gränsen UI-state vs applikationsstate** — UI-signaler för view; sql/db/http som källor för app-state; ingen “allt är useState”

## Hur det landar i koden (Fas 6+)
- Fiber: `negin/fiber` keyed walk så barn återanvänds på `key`, inte index
- Signals: `negin/signals` med `seen["e"+eid]` istället för linjär subscriber-scan
- Error: `negin/error` som giltig Kabootar (inga JS-`useState`-destructures)
- Gate: `examples/negin_fas6_smoke.kab`

## Arkitektur

Lager, inte en React-kopia. UI är deklarativa komponenter; app-data kommer från Kabootar.

| Lager | Roll | Behållen React-idé | Negin-skillnad |
|-------|------|--------------------|----------------|
| **element / core** | `createElement`, render | deklarativt träd | host-agnostisk (kdom + host-DOM) |
| **component** | komposition, återanvändning | komponenter | GC-livstid, ingen class-component-historik |
| **fiber / reconciler** | keyed patch | composable träd | keyed walk, inte index-only diff |
| **signals** | data → UI | unikällsflöde | auto-track, inga dependency arrays |
| **hooks** | lokal UI-state | bekant yta | sekundär; signals är default |
| **state** | app-store | ekosystem | inte “allt i useState”; sql/http är källor |
| **host-adapter** | sql / http / kdom | server+client | samma språk, ingen separat Node-runtime |
| **ssr / hydration** | server/client-modell | samma komponenter | samma VM/bytecode |
| **error** | felgränser | composability | giltig Kab-syntax, log + recover |

**UI-state** = signaler och lokal view (öppen panel, hover, input-caret).  
**Applikationsstate** = rader från `sql`/`db`, svar från `http_*`, kOS-session. Negin ska inte tvinga app-state genom hook-listan.

## Implementeringsplan

Fas 1–5 är historiska leveranser (core → Kabootar-integration → signals/SSR-yta). De är **subset**: många filer finns, men JS-syntax och “automatic everything” är inte produktgaranti. Fas 6 är den första som styrs av listan ovan med en körbar smoke.

### Fas 1–3 — Foundation (subset)
- [x] Core: createElement, render, komponenter, helpers
- [x] Reconciler, scheduler, events, context, suspense (API-yta)
- [x] SSR/docs/testing/hotreload/devtools-moduler (finns; parity varierar)

### Fas 4 — Kabootar-integration (subset)
- [x] Host-adapter: host-DOM + kdom
- [x] Render host-agnostisk
- [x] sql / db / http_route / http_request / kml / kdom_render i samma UI-lager

### Fas 5 — Signals och runtime-yta (subset)
- [x] Signals som reaktiv default (`negin/signals`, `negin/reactive-core`)
- [x] Auto-batch / scheduler-kö (yta)
- [x] Inbyggd state-modul utan externt Redux
- [x] SSR/hydration/dev-server/CLI/profiler som Kab-moduler

### Fas 6 — Refinement mot Reacts kostnader (subset, smoke)
- [x] Keyed fiber-walk (`negin/fiber`) — mindre onödig child-remount
- [x] O(1) signal-tracking (`seen["e"+eid]`) — inte linjär subscriber-scan
- [x] Giltig error-modul (ingen JS-destructure / `!==` / rekursiv `typeof`)
- [x] Gate: `examples/negin_fas6_smoke.kab`
- [x] Roadmap: behållvärda React-styrkor vs Kabootar-vinster

### Fas 7 — Göra “bättre än React” mätbart (nästa)
Arbete styrs av listan *Sådant Negin kan försöka göra bättre*. En punkt i taget, med smoke.

- [ ] **Onödig rendering** — mät: signal-set uppdaterar bara beroende fiber-noder; ingen fullträds-reconcile i smoke
- [ ] **State-gräns** — dokumenterad + API: `signal*` = UI; `sql`/`http_*` = app; förbjud “store i varje komponent”
- [ ] **Inga dependency arrays** — `useEffect(fn, deps)` antingen auto-track eller avvecklas till `createEffect`
- [ ] **Async state** — en `Result`/`loading`/`error`-yta driven av sql/http, inte separat Promise-reconciler
- [ ] **Hydration/SSR** — en host-path: samma `render` mot kdom och host-DOM; smoke som roundtrippar markup
- [ ] **Bundle/runtime** — Negin-core self-host-kompilerar; ingen dold JS-runtime i UI-hotpath
- [ ] **Memory / GC** — inga extra VNode-allocs per identisk keyed child; valfri `gc_frame_stats` i UI-tick-smoke
- [ ] **Giltig Kab överallt** — `reconciler`/`hooks` utan `if cond) {`, `fn()`-i-objekt och `let (a, b)`

## Filstruktur
```
lib/negin/
├── ROADMAP.md
├── README.md
├── core.kab / element.kab / component.kab
├── fiber.kab            # keyed walk (Fas 6)
├── reconciler.kab       # patch mot fiber
├── signals.kab          # createSignal / signalGet / signalSet / createEffect
├── hooks.kab            # UI-lokal state (sekundär)
├── state.kab            # app-store (inte UI-default)
├── host-adapter.kab     # sql, http, kdom
├── error.kab
├── scheduler.kab / events.kab / context.kab / suspense.kab / ssr.kab
└── examples/            # counter, todo, app, …
examples/negin_fas6_smoke.kab
```

## API (Kabootar, inte JS)

Signals + keyed fiber är den yta Fas 6 faktiskt kör. Inga tuple-destructures, inga anonyma `fn()` i objektliteral.

```kab
import "negin/fiber"
import "negin/signals"
import "negin/error"

let count = createSignal(0)

fn onCount() {
    signalGet(count)
}

fn bump() {
    signalSet(count, signalGet(count) + 1)
}

createEffect(onCount)

let prev = [{ "key": "a", "domNode": 1 }, { "key": "b", "domNode": 2 }]
let next = [{ "key": "b" }, { "key": "c" }]
let tree = reconcileKeyed(prev, next)

let err = createError("fail", "UI")
if isError(err) {
    logError(err, null)
}
```

App-state stannar utanför signalen:

```kab
import "negin/host-adapter"

let result = sql("SELECT id, name FROM users", [])
let rows = result["rows"]
```
