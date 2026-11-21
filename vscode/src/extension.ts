import { existsSync } from "node:fs";
import { join } from "node:path";
import * as vscode from "vscode";
import { resolveKlintBinary } from "./binary";
import { diagnosticsByFile, type FileDiagnostic } from "./diagnostics";
import { runKlint } from "./run";

const CONFIG_FILES = ["klint.yaml", "klint.config.json"];

let collection: vscode.DiagnosticCollection;
let output: vscode.OutputChannel;
const running = new Map<string, Promise<void>>();
const rerun = new Set<string>();

function hasKlintConfig(folder: vscode.WorkspaceFolder): boolean {
  return CONFIG_FILES.some((name) => existsSync(join(folder.uri.fsPath, name)));
}

function binaryFor(folder: vscode.WorkspaceFolder): string {
  return resolveKlintBinary({
    workspaceRoot: folder.uri.fsPath,
    configuredPath: vscode.workspace
      .getConfiguration("klint", folder)
      .get<string>("path"),
    platform: process.platform,
    exists: existsSync,
  });
}

function toVscodeDiagnostic(item: FileDiagnostic): vscode.Diagnostic {
  const { start, end } = item.characters ?? { start: 0, end: Number.MAX_SAFE_INTEGER };
  const range = new vscode.Range(item.line, start, item.line, end);
  const severity =
    item.severity === "warning"
      ? vscode.DiagnosticSeverity.Warning
      : vscode.DiagnosticSeverity.Error;
  const diagnostic = new vscode.Diagnostic(range, item.message, severity);
  diagnostic.source = "klint";
  diagnostic.code = { value: item.rule, target: vscode.Uri.parse(item.docsUrl) };

  return diagnostic;
}

function clearFolder(folder: vscode.WorkspaceFolder): void {
  const prefix = `${folder.uri.toString()}/`;
  collection.forEach((uri) => {
    if (uri.toString().startsWith(prefix)) collection.delete(uri);
  });
}

function publish(folder: vscode.WorkspaceFolder, stdout: string): void {
  clearFolder(folder);

  for (const [file, items] of diagnosticsByFile(stdout)) {
    collection.set(vscode.Uri.joinPath(folder.uri, file), items.map(toVscodeDiagnostic));
  }
}

async function lintOnce(folder: vscode.WorkspaceFolder): Promise<void> {
  const binary = binaryFor(folder);

  try {
    const result = await runKlint(binary, folder.uri.fsPath);
    publish(folder, result.stdout);
  } catch (error) {
    output.appendLine(`[${folder.name}] ${String(error)}`);
  }
}

async function lintUntilSettled(folder: vscode.WorkspaceFolder): Promise<void> {
  do {
    rerun.delete(folder.uri.toString());
    await lintOnce(folder);
  } while (rerun.has(folder.uri.toString()));
}

function lint(folder: vscode.WorkspaceFolder): void {
  const key = folder.uri.toString();

  if (!hasKlintConfig(folder)) return;

  if (running.has(key)) {
    rerun.add(key);

    return;
  }

  const run = lintUntilSettled(folder).finally(() => running.delete(key));
  running.set(key, run);
}

function lintAll(): void {
  for (const folder of vscode.workspace.workspaceFolders ?? []) lint(folder);
}

function lintFolderOf(uri: vscode.Uri): void {
  const folder = vscode.workspace.getWorkspaceFolder(uri);
  if (folder) lint(folder);
}

export function activate(context: vscode.ExtensionContext): void {
  collection = vscode.languages.createDiagnosticCollection("klint");
  output = vscode.window.createOutputChannel("klint");
  context.subscriptions.push(
    collection,
    output,
    vscode.commands.registerCommand("klint.lint", lintAll),
    vscode.workspace.onDidSaveTextDocument((document) => lintFolderOf(document.uri)),
    vscode.workspace.onDidChangeWorkspaceFolders((event) => {
      event.removed.forEach(clearFolder);
      event.added.forEach(lint);
    }),
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (event.affectsConfiguration("klint")) lintAll();
    })
  );
  lintAll();
}

export function deactivate(): void {}
