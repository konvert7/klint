import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const root = new URL("..", import.meta.url).pathname;
const outDir = mkdtempSync(join(tmpdir(), "klint-pack-check-"));

try {
  const packed = run("bun", [
    "pm",
    "pack",
    "--ignore-scripts",
    "--destination",
    outDir,
    "--quiet",
  ]);
  const tarball = packed.startsWith("/") ? packed : join(outDir, packed);

  const list = run("tar", ["-tzf", tarball]).split("\n").filter(Boolean).sort();

  assertIncludes(list, [
    "package/README.md",
    "package/cli.ts",
    "package/package.json",
    "package/core/native-binary.ts",
    "package/skill/klint-rules/SKILL.md",
  ]);

  assertExcludes(list, [
    "package/npm/native/darwin-arm64/package.json",
    "package/npm/native/darwin-x64/package.json",
    "package/npm/native/linux-x64/package.json",
    "package/npm/native/win32-x64/package.json",
    "package/tests/pack-check.ts",
    "package/tests/native-binary.test.ts",
    "package/crates/klint-rs/Cargo.toml",
  ]);

  process.stdout.write(`pack:check ok (${list.length} files)\n`);
} finally {
  rmSync(outDir, { recursive: true, force: true });
}

function run(command: string, args: string[]): string {
  const result = spawnSync(command, args, {
    cwd: root,
    encoding: "utf-8",
  });
  if ((result.status ?? -1) !== 0) {
    process.stderr.write(result.stdout);
    process.stderr.write(result.stderr);
    process.exit(result.status ?? 1);
  }
  return result.stdout.trim();
}

function assertIncludes(list: string[], expected: string[]): void {
  const actual = new Set(list);
  const missing = expected.filter((item) => !actual.has(item));
  if (missing.length > 0) {
    fail(`pack:check missing expected files:\n${missing.join("\n")}`);
  }
}

function assertExcludes(list: string[], forbidden: string[]): void {
  const actual = new Set(list);
  const present = forbidden.filter((item) => actual.has(item));
  if (present.length > 0) {
    fail(`pack:check found forbidden files:\n${present.join("\n")}`);
  }
}

function fail(message: string): never {
  process.stderr.write(`${message}\n`);
  process.exit(1);
}
