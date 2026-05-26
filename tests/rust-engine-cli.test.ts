import { beforeAll, describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const CLI = resolve(import.meta.dir, "../cli.ts");
const ROOT = resolve(import.meta.dir, "..");
const RUST_BUILD_TIMEOUT_MS = 120000;
let rustBin: string;

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

function runCliArgs(
  dir: string,
  args: string[],
  env: Record<string, string> = {}
): CliResult {
  const result = spawnSync("bun", [CLI, "--config", dir, ...args], {
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

function runCliText(dir: string, env: Record<string, string> = {}): CliResult {
  const result = spawnSync("bun", [CLI, "--config", dir], {
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
    timeout: RUST_BUILD_TIMEOUT_MS,
  });
  expect(result.status, result.stderr || result.stdout).toBe(0);
  expect(existsSync(bin)).toBe(true);
  return bin;
}

describe("KLINT_ENGINE=rust", () => {
  beforeAll(() => {
    rustBin = ensureRustBinary();
  }, RUST_BUILD_TIMEOUT_MS);

  test("matches TypeScript JSON output and exit code for arch errors", () => {
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
        KLINT_RUST_BIN: rustBin,
      });

      expect(rust.code).toBe(2);
      expect(rust.code).toBe(ts.code);
      expect(parseJson(rust)).toEqual(parseJson(ts));
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("matches TypeScript JSON output and exit code for warning-only arch runs", () => {
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
        KLINT_RUST_BIN: rustBin,
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
        KLINT_RUST_BIN: rustBin,
      });

      expect(rust.code).toBe(2);
      expect(rust.code).toBe(ts.code);
      expect(parseJson(rust)).toEqual(parseJson(ts));
      expect(rust.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine rust matches the KLINT_ENGINE=rust path", () => {
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
      const envRust = runCli(dir, {
        KLINT_ENGINE: "rust",
        KLINT_RUST_BIN: rustBin,
      });
      const flagRust = runCliArgs(dir, ["--engine", "rust", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });

      expect(flagRust.code).toBe(2);
      expect(flagRust.code).toBe(envRust.code);
      expect(parseJson(flagRust)).toEqual(parseJson(envRust));
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine ts uses the TypeScript engine even when KLINT_ENGINE=rust is set", () => {
    const dir = setupFixture(
      `
include: ["src"]
rules:
  no-string-match: error
`,
      `const hit = "abc".match(/a/);\n`
    );

    try {
      const result = runCliArgs(dir, ["--engine", "ts", "--json"], {
        KLINT_ENGINE: "rust",
        KLINT_RUST_BIN: rustBin,
      });

      expect(result.code).toBe(2);
      expect(parseJson(result)).toMatchObject({
        summary: { errors: 1, warnings: 0 },
      });
      expect(result.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("rejects unknown engine names", () => {
    const dir = setupFixture(
      `
include: ["src"]
rules: {}
`,
      `export const value = 1;\n`
    );

    try {
      const result = runCliArgs(dir, ["--engine", "go", "--json"]);

      expect(result.code).toBe(1);
      expect(result.stderr).toContain('unknown engine "go"');
      expect(result.stdout).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("normal CLI stays on TypeScript engine even when a native package exists", () => {
    const dir = setupFixture(
      `
include: ["src"]
rules: {}
`,
      `export const value = 1;\n`
    );

    try {
      const result = runCliText(dir);

      expect(result.code).toBe(0);
      expect(result.stdout).toContain("klint: 0 violations");
      expect(result.stderr).not.toContain("KLINT_ENGINE=rust");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });
});
