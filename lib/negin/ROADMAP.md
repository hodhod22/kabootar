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
- Host ABI: `negin/host` — fyra profiler, caps, samma createNode/setProp/insert-kärna
- Commit: `negin/commit` — op-lista (SET_TEXT, MOVE, …) sedan host apply (fast/compat)
- Static: `negin/static-tree` — UI utan signaler skapas en gång
- Error: `negin/error` som giltig Kabootar (inga JS-`useState`-destructures)
- Gate: `examples/negin_fas6_smoke.kab`, `examples/negin_fas7_smoke.kab`

## Arkitektur

Lager, inte en React-kopia. UI är deklarativa komponenter; app-data kommer från Kabootar.

| Lager | Roll | Behållen React-idé | Negin-skillnad |
|-------|------|--------------------|----------------|
| **element / core** | `createElement`, render | deklarativt träd | samma komponenter på fyra hostar |
| **component** | komposition, återanvändning | komponenter | GC-livstid, ingen class-component-historik |
| **fiber / signals** | keyed walk + auto-track | composable träd | inte “React-fast”; diffar en gång |
| **commit** | fiber → op-lista | patch | SET_TEXT / SET_ATTR / INSERT / REMOVE / MOVE |
| **host ABI** | createNode/setProp/insert/… | host-render | kDOM, KV8, browser-DOM, kOS-browser |
| **hooks / state** | lokal UI vs app-data | ekosystem | sql/http är källor; inte allt i useState |
| **ssr / hydration** | samma komponenter | server/client | capability `supportsHydration` |
| **error** | felgränser | composability | giltig Kab-syntax, log + recover |

Negin är **host-agnostiskt**. Kärnan ska inte veta om den kör mot kDOM, KV8, vanlig DOM eller Kabootars egen browser. En komponent, fyra host-situationer:

```
                    Negin
                      │
              Core / Signals / Fiber
                      │
               Commit Operations
                      │
                Host Adapter ABI
          ┌───────────┼───────────┐
          │           │           │
        kDOM        KV8       Browser DOM
          │           │           │
       kOS/browser   KV8      Chrome/Firefox/...
```

- **Fast path:** kDOM + kOS/browser — `supportsBatchCommit`, kompakt commit.
- **Compat path:** vanlig DOM + KV8 (tills KV8 får native batch/move) — samma ops, DOM-liknande apply.
- **KV8** är en förstaklassig host-profil, inte “browser-kompatibilitet”.
- **Capabilities, inte browser detection:** `supportsBatchCommit`, `supportsNativeEvents`, `supportsStaticNodes`, `supportsMoveNode`, `supportsHydration`.

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

### Fas 7 — Host-agnostisk runtime (ABI + commit + static)
Negin ska inte optimeras för en miljö. Samma API/komponentmodell över kDOM, KV8, vanlig DOM och kOS/browser. Inte “gör Negin React-fast”.

- [x] **Host Adapter ABI** — `createNode` / `setProp` / `removeProp` / `insert` / `remove` / `text` / `event` / `batch` (`negin/host`)
- [x] **Host-oberoende commit-ops** — fiber keyed diff → op-lista → host apply; fast vs compat (`negin/commit`)
- [x] **Static subtree hoisting** — statiskt UI skapas en gång och återanvänds (`negin/static-tree`)
- [x] **Capability detection** — `hostHasCap`, inte `if Chrome` / `if Kabootar`
- [x] **KV8 förstaklassig host** — egen profil (`kv8`), inte browser-fallback
- [x] **Event som host-feature** — native vs delegation
- [x] **List-primitive** — synligt fönster utan DOM-tricks (`negin/list`)
- [x] Gate: `examples/negin_fas7_smoke.kab`

Nästa (samma ABI, djupare hosts):
- [ ] kdom-host / kv8-host / browser-dom-host / kos-browser-host mot riktiga noder (inte bara in-memory ABI)
- [ ] KV8-optimering av objekt/events utan att röra browser-DOM-profilen
- [ ] Wire Fas 6 fiber/signals → commit-ops i `render`

### Fas 8 — Mätbara vinster (efter ABI)
Samma host-oberoende kärna. En punkt i taget, med smoke.

- [ ] **Onödig rendering** — signal-set uppdaterar bara beroende fiber-noder
- [ ] **State-gräns** — `signal*` = UI; `sql`/`http_*` = app
- [ ] **Inga dependency arrays** — `useEffect` auto-track eller `createEffect`
- [ ] **Async state** — `Result`/`loading`/`error` från sql/http
- [ ] **Hydration/SSR** — `supportsHydration` på host; smoke som roundtrippar markup
- [ ] **Bundle/runtime** — Negin-core self-host-kompilerar
- [ ] **Memory / GC** — inga extra VNode-allocs per identisk keyed child
- [ ] **Giltig Kab överallt** — `reconciler`/`hooks` utan `if cond) {`, `fn()`-i-objekt och `let (a, b)`

## Filstruktur
```
lib/negin/
├── ROADMAP.md
├── README.md
├── core.kab / element.kab / component.kab
├── fiber.kab            # keyed walk (Fas 6)
├── signals.kab          # createSignal / signalGet / signalSet / createEffect
├── host.kab             # Host Adapter ABI + caps + 4 profiler (Fas 7)
├── commit.kab           # SET_TEXT / SET_ATTR / INSERT / REMOVE / MOVE
├── static-tree.kab      # static subtree hoisting
├── list.kab             # host-oberoende list window
├── host-adapter.kab     # sql / http / kml (Kabootar-primitives)
├── error.kab
└── examples/            # counter, todo, app, …
examples/negin_fas6_smoke.kab
examples/negin_fas7_smoke.kab
```

## API (Kabootar, inte JS)

Signals + keyed fiber (Fas 6) och Host ABI + commit-ops (Fas 7) är den yta som faktiskt kör. Inga tuple-destructures, inga anonyma `fn()` i objektliteral.

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

Host ABI (Fas 7) — samma ops på kDOM, KV8, browser och kOS:

```kab
import "negin/host"
import "negin/commit"
import "negin/static-tree"

let host = createHost("kdom")
let node = hostCreateNode(host, "p")
commitClear()
commitPush(opSetText(node, "hi"))
commitPending(host)

let tree = { "type": "h1", "hoistKey": "title", "text": "Negin", "children": [] }
let a = hoistStatic(host, tree)
let b = hoistStatic(host, tree)
```
