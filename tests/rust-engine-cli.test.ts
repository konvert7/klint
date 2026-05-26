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

function setupNamedFixture(config: string, files: Record<string, string>): string {
  const dir = mkdtempSync(join(tmpdir(), "klint-rust-engine-"));
  writeFileSync(join(dir, "klint.yaml"), config);
  for (const [file, source] of Object.entries(files)) {
    const path = join(dir, file);
    mkdirSync(resolve(path, ".."), { recursive: true });
    writeFileSync(path, source);
  }
  return dir;
}

function parseJson(result: CliResult): unknown {
  return JSON.parse(result.stdout);
}

function sonarPluginSource(): string {
  return 'const r = /a[b]c/;\nconst last = items[items.length - 1];\nconst next = text.replace(/foo/g, repl);\nconst rx = new RegExp(`\\\\.foo`);\nconst path = "C:\\\\Users";\nif (value == null) value = fallback;\n';
}

function sonarPluginRules(): string[] {
  return [
    "sonar/no-single-char-class",
    "sonar/prefer-at",
    "sonar/prefer-nullish-coalescing-assign",
    "sonar/prefer-string-raw",
    "sonar/prefer-string-raw-regexp",
    "sonar/prefer-string-replaceall",
  ];
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
      expect(rust.stderr).toContain(
        "Rust engine currently supports arch rules and selected rules only"
      );
      expect(rust.stderr).toContain("- no-floating-promise");
      expect(rust.stdout).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("lists unsupported TypeScript rules for mixed configs", () => {
    const dir = setupFixture(
      `
include: ["src"]
rules:
  no-floating-promise: error
  no-string-match:
    severity: warn
  no-nested-template-literals: off
arch:
  forbidden:
    - pattern: "console.log("
      in: "src/**"
      message: "Use logger"
`,
      `console.log("x");\n`
    );

    try {
      const rust = runCli(dir, { KLINT_ENGINE: "rust" });

      expect(rust.code).toBe(1);
      expect(rust.stderr).toContain(
        "Rust engine currently supports arch rules and selected rules only"
      );
      expect(rust.stderr).toContain("Unsupported rules:");
      expect(rust.stderr).toContain("- no-floating-promise");
      expect(rust.stderr).not.toContain("- no-string-match");
      expect(rust.stderr).not.toContain("- no-nested-template-literals");
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

  test("--engine rust supports clean text output", () => {
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
      `export const value = 1;\n`
    );

    try {
      const result = runCliArgs(dir, ["--engine", "rust"], {
        KLINT_RUST_BIN: rustBin,
      });

      expect(result.code).toBe(0);
      expect(result.stdout).toContain("klint: 0 violations");
      expect(result.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine rust supports violation text output", () => {
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
      const result = runCliArgs(dir, ["--engine", "rust"], {
        KLINT_RUST_BIN: rustBin,
      });

      expect(result.code).toBe(2);
      expect(result.stdout).toBe("");
      expect(result.stderr).toContain("klint: 1 error(s)");
      expect(result.stderr).toContain("[arch/forbidden]");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine rust supports built-in sonar plugin defaults", () => {
    const dir = setupFixture(
      `
include: ["src"]
plugins: ["sonar"]
`,
      sonarPluginSource()
    );

    try {
      const rust = runCliArgs(dir, ["--engine", "rust", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });
      const payload = parseJson(rust) as {
        violations: Array<{ rule: string }>;
        summary: { errors: number; warnings: number };
      };

      expect(rust.code).toBe(2);
      expect(payload.summary).toEqual({ errors: 6, warnings: 0 });
      expect(payload.violations.map((violation) => violation.rule).sort()).toEqual(
        sonarPluginRules()
      );
      expect(rust.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine rust applies architecture pattern rules to Python files", () => {
    const dir = setupNamedFixture(
      `
include: ["src"]
rules: {}
arch:
  forbidden:
    - pattern: "print("
      in: "src/**"
      message: "Use logger"
  singleton:
    - pattern: "os.environ[\\"API_KEY\\"]"
      only: "src/lib/auth.py"
      in: "src/**"
      message: "Use auth module"
`,
      {
        "src/lib/auth.py": 'import os\nKEY = os.environ["API_KEY"]\n',
        "src/jobs/worker.py": 'import os\nprint("debug")\nKEY = os.environ["API_KEY"]\n',
      }
    );

    try {
      const rust = runCliArgs(dir, ["--engine", "rust", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });
      const payload = parseJson(rust) as {
        violations: Array<{
          file: string;
          line: number;
          rule: string;
          message: string;
          severity: string;
          fix: unknown;
        }>;
        summary: { errors: number; warnings: number };
      };

      expect(rust.code).toBe(2);
      expect(payload.summary).toEqual({ errors: 2, warnings: 0 });
      expect(payload.violations).toEqual([
        {
          file: "src/jobs/worker.py",
          line: 2,
          rule: "arch/forbidden",
          message: "Use logger",
          severity: "error",
          fix: null,
        },
        {
          file: "src/jobs/worker.py",
          line: 3,
          rule: "arch/singleton",
          message: "Use auth module",
          severity: "error",
          fix: null,
        },
      ]);
      expect(rust.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine rust applies architecture import rules to Python relative imports", () => {
    const dir = setupNamedFixture(
      `
include: ["src"]
rules: {}
arch:
  layers:
    jobs: ["src/jobs/**"]
    lib: ["src/lib/**"]
  imports:
    - from: jobs
      deny: lib
      message: "Jobs must not import lib directly"
`,
      {
        "src/jobs/worker.py": "import requests\nfrom ..lib.auth import load_key\n",
        "src/lib/auth.py": "def load_key():\n    return 'x'\n",
      }
    );

    try {
      const rust = runCliArgs(dir, ["--engine", "rust", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });
      const payload = parseJson(rust) as {
        violations: Array<{
          file: string;
          line: number;
          rule: string;
          message: string;
          severity: string;
          fix: unknown;
        }>;
        summary: { errors: number; warnings: number };
      };

      expect(rust.code).toBe(2);
      expect(payload.summary).toEqual({ errors: 1, warnings: 0 });
      expect(payload.violations).toEqual([
        {
          file: "src/jobs/worker.py",
          line: 2,
          rule: "arch/imports",
          message: "Jobs must not import lib directly",
          severity: "error",
          fix: null,
        },
      ]);
      expect(rust.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine rust applies architecture import rules to Python absolute imports", () => {
    const dir = setupNamedFixture(
      `
include: ["src"]
rules: {}
arch:
  layers:
    jobs: ["src/app/jobs/**"]
    lib: ["src/app/lib/**"]
  imports:
    - from: jobs
      deny: lib
      message: "Jobs must not import lib directly"
`,
      {
        "src/app/jobs/worker.py": "import requests\nfrom app.lib.auth import load_key\n",
        "src/app/lib/auth.py": "def load_key():\n    return 'x'\n",
      }
    );

    try {
      const rust = runCliArgs(dir, ["--engine", "rust", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });
      const payload = parseJson(rust) as {
        violations: Array<{
          file: string;
          line: number;
          rule: string;
          message: string;
          severity: string;
          fix: unknown;
        }>;
        summary: { errors: number; warnings: number };
      };

      expect(rust.code).toBe(2);
      expect(payload.summary).toEqual({ errors: 1, warnings: 0 });
      expect(payload.violations).toEqual([
        {
          file: "src/app/jobs/worker.py",
          line: 2,
          rule: "arch/imports",
          message: "Jobs must not import lib directly",
          severity: "error",
          fix: null,
        },
      ]);
      expect(rust.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine rust rejects unknown plugins", () => {
    const dir = setupFixture(
      `
include: ["src"]
plugins: ["unknown"]
`,
      `export const value = 1;\n`
    );

    try {
      const rust = runCliArgs(dir, ["--engine", "rust", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });

      expect(rust.code).toBe(1);
      expect(rust.stderr).toContain('Unknown klint plugin: "unknown"');
      expect(rust.stdout).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine compare emits TypeScript JSON when Rust matches", () => {
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
      const ts = runCliArgs(dir, ["--engine", "ts", "--json"]);
      const compare = runCliArgs(dir, ["--engine", "compare", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });

      expect(compare.code).toBe(2);
      expect(compare.code).toBe(ts.code);
      expect(parseJson(compare)).toEqual(parseJson(ts));
      expect(compare.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine compare supports no-string-match parity", () => {
    const dir = setupFixture(
      `
include: ["src"]
rules:
  no-string-match: error
`,
      `const hit = "abc".match(/a/);\nconst ok = "abc".match(/a/g);\n`
    );

    try {
      const ts = runCliArgs(dir, ["--engine", "ts", "--json"]);
      const compare = runCliArgs(dir, ["--engine", "compare", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });

      expect(compare.code).toBe(2);
      expect(compare.code).toBe(ts.code);
      expect(parseJson(compare)).toEqual(parseJson(ts));
      expect(compare.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine compare supports no-nested-template-literals parity", () => {
    const dir = setupFixture(
      `
include: ["src"]
rules:
  no-nested-template-literals: error
`,
      // biome-ignore lint/suspicious/noTemplateCurlyInString: intentional — string contains TS source code
      "declare const b: boolean;\nconst value = `${b ? `yes` : `no`}`;\n"
    );

    try {
      const ts = runCliArgs(dir, ["--engine", "ts", "--json"]);
      const compare = runCliArgs(dir, ["--engine", "compare", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });

      expect(compare.code).toBe(2);
      expect(compare.code).toBe(ts.code);
      expect(parseJson(compare)).toEqual(parseJson(ts));
      expect(compare.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine compare supports no-consecutive-array-push parity", () => {
    const dir = setupFixture(
      `
include: ["src"]
rules:
  no-consecutive-array-push: error
`,
      "const arr: number[] = [];\narr.push(1);\narr.push(2);\n"
    );

    try {
      const ts = runCliArgs(dir, ["--engine", "ts", "--json"]);
      const compare = runCliArgs(dir, ["--engine", "compare", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });

      expect(compare.code).toBe(2);
      expect(compare.code).toBe(ts.code);
      expect(parseJson(compare)).toEqual(parseJson(ts));
      expect(compare.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine compare supports no-unguarded-json-parse parity", () => {
    const dir = setupFixture(
      `
include: ["src"]
rules:
  no-unguarded-json-parse: error
`,
      "const value = JSON.parse(raw);\ntry {\n  JSON.parse(raw);\n} catch {}\n"
    );

    try {
      const ts = runCliArgs(dir, ["--engine", "ts", "--json"]);
      const compare = runCliArgs(dir, ["--engine", "compare", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });

      expect(compare.code).toBe(2);
      expect(compare.code).toBe(ts.code);
      expect(parseJson(compare)).toEqual(parseJson(ts));
      expect(compare.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine compare supports no-sync-in-async parity", () => {
    const dir = setupFixture(
      `
include: ["src"]
rules:
  no-sync-in-async: error
`,
      'import { readFileSync } from "node:fs";\nasync function load() {\n  readFileSync(path);\n}\nfunction ok() {\n  readFileSync(path);\n}\n'
    );

    try {
      const ts = runCliArgs(dir, ["--engine", "ts", "--json"]);
      const compare = runCliArgs(dir, ["--engine", "compare", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });

      expect(compare.code).toBe(2);
      expect(compare.code).toBe(ts.code);
      expect(parseJson(compare)).toEqual(parseJson(ts));
      expect(compare.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine compare supports sonar/no-single-char-class parity", () => {
    const dir = setupFixture(
      `
include: ["src"]
rules:
  sonar/no-single-char-class: error
`,
      `const r = /a[b]c/;\n`
    );

    try {
      const ts = runCliArgs(dir, ["--engine", "ts", "--json"]);
      const compare = runCliArgs(dir, ["--engine", "compare", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });

      expect(compare.code).toBe(2);
      expect(compare.code).toBe(ts.code);
      expect(parseJson(compare)).toEqual(parseJson(ts));
      expect(compare.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine compare supports sonar/prefer-at parity", () => {
    const dir = setupFixture(
      `
include: ["src"]
rules:
  sonar/prefer-at: error
`,
      `const last = items[items.length - 1];\n`
    );

    try {
      const ts = runCliArgs(dir, ["--engine", "ts", "--json"]);
      const compare = runCliArgs(dir, ["--engine", "compare", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });

      expect(compare.code).toBe(2);
      expect(compare.code).toBe(ts.code);
      expect(parseJson(compare)).toEqual(parseJson(ts));
      expect(compare.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine compare supports sonar/prefer-string-replaceall parity", () => {
    const dir = setupFixture(
      `
include: ["src"]
rules:
  sonar/prefer-string-replaceall: error
`,
      `const r = text.replace(/foo/g, repl);\n`
    );

    try {
      const ts = runCliArgs(dir, ["--engine", "ts", "--json"]);
      const compare = runCliArgs(dir, ["--engine", "compare", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });

      expect(compare.code).toBe(2);
      expect(compare.code).toBe(ts.code);
      expect(parseJson(compare)).toEqual(parseJson(ts));
      expect(compare.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine compare supports sonar/prefer-string-raw-regexp parity", () => {
    const dir = setupFixture(
      `
include: ["src"]
rules:
  sonar/prefer-string-raw-regexp: error
`,
      "const r = new RegExp(`\\\\.foo`);\n"
    );

    try {
      const ts = runCliArgs(dir, ["--engine", "ts", "--json"]);
      const compare = runCliArgs(dir, ["--engine", "compare", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });

      expect(compare.code).toBe(2);
      expect(compare.code).toBe(ts.code);
      expect(parseJson(compare)).toEqual(parseJson(ts));
      expect(compare.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine compare supports sonar/prefer-string-raw parity", () => {
    const dir = setupFixture(
      `
include: ["src"]
rules:
  sonar/prefer-string-raw: error
`,
      'const p = "C:\\\\Users";\n'
    );

    try {
      const ts = runCliArgs(dir, ["--engine", "ts", "--json"]);
      const compare = runCliArgs(dir, ["--engine", "compare", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });

      expect(compare.code).toBe(2);
      expect(compare.code).toBe(ts.code);
      expect(parseJson(compare)).toEqual(parseJson(ts));
      expect(compare.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine compare supports sonar/prefer-nullish-coalescing-assign parity", () => {
    const dir = setupFixture(
      `
include: ["src"]
rules:
  sonar/prefer-nullish-coalescing-assign: error
`,
      "let x: object | undefined;\nif (x == null) x = {};\nif (!y) y = {};\nif (z === null || z === undefined) { z = fallback; }\n"
    );

    try {
      const ts = runCliArgs(dir, ["--engine", "ts", "--json"]);
      const compare = runCliArgs(dir, ["--engine", "compare", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });

      expect(compare.code).toBe(2);
      expect(compare.code).toBe(ts.code);
      expect(parseJson(compare)).toEqual(parseJson(ts));
      expect(compare.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine compare supports built-in sonar plugin defaults", () => {
    const dir = setupFixture(
      `
include: ["src"]
plugins: ["sonar"]
`,
      sonarPluginSource()
    );

    try {
      const ts = runCliArgs(dir, ["--engine", "ts", "--json"]);
      const compare = runCliArgs(dir, ["--engine", "compare", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });

      expect(compare.code).toBe(2);
      expect(compare.code).toBe(ts.code);
      expect(parseJson(compare)).toEqual(parseJson(ts));
      expect(compare.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine compare rejects unknown plugins", () => {
    const dir = setupFixture(
      `
include: ["src"]
plugins: ["unknown"]
`,
      `export const value = 1;\n`
    );

    try {
      const compare = runCliArgs(dir, ["--engine", "compare", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });

      expect(compare.code).toBe(1);
      expect(compare.stderr).toContain('Unknown klint plugin: "unknown"');
      expect(compare.stdout).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine compare refuses configs Rust cannot verify", () => {
    const dir = setupFixture(
      `
include: ["src"]
rules:
  no-floating-promise: error
arch:
  forbidden:
    - pattern: "console.log("
      in: "src/**"
      message: "Use logger"
`,
      `console.log("x");\n`
    );

    try {
      const compare = runCliArgs(dir, ["--engine", "compare", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });

      expect(compare.code).toBe(1);
      expect(compare.stderr).toContain(
        "Rust engine currently supports arch rules and selected rules only"
      );
      expect(compare.stderr).toContain("- no-floating-promise");
      expect(compare.stdout).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine auto merges Rust-supported and TypeScript-only rules", () => {
    const dir = setupFixture(
      `
include: ["src"]
rules:
  no-string-match: error
  no-floating-promise: error
`,
      `async function load(): Promise<string> { return "ok"; }\nload();\nconst hit = "abc".match(/a/);\n`
    );

    try {
      const ts = runCliArgs(dir, ["--engine", "ts", "--json"]);
      const auto = runCliArgs(dir, ["--engine", "auto", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });
      const payload = parseJson(auto) as {
        violations: Array<{ rule: string }>;
        summary: { errors: number; warnings: number };
      };

      expect(auto.code).toBe(2);
      expect(auto.code).toBe(ts.code);
      expect(payload.summary).toEqual({ errors: 2, warnings: 0 });
      expect(payload.violations.map((violation) => violation.rule).sort()).toEqual([
        "no-floating-promise",
        "no-string-match",
      ]);
      expect(auto.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine auto splits sonar plugin defaults between Rust and TypeScript", () => {
    const dir = setupFixture(
      `
include: ["src"]
plugins: ["sonar"]
`,
      sonarPluginSource()
    );

    try {
      const ts = runCliArgs(dir, ["--engine", "ts", "--json"]);
      const auto = runCliArgs(dir, ["--engine", "auto", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });
      const payload = parseJson(auto) as {
        violations: Array<{ rule: string }>;
        summary: { errors: number; warnings: number };
      };

      expect(auto.code).toBe(2);
      expect(auto.code).toBe(ts.code);
      expect(payload.summary).toEqual({ errors: 6, warnings: 0 });
      expect(payload.violations.map((violation) => violation.rule).sort()).toEqual(
        sonarPluginRules()
      );
      expect(auto.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine auto merges architecture and TypeScript-only rules", () => {
    const dir = setupFixture(
      `
include: ["src"]
rules:
  no-floating-promise: error
arch:
  forbidden:
    - pattern: "console.log("
      in: "src/**"
      message: "Use logger"
`,
      `async function load(): Promise<string> { return "ok"; }\nload();\nconsole.log("x");\n`
    );

    try {
      const auto = runCliArgs(dir, ["--engine", "auto", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });
      const payload = parseJson(auto) as {
        violations: Array<{ rule: string }>;
        summary: { errors: number; warnings: number };
      };

      expect(auto.code).toBe(2);
      expect(payload.summary).toEqual({ errors: 2, warnings: 0 });
      expect(payload.violations.map((violation) => violation.rule).sort()).toEqual([
        "arch/forbidden",
        "no-floating-promise",
      ]);
      expect(auto.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine auto supports clean text output", () => {
    const dir = setupFixture(
      `
include: ["src"]
rules: {}
`,
      `export const value = 1;\n`
    );

    try {
      const result = runCliArgs(dir, ["--engine", "auto"]);

      expect(result.code).toBe(0);
      expect(result.stdout).toContain("klint: 0 violations");
      expect(result.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine auto supports violation text output", () => {
    const dir = setupFixture(
      `
include: ["src"]
rules:
  no-string-match: error
  no-floating-promise: error
`,
      `async function load(): Promise<string> { return "ok"; }\nload();\nconst hit = "abc".match(/a/);\n`
    );

    try {
      const result = runCliArgs(dir, ["--engine", "auto"], {
        KLINT_RUST_BIN: rustBin,
      });

      expect(result.code).toBe(2);
      expect(result.stdout).toBe("");
      expect(result.stderr).toContain("klint: 2 error(s)");
      expect(result.stderr).toContain("[no-floating-promise]");
      expect(result.stderr).toContain("[no-string-match]");
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
      expect(result.stderr).toContain('expected "ts", "rust", "compare", or "auto"');
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
