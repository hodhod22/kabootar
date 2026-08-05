# App (`import "app"`)

Kab-first **app shell** MVP for mobile/desktop product apps — navigation, lifecycle, UI widgets, offline cache, i18n, and host stubs for sensors/share.

## Quick start

```kab
import "app"

let app = createApp("demo", { "title": "Demo" })
app = start(app)

let nav = createStack("home")
nav = pushRoute(nav, "details", { "id": 1 })

let catalog = createCatalog("en", { "hello": "Hello {name}" })
catalog = addLocale(catalog, "sv", { "hello": "Hej {name}" })
catalog = setLocale(catalog, "sv")
t(catalog, "hello", { "name": "Kab" })
```

## Modules

| Path | Role |
|------|------|
| `app/ui` | Screen / panel / button / label / textfield + layout |
| `app/nav` | Stack (`pushRoute`/`popRoute`) + tabs |
| `app/lifecycle` | cold → foreground / background / terminate |
| `app/offline` | local cache + `pwa_*` facade |
| `app/i18n` | string tables + `{var}` substitution |
| `app/sensors` | geolocation / camera / mic / motion **stubs** |
| `app/share` | share sheet + URL schemes **stubs** |

Phone sensors (`app/sensors`) ≠ edge IoT (`import "iot"`).

## Files

- `lib/app.kab`, `lib/app/*.kab`
- `examples/app_shell.kab`
- `tests/app_module.rs`

Roadmap: [ROADMAP.md](ROADMAP.md) **Våg APP**.
