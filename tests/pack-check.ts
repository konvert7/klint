import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { nativePackages } from "../core/native-binary";

const root = new URL("..", import.meta.url).pathname;
const outDir = mkdtempSync(join(tmpdir(), "klint-pack-check-"));

try {
  const mainList = packAndList(root);

  assertIncludes(mainList, [
    "package/README.md",
    "package/cli.ts",
    "package/package.json",
    "package/core/native-binary.ts",
    "package/skill/klint-rules/SKILL.md",
  ]);

  assertExcludes(mainList, [
    "package/npm/native/darwin-arm64/package.json",
    "package/npm/native/darwin-x64/package.json",
    "package/npm/native/linux-x64/package.json",
    "package/npm/native/win32-x64/package.json",
    "package/tests/pack-check.ts",
    "package/tests/native-binary.test.ts",
    "package/crates/klint-rs/Cargo.toml",
  ]);

  for (const nativePackage of nativePackages()) {
    const packageRoot = join(
      root,
      "npm",
      "native",
      `${nativePackage.platform}-${nativePackage.arch}`
    );
    const packageJson = JSON.parse(
      readFileSync(join(packageRoot, "package.json"), "utf-8")
    );
    const expectedFiles = ["package/package.json"];
    const list = packAndList(packageRoot);

    assertEquals(
      packageJson.name,
      nativePackage.packageName,
      `${nativePackage.packageName} name`
    );
    assertEquals(packageJson.private, true, `${nativePackage.packageName} private`);
    assertEquals(
      packageJson.os,
      [nativePackage.platform],
      `${nativePackage.packageName} os`
    );
    assertEquals(
      packageJson.cpu,
      [nativePackage.arch],
      `${nativePackage.packageName} cpu`
    );
    assertEquals(
      packageJson.files,
      [nativePackage.binaryPath],
      `${nativePackage.packageName} files`
    );
    assertEquals(
      packageJson.bin,
      { "klint-rs": nativePackage.binaryPath },
      `${nativePackage.packageName} bin`
    );
    assertEquals(list, expectedFiles, `${nativePackage.packageName} packed files`);
  }

  process.stdout.write(
    `pack:check ok (${mainList.length} main files, ${nativePackages().length} native packages)\n`
  );
} finally {
  rmSync(outDir, { recursive: true, force: true });
}

function packAndList(cwd: string): string[] {
  const packed = run(
    "bun",
    ["pm", "pack", "--ignore-scripts", "--destination", outDir, "--quiet"],
    cwd
  );
  const tarball = packed.startsWith("/") ? packed : join(outDir, packed);
  return run("tar", ["-tzf", tarball], cwd).split("\n").filter(Boolean).sort();
}

function run(command: string, args: string[], cwd: string): string {
  const result = spawnSync(command, args, {
    cwd,
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

function assertEquals(actual: unknown, expected: unknown, label: string): void {
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    fail(
      `pack:check ${label} mismatch:\nexpected ${JSON.stringify(expected)}\nactual ${JSON.stringify(actual)}`
    );
  }
}

function fail(message: string): never {
  process.stderr.write(`${message}\n`);
  process.exit(1);
}
