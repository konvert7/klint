import { spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { arch, platform, tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const csprojPath = join(root, "nuget", "Klint.Tool.csproj");
const version = readCsprojVersion();
const rid = hostRid();
const binaryName = platform() === "win32" ? "klint-rs.exe" : "klint-rs";
const stagedBinary = join(root, "nuget", "binaries", rid, binaryName);
const workDir = mkdtempSync(join(tmpdir(), "klint-nuget-tool-"));
const nupkgDir = join(workDir, "nupkg");
const toolPath = join(workDir, "tools");

if (!existsSync(stagedBinary)) {
  fail(
    `Missing staged binary for ${rid}: ${stagedBinary}\n` +
      `Run: bun tests/stage-nuget-binary.ts --rid ${rid} --source target/release/${binaryName}`
  );
}

try {
  run("dotnet", ["pack", csprojPath, "-c", "Release", "-o", nupkgDir]);
  run("dotnet", [
    "tool",
    "install",
    "Konvert7.Klint",
    "--tool-path",
    toolPath,
    "--add-source",
    nupkgDir,
    "--version",
    version,
  ]);

  const toolBinary = join(toolPath, platform() === "win32" ? "klint.exe" : "klint");
  const result = run(toolBinary, ["--version"]);
  if (!result.stdout.trim()) {
    fail("klint --version produced no output");
  }
  process.stdout.write(
    `smoke passed: klint ${version} (${rid}) -> ${result.stdout.trim()}\n`
  );
} finally {
  rmSync(workDir, { recursive: true, force: true });
}

function run(command: string, args: string[]): { stdout: string } {
  const result = spawnSync(command, args, { cwd: root, encoding: "utf-8" });
  if (result.error) {
    fail(`${command} failed to start: ${result.error.message}`);
  }
  if ((result.status ?? -1) !== 0) {
    process.stderr.write(result.stdout ?? "");
    process.stderr.write(result.stderr ?? "");
    fail(`${command} ${args.join(" ")} exited with ${result.status ?? -1}`);
  }
  return { stdout: result.stdout ?? "" };
}

function readCsprojVersion(): string {
  const match = readFileSync(csprojPath, "utf-8").match(/<Version>(.+)<\/Version>/);
  if (!match) {
    fail(`No <Version> found in ${csprojPath}`);
  }
  return match[1];
}

function hostRid(): string {
  const cpu = arch() === "arm64" ? "arm64" : "x64";
  return `${hostOs()}-${cpu}`;
}

function hostOs(): string {
  if (platform() === "win32") return "win";
  if (platform() === "darwin") return "osx";
  return "linux";
}

function fail(message: string): never {
  process.stderr.write(`${message}\n`);
  process.exit(1);
}
