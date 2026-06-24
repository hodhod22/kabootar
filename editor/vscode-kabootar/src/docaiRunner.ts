import * as fs from "fs";
import * as path from "path";
import { execFile } from "child_process";
import { promisify } from "util";
import * as vscode from "vscode";

const execFileAsync = promisify(execFile);

export function resolveDocaiPath(): string | undefined {
  const configured = vscode.workspace
    .getConfiguration("kabootar")
    .get<string>("docai.path")
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
      ? ["kabootar-docai.exe", "kabootar-docai"]
      : ["kabootar-docai", "kabootar-docai.exe"];

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

export async function runDocaiAsk(query: string): Promise<string> {
  const bin = resolveDocaiPath();
  if (!bin) {
    throw new Error(
      "kabootar-docai hittades inte. Kör: cargo build --bin kabootar-docai"
    );
  }
  const { stdout } = await execFileAsync(bin, ["--ask", query], {
    maxBuffer: 4 * 1024 * 1024,
    windowsHide: true,
  });
  return stdout.trim();
}

export async function runDocaiSearch(
  query: string,
  limit = 8
): Promise<string> {
  const bin = resolveDocaiPath();
  if (!bin) {
    throw new Error(
      "kabootar-docai hittades inte. Kör: cargo build --bin kabootar-docai"
    );
  }
  const { stdout } = await execFileAsync(
    bin,
    ["--search", query, "--limit", String(limit)],
    { maxBuffer: 4 * 1024 * 1024, windowsHide: true }
  );
  return stdout.trim();
}

export async function runDocaiTopics(): Promise<string[]> {
  const bin = resolveDocaiPath();
  if (!bin) {
    throw new Error(
      "kabootar-docai hittades inte. Kör: cargo build --bin kabootar-docai"
    );
  }
  const { stdout } = await execFileAsync(bin, ["--topics"], {
    maxBuffer: 1024 * 1024,
    windowsHide: true,
  });
  return stdout
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter(Boolean);
}
