# Kabootar — LSP / IDE-stöd

Kabootar har en Language Server (`kabootar-lsp`) och en VS Code/Cursor-extension.

## Bygg LSP

```bash
cargo build --features lsp
```

Binären hamnar i `target/debug/kabootar-lsp` (eller `.exe` på Windows).

## VS Code / Cursor extension (rekommenderat)

```bash
cargo build --features lsp
cd editor/vscode-kabootar
npm install
npm run compile
code --install-extension .
```

Extensionen ger:

- Syntax highlighting för `.kab` / `.kabootar`
- Automatisk start av `kabootar-lsp`
- Virtuella modulfiler (`kabootar://module/math`) vid go to definition
- **DocAI** och **CodAI** — kommandon och paneler (VS Code & Cursor)

Se [IDE.md](IDE.md) och [editor/vscode-kabootar/README.md](../editor/vscode-kabootar/README.md).

## LSP-funktioner

| LSP-funktion | Beskrivning |
|--------------|-------------|
| `textDocument/publishDiagnostics` | Lexer- och parserfel med rad/kolumn |
| `textDocument/completion` | Nyckelord, inbyggda, **concrete types efter `<`**, generiska mallar (`Box`, `id<T>`) |
| `textDocument/hover` | Signatur för fn/klass/enum + generiska typer; specialisering vid `$`-mangling |
| `textDocument/definition` | Gå till definition (`let`, `fn`, `class`, `import`, **type param `T`**) |

### Manuell LSP-konfiguration

Om du inte använder extensionen:

```json
{
  "kabootar.languageServer.path": "C:/path/to/kabootar-lsp.exe"
}
```

## Go to definition

| Symbol | Hoppa till |
|--------|------------|
| `let x` | Variabeldeklarationen |
| `fn foo` | Funktionsdeklarationen |
| `class Person` | Klassdeklarationen |
| Fält / metod | Respektive deklaration i klassen |
| `import "math"` | Modulnamnet i import-raden |
| `add` (efter import) | `fn add` i modulkällan (`kabootar://module/math`) |
| `T` (type param) | Type parameter-deklarationen i `<T>` |

Importerade symboler pekar på inbyggd modulkällkod via virtuell URI `kabootar://module/<namn>`. VS Code-extensionen registrerar en `TextDocumentContentProvider` för detta schema.

## Arkitektur

```
editor/vscode-kabootar/   # VS Code-extension (grammar + LSP-klient)
src/span.rs               # Span, Spanned<T>, ParseError
src/language/symbols.rs   # SymbolIndex, definition lookup
src/language/generics_lsp.rs # hover/completion för generics (G11)
src/language/mod.rs       # analyze(), goto_definition(), hover_at(), completions_at()
src/bin/kabootar-lsp.rs   # tower-lsp-server (stdio)
src/lexer.rs              # Spanned tokens + tokenize()
src/parser.rs             # Samlar symboler vid parse
```
