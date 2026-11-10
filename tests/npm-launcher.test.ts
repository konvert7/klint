import { afterEach, describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import {
  chmodSync,
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const repo = fileURLToPath(new URL("..", import.meta.url));
const binaryName = process.platform === "win32" ? "klint-rs.exe" : "klint-rs";
const builtBinary = join(repo, "target", "debug", binaryName);
const crateVersion = /^version = "([^"]+)"/m.exec(
  readFileSync(join(repo, "crates", "klint-rs", "Cargo.toml"), "utf-8")
)?.[1];

let sandbox: string | undefined;
afterEach(() => {
  if (sandbox) rmSync(sandbox, { recursive: true, force: true });
  sandbox = undefined;
});

function installedLauncher({
  withNativePackage,
}: {
  withNativePackage: boolean;
}): string {
  sandbox = mkdtempSync(join(tmpdir(), "klint-npm-launcher-"));
  const scope = join(sandbox, "node_modules", "@konvert7");
  const klint = join(scope, "klint");
  mkdirSync(join(klint, "bin"), { recursive: true });
  writeFileSync(
    join(klint, "package.json"),
    JSON.stringify({ name: "@konvert7/klint", type: "module" })
  );
  copyFileSync(join(repo, "bin", "klint.js"), join(klint, "bin", "klint.js"));

  if (withNativePackage) {
    const native = join(scope, `klint-${process.platform}-${process.arch}`);
    mkdirSync(join(native, "bin"), { recursive: true });
    writeFileSync(
      join(native, "package.json"),
      JSON.stringify({ name: `@konvert7/klint-${process.platform}-${process.arch}` })
    );
    copyFileSync(builtBinary, join(native, "bin", binaryName));
    chmodSync(join(native, "bin", binaryName), 0o755);
  }
  return join(klint, "bin", "klint.js");
}

function runNode(launcher: string, args: string[]) {
  return spawnSync("node", [launcher, ...args], {
    encoding: "utf-8",
    env: { ...process.env, KLINT_NO_STATS: "1" },
  });
}

describe("npm launcher", () => {
  test("runs the native binary of this platform", () => {
    const result = runNode(installedLauncher({ withNativePackage: true }), ["--version"]);

    expect(result.stdout).toBe(`klint ${crateVersion}\n`);
    expect(result.status).toBe(0);
  });

  test("passes arguments through and returns the binary's exit code", () => {
    const launcher = installedLauncher({ withNativePackage: true });
    const project = join(sandbox as string, "project");
    mkdirSync(join(project, "src"), { recursive: true });
    writeFileSync(
      join(project, "klint.yaml"),
      'include: ["src"]\nrules:\n  sonar/prefer-at: error\n'
    );
    writeFileSync(join(project, "src", "a.ts"), "const x = arr[arr.length - 1];\n");

    const result = runNode(launcher, ["--config", project, "--json"]);

    expect(JSON.parse(result.stdout).summary).toEqual({ errors: 1, warnings: 0 });
    expect(result.status).toBe(2);
  });

  test.skipIf(process.platform === "win32")(
    "runs the x64 binary on Windows on Arm",
    () => {
      const launcher = installedLauncher({ withNativePackage: false });
      const native = join(
        sandbox as string,
        "node_modules",
        "@konvert7",
        "klint-win32-x64"
      );
      mkdirSync(join(native, "bin"), { recursive: true });
      writeFileSync(
        join(native, "package.json"),
        JSON.stringify({ name: "@konvert7/klint-win32-x64" })
      );
      const fakeBinary = join(native, "bin", "klint-rs.exe");
      writeFileSync(fakeBinary, '#!/bin/sh\necho "win32-x64 binary: $*"\n');
      chmodSync(fakeBinary, 0o755);
      const windowsOnArm = `data:text/javascript,${encodeURIComponent(
        'Object.defineProperty(process, "platform", { value: "win32" });' +
          'Object.defineProperty(process, "arch", { value: "arm64" });'
      )}`;

      const result = spawnSync(
        "node",
        ["--import", windowsOnArm, launcher, "--version"],
        {
          encoding: "utf-8",
        }
      );

      expect(result.stdout).toBe("win32-x64 binary: --version\n");
      expect(result.status).toBe(0);
    }
  );

  test("explains a missing native package instead of crashing", () => {
    const result = runNode(installedLauncher({ withNativePackage: false }), [
      "--version",
    ]);

    expect(result.stderr).toContain(
      `@konvert7/klint-${process.platform}-${process.arch} is not installed`
    );
    expect(result.status).toBe(1);
  });
});
