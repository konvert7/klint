import { spawnSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

export interface CheckResult {
  exitCode: number;
  stdout: string;
  stderr: string;
}

function listTrackedFilesWithEol(root: string): string[] {
  const r = spawnSync("git", ["ls-files", "--eol"], { cwd: root, encoding: "utf8" });
  if (r.status !== 0) throw new Error(r.stderr || "git ls-files --eol failed\n");

  return r.stdout.split("\n");
}

export function hasCarriageReturn(eolLine: string): boolean {
  const [index, worktree] = eolLine.split(/\s+/);

  return /crlf|mixed/.test(index) || /crlf|mixed/.test(worktree);
}

export function pathFrom(eolLine: string): string {
  return eolLine.split("\t").slice(1).join("\t");
}

export function isBinary(contents: Buffer): boolean {
  return contents.includes(0);
}

function convertToLf(path: string): boolean {
  const contents = readFileSync(path);
  if (isBinary(contents)) return false;
  writeFileSync(path, contents.toString("utf8").replaceAll("\r\n", "\n"));

  return true;
}

function report(offenders: string[]): CheckResult {
  return {
    exitCode: 1,
    stdout: "",
    stderr:
      `CRLF found in tracked files (LF required):\n${offenders.join("\n")}\n` +
      `Run 'bun run lf:fix' to convert them.\n`,
  };
}

function fix(root: string, offenders: string[]): CheckResult {
  const skipped = offenders.filter((path) => !convertToLf(join(root, path)));
  const stdout = `Converted ${offenders.length - skipped.length} file(s) to LF.\n`;

  if (skipped.length === 0) return { exitCode: 0, stdout, stderr: "" };

  return {
    exitCode: 1,
    stdout,
    stderr: `Skipped binary file(s):\n${skipped.join("\n")}\n`,
  };
}

export function checkLf(root: string, shouldFix: boolean): CheckResult {
  const offenders = listTrackedFilesWithEol(root).filter(hasCarriageReturn).map(pathFrom);

  if (offenders.length === 0) {
    return {
      exitCode: 0,
      stdout: "All tracked files use LF line endings.\n",
      stderr: "",
    };
  }

  return shouldFix ? fix(root, offenders) : report(offenders);
}

if (import.meta.main) {
  try {
    const result = checkLf(process.cwd(), process.argv.includes("--fix"));
    process.stdout.write(result.stdout);
    process.stderr.write(result.stderr);
    process.exit(result.exitCode);
  } catch (error) {
    process.stderr.write(error instanceof Error ? error.message : String(error));
    process.exit(1);
  }
}
