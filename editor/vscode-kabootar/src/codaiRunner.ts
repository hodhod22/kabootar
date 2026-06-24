import * as fs from "fs";
import * as path from "path";
import { execFile } from "child_process";
import { promisify } from "util";
import * as vscode from "vscode";

const execFileAsync = promisify(execFile);

export function resolveCodaiPath(): string | undefined {
  const configured = vscode.workspace
    .getConfiguration("kabootar")
    .get<string>("codai.path")
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
      ? ["kabootar-codai.exe", "kabootar-codai"]
      : ["kabootar-codai", "kabootar-codai.exe"];

  const dirs = ["target/release", "target/debug"];
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

function workspaceRoot(): string | undefined {
  const folders = vscode.workspace.workspaceFolders;
  return folders?.[0]?.uri.fsPath;
}

async function runCodai(args: string[]): Promise<string> {
  const bin = resolveCodaiPath();
  if (!bin) {
    throw new Error(
      "kabootar-codai hittades inte. Kör: cargo build --bin kabootar-codai"
    );
  }
  const { stdout } = await execFileAsync(bin, args, {
    maxBuffer: 4 * 1024 * 1024,
    windowsHide: true,
    cwd: workspaceRoot(),
  });
  return stdout.trim();
}

export async function runCodaiSync(dir = "."): Promise<string> {
  return runCodai(["--project-sync", "--dir", dir]);
}

export async function runCodaiSuggest(query: string): Promise<string> {
  return runCodai(["--suggest", query, "--limit", "8"]);
}

export async function runCodaiProjectSuggest(query: string): Promise<string> {
  return runCodai(["--project-suggest", query, "--limit", "5"]);
}

export async function runCodaiUtil(id: string): Promise<string> {
  return runCodai(["--util", id]);
}

export async function openWorkspaceFile(relPath: string): Promise<void> {
  const root = workspaceRoot();
  if (!root) {
    throw new Error("Ingen workspace-mapp öppen.");
  }
  const full = path.join(root, relPath);
  if (!fs.existsSync(full)) {
    throw new Error(`Filen finns inte ännu: ${relPath}. Kör CodAI sync först.`);
  }
  const doc = await vscode.workspace.openTextDocument(full);
  await vscode.window.showTextDocument(doc, { preview: false });
}
