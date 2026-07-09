import * as vscode from "vscode";
import { runDocaiAsk, runDocaiSearch, runDocaiTopics } from "./docaiRunner";

interface ChatMessage {
  role: "user" | "assistant" | "error";
  text: string;
}

export class DocaiPanel {
  public static currentPanel: DocaiPanel | undefined;
  private readonly panel: vscode.WebviewPanel;
  private messages: ChatMessage[] = [];

  private constructor(panel: vscode.WebviewPanel) {
    this.panel = panel;
    this.panel.webview.html = this.renderHtml();
    this.panel.webview.onDidReceiveMessage(async (msg) => {
      if (msg.type === "ask" && typeof msg.query === "string") {
        await this.handleAsk(msg.query);
      }
    });
    this.panel.onDidDispose(() => {
      DocaiPanel.currentPanel = undefined;
    });
  }

  public static createOrShow(): DocaiPanel {
    if (DocaiPanel.currentPanel) {
      DocaiPanel.currentPanel.panel.reveal(vscode.ViewColumn.Beside);
      return DocaiPanel.currentPanel;
    }

    const panel = vscode.window.createWebviewPanel(
      "kabootarDocai",
      "Kabootar DocAI",
      vscode.ViewColumn.Beside,
      { enableScripts: true, retainContextWhenHidden: true }
    );
    DocaiPanel.currentPanel = new DocaiPanel(panel);
    return DocaiPanel.currentPanel;
  }

  public async ask(query: string): Promise<void> {
    await this.handleAsk(query);
    this.panel.reveal(vscode.ViewColumn.Beside);
  }

  private async handleAsk(query: string): Promise<void> {
    const trimmed = query.trim();
    if (!trimmed) {
      return;
    }

    this.messages.push({ role: "user", text: trimmed });
    this.postMessages();

    try {
      const answer = await runDocaiAsk(trimmed);
      this.messages.push({ role: "assistant", text: answer });
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      this.messages.push({ role: "error", text: message });
    }

    this.postMessages();
  }

  private postMessages(): void {
    void this.panel.webview.postMessage({
      type: "messages",
      messages: this.messages,
    });
  }

  private renderHtml(): string {
    return `<!DOCTYPE html>
<html lang="sv">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <style>
    :root {
      --bg: var(--vscode-editor-background);
      --fg: var(--vscode-editor-foreground);
      --border: var(--vscode-panel-border);
      --input-bg: var(--vscode-input-background);
      --btn-bg: var(--vscode-button-background);
      --btn-fg: var(--vscode-button-foreground);
      --user-bg: var(--vscode-textBlockQuote-background);
      --assistant-bg: var(--vscode-editor-inactiveSelectionBackground);
      --error: var(--vscode-errorForeground);
    }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      font-family: var(--vscode-font-family);
      font-size: var(--vscode-font-size);
      color: var(--fg);
      background: var(--bg);
      display: flex;
      flex-direction: column;
      height: 100vh;
    }
    header {
      padding: 12px 16px;
      border-bottom: 1px solid var(--border);
      font-weight: 600;
    }
    #messages {
      flex: 1;
      overflow-y: auto;
      padding: 16px;
      display: flex;
      flex-direction: column;
      gap: 12px;
    }
    .msg {
      padding: 10px 12px;
      border-radius: 8px;
      line-height: 1.5;
      white-space: pre-wrap;
      word-break: break-word;
    }
    .user { background: var(--user-bg); align-self: flex-end; max-width: 90%; }
    .assistant { background: var(--assistant-bg); align-self: flex-start; max-width: 95%; }
    .error { color: var(--error); border: 1px solid var(--error); }
    pre.code {
      background: var(--vscode-textCodeBlock-background);
      padding: 8px;
      border-radius: 4px;
      overflow-x: auto;
      margin: 8px 0 0;
    }
    footer {
      border-top: 1px solid var(--border);
      padding: 12px 16px;
      display: flex;
      gap: 8px;
    }
    input {
      flex: 1;
      background: var(--input-bg);
      color: var(--fg);
      border: 1px solid var(--border);
      padding: 8px 10px;
      border-radius: 4px;
    }
    button {
      background: var(--btn-bg);
      color: var(--btn-fg);
      border: none;
      padding: 8px 14px;
      border-radius: 4px;
      cursor: pointer;
    }
    .hint {
      padding: 0 16px 8px;
      opacity: 0.75;
      font-size: 0.9em;
    }
  </style>
</head>
<body>
  <header>Kabootar DocAI — fråga om dokumentationen</header>
  <div class="hint">Exempel: hur importerar jag science? · stat_mean · SQL INSERT · HTTP routes</div>
  <div id="messages"></div>
  <footer>
    <input id="query" type="text" placeholder="Skriv din fråga…" />
    <button id="send">Fråga</button>
  </footer>
  <script>
    const vscode = acquireVsCodeApi();
    const messagesEl = document.getElementById('messages');
    const queryEl = document.getElementById('query');
    const sendBtn = document.getElementById('send');

    function escapeHtml(s) {
      return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
    }

    function renderMessages(messages) {
      messagesEl.innerHTML = messages.map(m => {
        const cls = m.role;
        let body = escapeHtml(m.text);
        body = body.replace(/\\*\\*(.+?)\\*\\*/g, '<strong>$1</strong>');
        body = body.replace(/\`\`\`kabootar\\n([\\s\\S]*?)\`\`\`/g, '<pre class="code"><code>$1</code></pre>');
        body = body.replace(/\\n/g, '<br/>');
        return '<div class="msg ' + cls + '">' + body + '</div>';
      }).join('');
      messagesEl.scrollTop = messagesEl.scrollHeight;
    }

    window.addEventListener('message', e => {
      if (e.data.type === 'messages') {
        renderMessages(e.data.messages);
      }
    });

    function submit() {
      const q = queryEl.value.trim();
      if (!q) return;
      queryEl.value = '';
      vscode.postMessage({ type: 'ask', query: q });
    }

    sendBtn.addEventListener('click', submit);
    queryEl.addEventListener('keydown', e => {
      if (e.key === 'Enter') submit();
    });
    queryEl.focus();
  </script>
</body>
</html>`;
  }
}

export async function promptAndAsk(): Promise<void> {
  const query = await vscode.window.showInputBox({
    title: "Kabootar DocAI",
    prompt: "Fråga om Kabootar-dokumentationen",
    placeHolder: "t.ex. hur importerar jag science?",
  });
  if (!query) {
    return;
  }
  const panel = DocaiPanel.createOrShow();
  await panel.ask(query);
}

export async function searchDocumentation(): Promise<void> {
  const query = await vscode.window.showInputBox({
    title: "DocAI — sök",
    prompt: "Sök i dokumentationen",
    placeHolder: "t.ex. SQL WHERE",
  });
  if (!query) {
    return;
  }

  try {
    const raw = await runDocaiSearch(query);
    const doc = await vscode.workspace.openTextDocument({
      content: raw || "(inga träffar)",
      language: "plaintext",
    });
    await vscode.window.showTextDocument(doc, { preview: true });
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    void vscode.window.showErrorMessage(message);
  }
}

export async function showTopics(): Promise<void> {
  try {
    const topics = await runDocaiTopics();
    const pick = await vscode.window.showQuickPick(topics, {
      title: "DocAI — dokument",
      placeHolder: "Välj ett dokumentämne",
    });
    if (pick) {
      const panel = DocaiPanel.createOrShow();
      await panel.ask(`vad handlar ${pick} om?`);
    }
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    void vscode.window.showErrorMessage(message);
  }
}
