import { describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const CLI = resolve(import.meta.dir, "../cli.ts");
const ROOT = resolve(import.meta.dir, "..");

interface CliResult {
  stdout: string;
  stderr: string;
  code: number;
}

function runCli(dir: string, env: Record<string, string> = {}): CliResult {
  const result = spawnSync("bun", [CLI, "--config", dir, "--json"], {
    encoding: "utf-8",
    env: { ...process.env, ...env },
    timeout: 30000,
  });
  return {
    stdout: result.stdout ?? "",
    stderr: result.stderr ?? "",
    code: result.status ?? -1,
  };
}

function setupFixture(config: string, source: string): string {
  const dir = mkdtempSync(join(tmpdir(), "klint-rust-engine-"));
  mkdirSync(join(dir, "src"));
  writeFileSync(join(dir, "klint.yaml"), config);
  writeFileSync(join(dir, "src", "subject.ts"), source);
  return dir;
}

function parseJson(result: CliResult): unknown {
  return JSON.parse(result.stdout);
}

function rustBinPath(): string {
  return join(
    ROOT,
    "target",
    "debug",
    process.platform === "win32" ? "klint-rs.exe" : "klint-rs"
  );
}

function ensureRustBinary(): string {
  const bin = rustBinPath();
  if (existsSync(bin)) return bin;

  const result = spawnSync("cargo", ["build", "-p", "klint-rs"], {
    cwd: ROOT,
    encoding: "utf-8",
    timeout: 30000,
  });
  expect(result.status, result.stderr || result.stdout).toBe(0);
  expect(existsSync(bin)).toBe(true);
  return bin;
}

describe("KLINT_ENGINE=rust", () => {
  test("matches TypeScript JSON output and exit code for arch errors", () => {
    const bin = ensureRustBinary();
    const dir = setupFixture(
      `
include: ["src"]
rules: {}
arch:
  forbidden:
    - pattern: "console.log("
      in: "src/**"
      message: "Use logger"
`,
      `console.log("x");\n`
    );

    try {
      const ts = runCli(dir);
      const rust = runCli(dir, {
        KLINT_ENGINE: "rust",
        KLINT_RUST_BIN: bin,
      });

      expect(rust.code).toBe(2);
      expect(rust.code).toBe(ts.code);
      expect(parseJson(rust)).toEqual(parseJson(ts));
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("matches TypeScript JSON output and exit code for warning-only arch runs", () => {
    const bin = ensureRustBinary();
    const dir = setupFixture(
      `
include: ["src"]
rules: {}
arch:
  forbidden:
    - pattern: "console.log("
      in: "src/**"
      message: "Use logger"
      severity: warn
`,
      `console.log("x");\n`
    );

    try {
      const ts = runCli(dir);
      const rust = runCli(dir, {
        KLINT_ENGINE: "rust",
        KLINT_RUST_BIN: bin,
      });

      expect(rust.code).toBe(0);
      expect(rust.code).toBe(ts.code);
      expect(parseJson(rust)).toEqual(parseJson(ts));
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("refuses TypeScript-only configs instead of silently skipping rules", () => {
    const dir = setupFixture(
      `
include: ["src"]
rules:
  no-floating-promise: error
`,
      `async function run() {}\nrun();\n`
    );

    try {
      const rust = runCli(dir, { KLINT_ENGINE: "rust" });

      expect(rust.code).toBe(1);
      expect(rust.stderr).toContain("KLINT_ENGINE=rust requires an arch config");
      expect(rust.stdout).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("respects explicit KLINT_RUST_BIN override", () => {
    const bin = ensureRustBinary();
    const dir = setupFixture(
      `
include: ["src"]
rules: {}
arch:
  forbidden:
    - pattern: "console.log("
      in: "src/**"
      message: "Use logger"
`,
      `console.log("x");\n`
    );

    try {
      const ts = runCli(dir);
      const rust = runCli(dir, {
        KLINT_ENGINE: "rust",
        KLINT_RUST_BIN: bin,
      });

      expect(rust.code).toBe(2);
      expect(rust.code).toBe(ts.code);
      expect(parseJson(rust)).toEqual(parseJson(ts));
      expect(rust.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });
});
