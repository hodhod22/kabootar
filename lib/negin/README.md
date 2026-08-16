# Negin - React-like UI Library for Kabootar

Negin är ett modernt UI-bibliotek för Kabootar som bygger på Reacts styrkor men löser dess svagheter.

## Installation

```kab
import "negin/core"
import "negin/hooks"
import "negin/signals"
```

## Grundläggande Användning

### Enkel Komponent

```kab
import "negin/core"
import "negin/hooks"

pub fn Counter() {
    let (count, setCount) = useState(0)
    
    return createElement("div", {},
        createElement("button", { "onClick": fn() { setCount(count - 1) } }, "-"),
        createElement("span", {}, count),
        createElement("button", { "onClick": fn() { setCount(count + 1) } }, "+")
    )
}

let container = { "kind": "Container", "children": [] }
render(createElement(Counter, {}), container)
```

### Med Signals (Reaktiv Data)

Signals är en förbättring jämfört med Reacts useState - automatisk uppdatering utan explicit setState.

```kab
import "negin/core"
import "negin/signals"

pub fn CounterWithSignal() {
    let count = createSignal(0)
    
    return createElement("div", {},
        createElement("button", { "onClick": fn() { count.set(count.get() - 1) } }, "-"),
        createElement("span", {}, count.get()),
        createElement("button", { "onClick": fn() { count.set(count.get() + 1) } }, "+")
    )
}
```

## Hooks

### useState

```kab
pub fn Counter() {
    let (count, setCount) = useState(0)
    return createElement("div", {}, count)
}
```

### useEffect

```kab
pub fn DataFetcher() {
    let (data, setData) = useState(null)
    
    useEffect(fn() {
        fetchData().then(fn(result) { setData(result) })
    }, [])
    
    return createElement("div", {}, data)
}
```

### useMemo

```kab
pub fn ExpensiveComponent(props) {
    let expensiveValue = useMemo(fn() {
        return computeExpensiveValue(props)
    }, [props.id])
    
    return createElement("div", {}, expensiveValue)
}
```

### useContext

```kab
let ThemeContext = createContext("light")

pub fn ThemedComponent() {
    let theme = useContext(ThemeContext)
    return createElement("div", { "className": theme }, "Content")
}
```

## Signals (Reaktiv Data)

Signals är en av Negins största förbättringar jämfört med React.

```kab
// Skapa en signal
let count = createSignal(0)

// Läs värdet
console.log(count.get()) // 0

// Uppdatera värdet (automatisk uppdatering av beroende komponenter)
count.set(1)

// Computed values (automatisk uppdatering)
let doubled = createComputed(fn() { return count.get() * 2 })

// Effects (kör när beroenden ändras)
createEffect(fn() {
    console.log("Count changed:", count.get())
})
```

## Global State

Negin har inbyggt state management - inget behov för Redux.

```kab
let store = createStore({ count: 0 })

let actions = {
    "increment": fn(state) { return { "count": state["count"] + 1 } },
    "decrement": fn(state) { return { "count": state["count"] - 1 } }
}

let storeWithActions = createStoreWithActions({ count: 0 }, actions)

// Använd i komponent
pub fn Counter() {
    let store = useStore({ count: 0 })
    return createElement("div", {}, store.getState()["count"])
}
```

## Context API

```kab
let UserContext = createContext(null)

pub fn App() {
    let user = { "name": "Alice", "email": "alice@example.com" }
    
    return createElement(Provider(UserContext, user),
        createElement(UserProfile, {})
    )
}

pub fn UserProfile() {
    let user = useContext(UserContext)
    return createElement("div", {}, user["name"])
}
```

## Suspense och Lazy Loading

```kab
let LazyComponent = lazy(fn() {
    return import("./Component.kab")
})

pub fn App() {
    return createElement(Suspense(createElement("div", {}, "Loading..."),
        createElement(LazyComponent, {})
    ))
}
```

## Server-Side Rendering

```kab
import "negin/ssr"

let html = renderToString(createElement(App, {}))
```

## Förbättringar jämfört med React

### 1. Signals - Enklare State Management
- React: `useState` + explicit `setState`
- Negin: Signals med automatisk uppdatering

### 2. Mindre Boilerplate
- React: Många hooks för enkla saker
- Negin: Mer intuitivt API

### 3. Bättre Performance
- React: Onödiga re-renders
- Negin: Smartare diffing, signals för granulära uppdateringar

### 4. Inbyggt State Management
- React: Behöver Redux/Context
- Negin: `createStore` inbyggt

### 5. Reaktiv Data
- React: Manuellt hantera beroenden
- Negin: Automatisk dependency tracking med signals

## API Reference

### Core
- `createElement(type, props, children)` - Skapa VNode
- `render(element, container)` - Rendera till container
- `createRoot(container)` - Skapa root

### Hooks
- `useState(initialValue)` - State hook
- `useEffect(effect, deps)` - Effect hook
- `useMemo(factory, deps)` - Memoization
- `useCallback(callback, deps)` - Callback memoization
- `useRef(initialValue)` - Ref hook
- `useContext(context)` - Context hook
- `useReducer(reducer, initialState)` - Reducer hook

### Signals
- `createSignal(initialValue)` - Skapa signal
- `createComputed(computation)` - Skapa computed value
- `createEffect(effectFn)` - Skapa effect

### State
- `createStore(initialState)` - Skapa global store
- `createStoreWithActions(initialState, actions)` - Skapa store med actions

## Exempel

Se `examples/` mappen för fler exempel:
- `counter.kab` - Enkel counter
- `todo.kab` - Todo app
- `app.kab` - Fullständig app

## Licens

MIT
