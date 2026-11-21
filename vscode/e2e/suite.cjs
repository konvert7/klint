const vscode = require("vscode");
const { writeFileSync } = require("node:fs");
const { join } = require("node:path");

function snapshot(uri) {
  return vscode.languages.getDiagnostics(uri).map((d) => ({
    line: d.range.start.line,
    severity: vscode.DiagnosticSeverity[d.severity],
    message: d.message,
    source: d.source,
    code: d.code && typeof d.code === "object" ? d.code.value : d.code,
    target: d.code && typeof d.code === "object" ? d.code.target.toString() : null,
  }));
}

async function waitFor(check, timeoutMs) {
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    const value = check();
    if (value) return value;
    await new Promise((resolve) => setTimeout(resolve, 200));
  }
  return null;
}

async function run() {
  const root = vscode.workspace.workspaceFolders[0].uri;
  const file = vscode.Uri.joinPath(root, "src/index.ts");
  const result = { initial: null, afterFixAndSave: null };

  result.initial = await waitFor(() => {
    const items = snapshot(file);
    return items.length ? items : null;
  }, 20000);

  const document = await vscode.workspace.openTextDocument(file);
  const editor = await vscode.window.showTextDocument(document);
  await editor.edit((edit) =>
    edit.replace(new vscode.Range(0, 0, document.lineCount, 0), "export const ok = 1;\n")
  );
  await document.save();
  await waitFor(() => snapshot(file).length === 0, 20000);
  result.afterFixAndSave = snapshot(file);

  writeFileSync(join(root.fsPath, "e2e-result.json"), JSON.stringify(result, null, 2));
}

module.exports = { run };
