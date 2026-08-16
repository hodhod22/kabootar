# Negin - React-like UI Library for Kabootar

## Vision
Negin är ett modernt UI-bibliotek för Kabootar som bygger på Reacts styrkor men löser dess svagheter.

## Reacts Styrkor (att behålla)
- **Komponentbaserad arkitektur** - Återanvändbara, isolerade komponenter
- **Virtual DOM** - Effektiv uppdatering av UI
- **Unidirectional data flow** - Förutsägbar datahantering
- **Declarative syntax** - Beskriv vad, inte hur
- **Rich ecosystem** - Hooks, Context, Suspense
- **Strong community** - Många resurser och lösningar

## Reacts Svagheter (att lösa)
- **Learning curve** - Hooks kan vara förvirrande för nybörjare
- **Boilerplate** - Mycket kod för enkla saker
- **Performance overhead** - Virtual DOM kan vara långsamt för stora appar
- **State management complexity** - Redux/Context kan vara komplexa
- **Bundle size** - React är relativt stort
- **Re-renders** - Ofta onödiga omrenderingar
- **Prop drilling** - Svårt att dela data djupt i komponentträdet

## Negins Förbättringar
- **Simpler API** - Mindre boilerplate, mer intuitiv syntax
- **Optimerad rendering** - Smartare diffing, färre onödiga renders
- **Inbyggt state management** - Inget behov för externa bibliotek
- **Reaktiv data** - Automatisk uppdatering utan explicit setState
- **Signals** - Modern reaktivitet liknande SolidJS/Svelte
- **Smaller bundle** - Bättre tree-shaking, modulär design
- **Better TypeScript support** - Fullständig typsäkerhet
- **Faster development** - Snabbare iteration med hot reload

## Arkitektur

### Core Moduler
1. **Core** - createElement, render, virtual DOM
2. **Components** - Komponentsystem, lifecycle
3. **Hooks** - useState, useEffect, useMemo, useCallback
4. **Signals** - Reaktiv datahantering (nytt!)
5. **State** - Global state management (nytt!)
6. **Reconciler** - Diffing och patching
7. **Scheduler** - Prioriterad uppdatering
8. **Events** - Event system
9. **Context** - Context API
10. **Suspense** - Lazy loading och code splitting

## Implementeringsplan

### Fas 1: Core Foundation (High Priority)
- [x] Skapa roadmap
- [x] Analysera Reacts styrkor/svagheter
- [x] Designa Negin arkitektur och API
- [x] Skapa core modul (createElement, render)
- [x] Implementera komponentsystem med hooks
- [x] Implementera state management (bättre än React)
- [x] Implementera virtual DOM och diffing

### Fas 2: Advanced Features (Medium Priority)
- [x] Implementera reconciliation och scheduling
- [x] Implementera event system
- [x] Implementera context API
- [x] Implementera Suspense och lazy loading

### Fas 3: Production Ready (Low Priority)
- [x] Implementera server-side rendering
- [x] Skapa dokumentation och exempel
- [x] Performance optimeringar
- [x] Testing support
- [x] Hot reload support
- [x] DevTools integration
- [x] TypeScript definitions

## Filstruktur
```
lib/negin/
├── ROADMAP.md
├── core.kab          # Core funktioner
├── element.kab       # Element och VNode
├── component.kab     # Komponentsystem
├── hooks.kab         # React-like hooks
├── signals.kab       # Reaktiv data (nytt!)
├── state.kab         # Global state (nytt!)
├── reconciler.kab    # Diffing och patching
├── scheduler.kab     # Uppdateringsscheduling
├── events.kab        # Event system
├── context.kab       # Context API
├── suspense.kab      # Suspense och lazy loading
├── ssr.kab           # Server-side rendering
└── examples/
    ├── counter.kab
    ├── todo.kab
    └── app.kab
```

## API Design (förslag)

```kab
// Enkel komponent
pub fn Counter() {
    let (count, setCount) = useState(0)
    
    return createElement("div", {},
        createElement("button", { "onClick": fn() { setCount(count - 1) } }, "-"),
        createElement("span", {}, count),
        createElement("button", { "onClick": fn() { setCount(count + 1) } }, "+")
    )
}

// Med signals (nytt!)
pub fn CounterWithSignal() {
    let count = createSignal(0)
    
    return createElement("div", {},
        createElement("button", { "onClick": fn() { count.value = count.value - 1 } }, "-"),
        createElement("span", {}, count.value),
        createElement("button", { "onClick": fn() { count.value = count.value + 1 } }, "+")
    )
}

// Global state (nytt!)
pub fn createStore(initialState) {
    let store = createSignal(initialState)
    return { store, getState: fn() { store.value }, setState: fn(s) { store.value = s } }
}
```
