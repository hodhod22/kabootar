# Negin - Kabootar UI Layer

Negin är Kabootars UI-lager med GC + signals + sql/http/kdom integration. Negin är inte en isolerad React-klon - det är ett UI-lager som är djupt integrerat med Kabootars fullstack-ekosystem.

## Arkitektur

Negin är designat för Kabootar:
- **GC-läge (default)** för UI/signals/hooks/reconciler
- **Ingen @manual i kärnan** (endast för kOS/netstack/buffertar)
- **Språkets inbyggda yor** (sql, db, http, kdom)
- **Dubbel DOM-stöd** (host-DOM för WASM, kdom för Kabootar)
- **Signals + GC** för reaktivitet
- **Fas 6:** keyed fiber-walk (`negin/fiber`), O(1) signal-tracking, giltig error-modul. Smoke: `examples/negin_fas6_smoke.kab`.

## Installation

```kab
import "negin/core"
import "negin/reactive-core"
import "negin/host-adapter"
```

## Kabootar Integration

### Database

```kab
import "negin/host-adapter"

// Använd Kabootars sql() direkt
let result = sql("SELECT * FROM users WHERE id = ?", [1])
let rows = result["rows"]

// Använd Kabootars db() för connection
let db = db()
```

### HTTP

```kab
import "negin/host-adapter"

// HTTP routes
http_route("GET", "/api/users", fn(req) {
    let users = sql("SELECT * FROM users", [])
    return { "status": 200, "body": users["rows"] }
})

// HTTP requests
let response = http_request("https://api.example.com/data", { "method": "GET" })
```

### kdom (Kabootar DOM)

```kab
import "negin/host-adapter"

// Render till kdom
let kdomContainer = { "kind": "KdomContainer" }
render(createElement(App, {}), kdomContainer)

// Skapa kdom-element
let element = kml(" div ", { "className": "container" }, "Content")
kdom_render(element, container)
```

### Host-Agnostisk Rendering

Negin stöder både host-DOM (WASM) och kdom (Kabootar):

```kab
import "negin/host-adapter"

// Render till host-DOM (WASM)
let domContainer = document.getElementById("app")
render(createElement(App, {}), domContainer)

// Render till kdom (Kabootar)
let kdomContainer = { "kind": "KdomContainer" }
render(createElement(App, {}), kdomContainer)
```

## Fullstack Exempel med Kabootar

### Todo App med Database + HTTP + UI

```kab
import "negin/core"
import "negin/reactive-core"
import "negin/host-adapter"

pub fn TodoApp() {
    let todos = signal([])
    let newTodo = signal("")
    
    // Load todos from database
    useEffect(fn() {
        let result = sql("SELECT * FROM todos", [])
        todos["set"](result["rows"])
    }, [])
    
    let addTodo = fn() {
        if newTodo["get"]() != "") {
            let todo = newTodo["get"]()
            sql("INSERT INTO todos (text) VALUES (?)", [todo])
            
            let currentTodos = todos["get"]()
            todos["set"](push(currentTodos, { "id": len(currentTodos) + 1, "text": todo }))
            newTodo["set"]("")
        }
    }
    
    return createElement("div", { "className": "todo-app" },
        createElement("h1", {}, "Todo App"),
        createElement("input", {
            "value": newTodo["get"](),
            "onChange": fn(e) { newTodo["set"](e["target"]["value"]) }
        }),
        createElement("button", { "onClick": addTodo }, "Add"),
        createElement("ul", {}, todos["get"]())
    )
}

// HTTP routes
http_route("GET", "/api/todos", fn(req) {
    let todos = sql("SELECT * FROM todos", [])
    return { "status": 200, "body": todos["rows"] }
})

http_route("POST", "/api/todos", fn(req) {
    let text = req["body"]["text"]
    sql("INSERT INTO todos (text) VALUES (?)", [text])
    return { "status": 201, "body": { "success": true } }
})

// Render to kdom
let kdomContainer = { "kind": "KdomContainer" }
render(createElement(TodoApp, {}), kdomContainer)
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
