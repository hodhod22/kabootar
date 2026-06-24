# Kabootar — VS Code & Cursor (IDE)

Kabootar är byggd för att skriva kod i **VS Code** och **Cursor** med samma extension, LSP och AI-moduler.

## Snabbstart

```bash
# I Kabootar-repot (nova-interpreter)
cargo build --features lsp
cargo build --bin kabootar-docai --bin kabootar-codai

cd editor/vscode-kabootar
npm install && npm run compile
code --install-extension .
```

**Cursor:** samma steg — öppna `editor/vscode-kabootar`, kör `F5` (Run Extension) eller installera `.vsix`.

Öppna ditt Kabootar-projekt som **workspace-mapp** (mappen med `kabootar.toml`).

## Vad du får

| Funktion | VS Code / Cursor |
|----------|------------------|
| Syntax | `.kab` / `.kabootar` highlighting |
| LSP | Fel, autocomplete, hover, go to definition |
| Moduler | `import "math"` → hoppa till `kabootar://module/math` |
| DocAI | Dokumentationsfrågor (Command Palette) |
| CodAI | Kodutilities, scaffold, **sync** av PROGRESS.txt och `road/` |

## Rekommenderat arbetsflöde

1. **Öppna projektmappen** som workspace (inte bara en enskild fil).
2. **Skriv kod** i `main.kab` och `lib/*.kab` — LSP hjälper under tiden.
3. **Synka CodAI** efter ändringar:
   - Command Palette → `Kabootar: CodAI — synka projekt`
   - eller högerklick i `.kab` → samma kommando
4. **Läs** `road/NOW.txt` och `PROGRESS.txt` för nästa steg.
5. **Kör** i terminal: `kabootar serve --watch main.kab`

## CodAI-kommandon (IDE)

| Kommando | Beskrivning |
|----------|-------------|
| `Kabootar: CodAI — synka projekt` | Uppdaterar `PROGRESS.txt` och `road/` från nuvarande kod |
| `Kabootar: Öppna CodAI-panel` | Sidopanel: sync, utilities, projektmallar |
| `Kabootar: CodAI — föreslå utility` | Tailwind-liknande kodbyggblock |
| `Kabootar: CodAI — föreslå projektmall` | web, api, science, … |

## DocAI-kommandon

| Kommando | Beskrivning |
|----------|-------------|
| `Kabootar: Fråga DocAI` | Fråga om dokumentation |
| `Kabootar: Öppna DocAI-panel` | Doc-chatt |
| `Kabootar: Sök i dokumentation` | Sökträffar |
| `Kabootar: DocAI ämnen` | Bläddra docs |

## Inställningar

| Inställning | Binär | Auto-detektering |
|-------------|-------|------------------|
| `kabootar.languageServer.path` | `kabootar-lsp` | `target/debug/` eller `target/release/` |
| `kabootar.docai.path` | `kabootar-docai` | samma |
| `kabootar.codai.path` | `kabootar-codai` | samma |

Lämna tomt om du bygger i samma repo som språket.

## Cursor-specifikt

- Lägg `@PROGRESS.txt`, `@road/NOW.txt` eller `@road/IDE.txt` i AI-chatten som kontext.
- Be agenten: *"Fortsätt enligt road/NOW.txt"* efter sync.
- `road/IDE.txt` uppdateras vid sync med IDE-tips för ditt projekt.

## VS Code-specifikt

- Dela editorn: kod + `PROGRESS.txt` sida vid sida.
- Problems-panelen visar LSP-parsefel direkt.
- Terminal integrerad: `kabootar serve`, `kabootar compile`.

## Filer CodAI skapar/uppdaterar

```
projekt/
├── PROGRESS.txt      # Status och nästa steg
└── road/
    ├── ROADMAP.txt   # Utvecklingsfaser
    ├── NOW.txt       # Gör härnäst
    ├── DONE.txt      # Det som är klart
    └── IDE.txt       # VS Code & Cursor rekommendationer
```

## I Kabootar-kod

```kabootar
import "codai";
code_project_sync(".");
code_util("http-route-get");
```

## Se även

- [LSP.md](LSP.md) — language server
- [CODAI.md](CODAI.md) — kodassistent
- [DOCAI.md](DOCAI.md) — dokumentations-AI
- [editor/vscode-kabootar/README.md](../editor/vscode-kabootar/README.md)
