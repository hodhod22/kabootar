import * as vscode from "vscode";
import {
  openWorkspaceFile,
  runCodaiProjectSuggest,
  runCodaiSuggest,
  runCodaiSync,
  runCodaiUtil,
} from "./codaiRunner";

export class CodaiPanel {
  public static currentPanel: CodaiPanel | undefined;
  private readonly panel: vscode.WebviewPanel;
  private log = "";

  private constructor(panel: vscode.WebviewPanel) {
    this.panel = panel;
    this.panel.webview.html = this.renderHtml();
    this.panel.webview.onDidReceiveMessage(async (msg) => {
      if (msg.type === "sync") {
        await this.handleSync();
      } else if (msg.type === "suggest" && typeof msg.query === "string") {
        await this.handleSuggest(msg.query);
      } else if (msg.type === "project" && typeof msg.query === "string") {
        await this.handleProjectSuggest(msg.query);
      } else if (msg.type === "util" && typeof msg.id === "string") {
        await this.handleUtil(msg.id);
      } else if (msg.type === "open" && typeof msg.file === "string") {
        await this.openFile(msg.file);
      }
    });
    this.panel.onDidDispose(() => {
      CodaiPanel.currentPanel = undefined;
    });
  }

  public static createOrShow(): CodaiPanel {
    if (CodaiPanel.currentPanel) {
      CodaiPanel.currentPanel.panel.reveal(vscode.ViewColumn.Beside);
      return CodaiPanel.currentPanel;
    }

    const panel = vscode.window.createWebviewPanel(
      "kabootarCodai",
      "Kabootar CodAI",
      vscode.ViewColumn.Beside,
      { enableScripts: true, retainContextWhenHidden: true }
    );
    CodaiPanel.currentPanel = new CodaiPanel(panel);
    return CodaiPanel.currentPanel;
  }

  public append(text: string): void {
    this.log += (this.log ? "\n\n" : "") + text;
    void this.panel.webview.postMessage({ type: "log", text: this.log });
  }

  private async handleSync(): Promise<void> {
    try {
      const out = await runCodaiSync(".");
      this.append(out);
      void vscode.window.showInformationMessage(
        "CodAI: PROGRESS.txt och road/ uppdaterade."
      );
    } catch (err) {
      this.append(err instanceof Error ? err.message : String(err));
    }
  }

  private async handleSuggest(query: string): Promise<void> {
    try {
      const out = await runCodaiSuggest(query.trim());
      this.append(`Utility-förslag för "${query}":\n${out}`);
    } catch (err) {
      this.append(err instanceof Error ? err.message : String(err));
    }
  }

  private async handleProjectSuggest(query: string): Promise<void> {
    try {
      const out = await runCodaiProjectSuggest(query.trim());
      this.append(`Projektmallar för "${query}":\n${out}`);
    } catch (err) {
      this.append(err instanceof Error ? err.message : String(err));
    }
  }

  private async handleUtil(id: string): Promise<void> {
    try {
      const code = await runCodaiUtil(id.trim());
      const doc = await vscode.workspace.openTextDocument({
        content: code,
        language: "kabootar",
      });
      await vscode.window.showTextDocument(doc, { preview: true });
    } catch (err) {
      this.append(err instanceof Error ? err.message : String(err));
    }
  }

  private async openFile(rel: string): Promise<void> {
    try {
      await openWorkspaceFile(rel);
    } catch (err) {
      this.append(err instanceof Error ? err.message : String(err));
    }
  }

  private renderHtml(): string {
    return `<!DOCTYPE html>
<html lang="sv">
<head>
  <meta charset="UTF-8" />
  <style>
    body { font-family: var(--vscode-font-family); color: var(--vscode-editor-foreground);
      background: var(--vscode-editor-background); margin: 0; padding: 12px; }
    h2 { margin: 0 0 8px; font-size: 1.1em; }
    .hint { opacity: 0.8; font-size: 0.9em; margin-bottom: 12px; }
    button { margin: 4px 4px 4px 0; padding: 6px 10px; cursor: pointer;
      background: var(--vscode-button-background); color: var(--vscode-button-foreground); border: none; border-radius: 4px; }
    input { width: 100%; padding: 6px; margin: 4px 0; box-sizing: border-box;
      background: var(--vscode-input-background); color: var(--vscode-input-foreground); border: 1px solid var(--vscode-panel-border); }
    #log { white-space: pre-wrap; font-size: 0.9em; margin-top: 12px; padding: 10px;
      background: var(--vscode-textBlockQuote-background); border-radius: 6px; max-height: 50vh; overflow: auto; }
  </style>
</head>
<body>
  <h2>CodAI — kodassistent</h2>
  <p class="hint">VS Code &amp; Cursor. Synka efter kodändringar.</p>
  <button id="sync">Synka projekt (PROGRESS.txt + road/)</button>
  <div>
    <button data-file="PROGRESS.txt">PROGRESS.txt</button>
    <button data-file="road/NOW.txt">road/NOW.txt</button>
    <button data-file="road/IDE.txt">road/IDE.txt</button>
  </div>
  <input id="suggest" placeholder="Föreslå utility, t.ex. REST API" />
  <button id="btnSuggest">Utility-förslag</button>
  <input id="project" placeholder="Föreslå projektmall, t.ex. PLC" />
  <button id="btnProject">Projektmall</button>
  <input id="util" placeholder="Utility-id, t.ex. http-route-get" />
  <button id="btnUtil">Visa kodmall</button>
  <div id="log"></div>
  <script>
    const vscode = acquireVsCodeApi();
    document.getElementById('sync').onclick = () => vscode.postMessage({ type: 'sync' });
    document.getElementById('btnSuggest').onclick = () => {
      const q = document.getElementById('suggest').value;
      if (q) vscode.postMessage({ type: 'suggest', query: q });
    };
    document.getElementById('btnProject').onclick = () => {
      const q = document.getElementById('project').value;
      if (q) vscode.postMessage({ type: 'project', query: q });
    };
    document.getElementById('btnUtil').onclick = () => {
      const id = document.getElementById('util').value;
      if (id) vscode.postMessage({ type: 'util', id });
    };
    document.querySelectorAll('[data-file]').forEach(btn => {
      btn.addEventListener('click', () => vscode.postMessage({ type: 'open', file: btn.dataset.file }));
    });
    window.addEventListener('message', e => {
      if (e.data.type === 'log') document.getElementById('log').textContent = e.data.text;
    });
  </script>
</body>
</html>`;
  }
}

export async function syncProject(): Promise<void> {
  try {
    const out = await runCodaiSync(".");
    const panel = CodaiPanel.createOrShow();
    panel.append(out);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    void vscode.window.showErrorMessage(message);
  }
}

export async function suggestUtility(): Promise<void> {
  const query = await vscode.window.showInputBox({
    title: "CodAI — utility-förslag",
    prompt: "Beskriv vad du vill bygga",
    placeHolder: "t.ex. SQL insert, PLC timer",
  });
  if (!query) return;
  try {
    const raw = await runCodaiSuggest(query);
    const doc = await vscode.workspace.openTextDocument({
      content: raw || "(inga träffar)",
      language: "plaintext",
    });
    await vscode.window.showTextDocument(doc, { preview: true });
  } catch (err) {
    void vscode.window.showErrorMessage(
      err instanceof Error ? err.message : String(err)
    );
  }
}

export async function suggestProjectTemplate(): Promise<void> {
  const query = await vscode.window.showInputBox({
    title: "CodAI — projektmall",
    prompt: "Vad vill du bygga?",
    placeHolder: "t.ex. REST API, fullstack webb",
  });
  if (!query) return;
  try {
    const raw = await runCodaiProjectSuggest(query);
    const doc = await vscode.workspace.openTextDocument({
      content: raw || "(inga träffar)",
      language: "plaintext",
    });
    await vscode.window.showTextDocument(doc, { preview: true });
  } catch (err) {
    void vscode.window.showErrorMessage(
      err instanceof Error ? err.message : String(err)
    );
  }
}

export function openCodaiPanel(): void {
  CodaiPanel.createOrShow();
}
