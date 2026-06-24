import * as fs from "fs";
import * as path from "path";
import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";
import {
  CodaiPanel,
  openCodaiPanel,
  suggestProjectTemplate,
  suggestUtility,
  syncProject,
} from "./codaiPanel";
import {
  DocaiPanel,
  promptAndAsk,
  searchDocumentation,
  showTopics,
} from "./docaiPanel";
import { moduleSource } from "./modules";

let client: LanguageClient | undefined;

class KabootarModuleProvider implements vscode.TextDocumentContentProvider {
  provideTextDocumentContent(uri: vscode.Uri): string {
    const moduleName = uri.path.replace(/^\//, "");
    const source = moduleSource(moduleName);
    if (source) {
      return source;
    }
    return `// Module not found: ${moduleName}\n`;
  }
}

function resolveServerPath(): string | undefined {
  const configured = vscode.workspace
    .getConfiguration("kabootar")
    .get<string>("languageServer.path")
    ?.trim();
  if (configured && fs.existsSync(configured)) {
    return configured;
  }

  const folders = vscode.workspace.workspaceFolders;
  if (!folders || folders.length === 0) {
    return undefined;
  }

  const root = folders[0].uri.fsPath;
  const names =
    process.platform === "win32"
      ? ["kabootar-lsp.exe", "kabootar-lsp"]
      : ["kabootar-lsp", "kabootar-lsp.exe"];

  const dirs = ["target/debug", "target/release"];
  for (const dir of dirs) {
    for (const name of names) {
      const full = path.join(root, dir, name);
      if (fs.existsSync(full)) {
        return full;
      }
    }
  }

  return undefined;
}

function startLanguageServer(context: vscode.ExtensionContext): void {
  const serverPath = resolveServerPath();
  if (!serverPath) {
    void vscode.window.showWarningMessage(
      "Kabootar: kabootar-lsp hittades inte. Kör: cargo build --features lsp"
    );
    return;
  }

  const serverOptions: ServerOptions = {
    run: { command: serverPath, transport: TransportKind.stdio },
    debug: { command: serverPath, transport: TransportKind.stdio },
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "kabootar" }],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher("**/*.{kab,kabootar}"),
    },
  };

  client = new LanguageClient(
    "kabootar",
    "Kabootar Language Server",
    serverOptions,
    clientOptions
  );

  context.subscriptions.push(client);
  void client.start();
}

function registerCodaiCommands(context: vscode.ExtensionContext): void {
  context.subscriptions.push(
    vscode.commands.registerCommand("kabootar.codai.sync", () => syncProject()),
    vscode.commands.registerCommand("kabootar.codai.openPanel", () => openCodaiPanel()),
    vscode.commands.registerCommand("kabootar.codai.suggest", () => suggestUtility()),
    vscode.commands.registerCommand("kabootar.codai.projectSuggest", () =>
      suggestProjectTemplate()
    )
  );
}

function registerDocaiCommands(context: vscode.ExtensionContext): void {
  context.subscriptions.push(
    vscode.commands.registerCommand("kabootar.docai.ask", () => promptAndAsk()),
    vscode.commands.registerCommand("kabootar.docai.openPanel", () => {
      DocaiPanel.createOrShow();
    }),
    vscode.commands.registerCommand("kabootar.docai.search", () =>
      searchDocumentation()
    ),
    vscode.commands.registerCommand("kabootar.docai.topics", () => showTopics())
  );
}

export function activate(context: vscode.ExtensionContext): void {
  context.subscriptions.push(
    vscode.workspace.registerTextDocumentContentProvider(
      "kabootar",
      new KabootarModuleProvider()
    )
  );

  registerDocaiCommands(context);
  registerCodaiCommands(context);
  startLanguageServer(context);
}

export async function deactivate(): Promise<void> {
  if (client) {
    await client.stop();
  }
}
