import { execFile } from "node:child_process";

export interface KlintRun {
  stdout: string;
  stderr: string;
}

const LINT_EXIT_CODES = new Set([0, 2]);
const MAX_OUTPUT_BYTES = 64 * 1024 * 1024;

function needsShell(binary: string): boolean {
  return /\.(cmd|bat)$/i.test(binary);
}

function quoted(text: string): string {
  return `"${text}"`;
}

function command(binary: string, root: string): { file: string; args: string[] } {
  const args = ["--config", root, "--json"];

  if (!needsShell(binary)) return { file: binary, args };

  return { file: quoted(binary), args: args.map(quoted) };
}

function exitCode(error: unknown): number | undefined {
  if (error === null) return 0;

  const code = (error as { code?: unknown }).code;

  return typeof code === "number" ? code : undefined;
}

export function runKlint(binary: string, root: string): Promise<KlintRun> {
  const { file, args } = command(binary, root);

  return new Promise((resolve, reject) => {
    execFile(
      file,
      args,
      { cwd: root, maxBuffer: MAX_OUTPUT_BYTES, shell: needsShell(binary) },
      (error, stdout, stderr) => {
        const code = exitCode(error);
        if (code !== undefined && LINT_EXIT_CODES.has(code)) {
          resolve({ stdout, stderr });

          return;
        }

        reject(new Error(`${binary} failed: ${stderr.trim() || String(error)}`));
      }
    );
  });
}
