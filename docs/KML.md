# Kabootar — KML (Kabootar Markup Language)

KML är ett XML-liknande markup-språk för Kabootars egen DOM (`kdom`), oberoende av webbläsarens DOM.

## Syntax

```kml
<div class="main">
  <h1>Title</h1>
  <p>Hello Kabootar</p>
</div>
```

### Element

| Form | Exempel |
|------|---------|
| Öppnande + stängande | `<p>text</p>` |
| Självstängande | `<br />` |
| Attribut | `<div class="box" id="root">` |

Attributvärden måste vara citerade med `"` eller `'`.

### Text

Text mellan taggar blir textnoder:

```kml
<p>Hello</p>
```

## API

| Funktion | Beskrivning |
|----------|-------------|
| `kml(source)` | Parsar KML-sträng → `KabootarDom`-nod |
| `kdom_render(node)` | Renderar nod till HTML-sträng |
| `kdom` | Inbyggd DOM-root (`<html />`) |

## Exempel

```kabootar
let ui = kml("<div class=\"app\"><h1>Kabootar</h1></div>");
let html = kdom_render(ui);
println(html);
```

## Implementation

- Parser och renderer: `src/kml/mod.rs`
- DOM-typer: `src/runtime/kabootar_dom.rs`

Se [RUNTIME.md](RUNTIME.md) för hur KDOM skiljer sig från browser-DOM.
