import { beforeAll, describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
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

  test("--engine rust applies architecture pattern rules to Swift files", () => {
    const dir = setupNamedFixture(
      `
include: ["Sources"]
rules: {}
arch:
  forbidden:
    - pattern: "URLSession.shared"
      in: "Sources/App/UI/**"
      message: "Use networking client"
  singleton:
    - pattern: "ProcessInfo.processInfo.environment[\\"API_KEY\\"]"
      only: "Sources/App/Config/AppConfig.swift"
      in: "Sources/**"
      message: "Use AppConfig"
`,
      {
        "Sources/App/Config/AppConfig.swift":
          'let key = ProcessInfo.processInfo.environment["API_KEY"]\n',
        "Sources/App/UI/ViewModel.swift":
          'let session = URLSession.shared\nlet key = ProcessInfo.processInfo.environment["API_KEY"]\n',
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
          file: "Sources/App/UI/ViewModel.swift",
          line: 1,
          rule: "arch/forbidden",
          message: "Use networking client",
          severity: "error",
          fix: null,
        },
        {
          file: "Sources/App/UI/ViewModel.swift",
          line: 2,
          rule: "arch/singleton",
          message: "Use AppConfig",
          severity: "error",
          fix: null,
        },
      ]);
      expect(rust.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine rust applies architecture import rules to Swift module imports", () => {
    const dir = setupNamedFixture(
      `
include: ["Sources"]
rules: {}
arch:
  layers:
    ui: ["Sources/App/UI/**"]
    core: ["Sources/App/Core/**"]
  imports:
    - from: ui
      deny: core
      message: "UI must not import core directly"
`,
      {
        "Sources/App/UI/ViewModel.swift": "import Foundation\nimport Core\n",
        "Sources/App/Core/Auth.swift": "public struct Auth {}\n",
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
          file: "Sources/App/UI/ViewModel.swift",
          line: 2,
          rule: "arch/imports",
          message: "UI must not import core directly",
          severity: "error",
          fix: null,
        },
      ]);
      expect(rust.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine rust ignores Swift imports written inside comments", () => {
    const dir = setupNamedFixture(
      `
include: ["Sources"]
rules: {}
arch:
  layers:
    ui: ["Sources/App/UI/**"]
    core: ["Sources/App/Core/**"]
  imports:
    - from: ui
      deny: core
      message: "UI must not import core directly"
`,
      {
        "Sources/App/UI/ViewModel.swift":
          "/*\nimport Core\n*/\n// import Core\n/* outer /* inner import Core */ still */\nimport Foundation\n",
        "Sources/App/Core/Auth.swift": "public struct Auth {}\n",
      }
    );

    try {
      const rust = runCliArgs(dir, ["--engine", "rust", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });
      const payload = parseJson(rust) as {
        violations: unknown[];
        summary: { errors: number; warnings: number };
      };

      expect(payload.violations).toEqual([]);
      expect(payload.summary).toEqual({ errors: 0, warnings: 0 });
      expect(rust.code).toBe(0);
      expect(rust.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine rust reads Swift imports carrying attributes and submodule paths", () => {
    const dir = setupNamedFixture(
      `
include: ["Sources"]
rules: {}
arch:
  layers:
    ui: ["Sources/App/UI/**"]
    core: ["Sources/App/Core/**"]
  imports:
    - from: ui
      deny: core
      message: "UI must not import core directly"
`,
      {
        "Sources/App/UI/ViewModel.swift":
          "@_exported import Core\n@testable import Core\nimport struct Core.Auth\nimport Core.Session\n",
        "Sources/App/Core/Auth.swift": "public struct Auth {}\n",
      }
    );

    try {
      const rust = runCliArgs(dir, ["--engine", "rust", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });
      const payload = parseJson(rust) as {
        violations: Array<{ file: string; line: number; rule: string }>;
        summary: { errors: number; warnings: number };
      };

      expect(payload.violations.map((violation) => violation.line)).toEqual([1, 2, 3, 4]);
      expect(payload.summary).toEqual({ errors: 4, warnings: 0 });
      expect(rust.code).toBe(2);
      expect(rust.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine rust counts Swift block comments toward maxCommentBlock", () => {
    const dir = setupNamedFixture(
      `
include: ["Sources"]
rules: {}
arch:
  maxCommentBlock:
    - limit: 2
      in: "Sources/**"
`,
      {
        "Sources/App/UI/ViewModel.swift":
          "/*\n * block\n * block\n * block\n */\npublic struct Widget {}\n",
      }
    );

    try {
      const rust = runCliArgs(dir, ["--engine", "rust", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });
      const payload = parseJson(rust) as {
        violations: Array<{ file: string; line: number; rule: string }>;
        summary: { errors: number; warnings: number };
      };

      expect(payload.violations).toHaveLength(1);
      expect(payload.violations[0]?.rule).toBe("arch/max-comment-block");
      expect(payload.violations[0]?.file).toBe("Sources/App/UI/ViewModel.swift");
      expect(rust.code).toBe(2);
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

  test("--engine rust checks every target of a multi-target Python import", () => {
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
        "src/app/jobs/worker.py":
          "import json, app.lib.auth\nfrom . import helper, sibling\n",
        "src/app/jobs/helper.py": "value = 1\n",
        "src/app/jobs/sibling.py": "value = 2\n",
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
          line: 1,
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

  test("--engine rust applies architecture import rules to Python dynamic imports", () => {
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
        "src/app/jobs/worker.py":
          'import importlib\n\n\ndef load():\n    return importlib.import_module("app.lib.auth")\n',
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
          line: 5,
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

  test("--engine rust resolves Python namespace packages that carry no __init__.py", () => {
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
        "src/app/jobs/worker.py": "from app.lib import auth\nimport app.lib\n",
        "src/app/lib/auth.py": "class Key:\n    pass\n",
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
      expect(payload.violations.map((violation) => violation.line)).toEqual([1, 2]);
      expect(rust.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine rust exempts Python TYPE_CHECKING imports under type-only allow", () => {
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
      type-only: allow
      message: "Jobs must not import lib directly"
`,
      {
        "src/app/jobs/worker.py":
          "from typing import TYPE_CHECKING\n\nif TYPE_CHECKING:\n    from app.lib.auth import Key\nelse:\n    from app.lib.auth import load_key\n",
        "src/app/lib/auth.py":
          "class Key:\n    pass\n\n\ndef load_key():\n    return 'x'\n",
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
          line: 6,
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

  test("--engine rust blocks denied Python packages and stdlib modules", () => {
    const dir = setupNamedFixture(
      `
include: ["src"]
rules: {}
arch:
  layers:
    jobs: ["src/app/jobs/**"]
  imports:
    - from: jobs
      deny-packages: ["requests", "os"]
      message: "Jobs must go through the http client and config"
`,
      {
        "src/app/jobs/worker.py":
          'import requests\nimport os.path\nfrom requests.adapters import HTTPAdapter\nimport requests_mock\nimport oscrypto\nfrom . import helper\nimport importlib\nlazy = importlib.import_module("requests.sessions")\n',
        "src/app/jobs/helper.py": "value = 1\n",
      }
    );

    try {
      const rust = runCliArgs(dir, ["--engine", "rust", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });
      const payload = parseJson(rust) as {
        violations: Array<{ file: string; line: number; message: string }>;
        summary: { errors: number; warnings: number };
      };

      expect(rust.code).toBe(2);
      expect(payload.summary).toEqual({ errors: 4, warnings: 0 });
      expect(payload.violations.map((violation) => violation.line)).toEqual([1, 2, 3, 8]);
      expect(rust.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine rust blocks denied Swift system frameworks", () => {
    const dir = setupNamedFixture(
      `
include: ["Sources"]
rules: {}
arch:
  layers:
    core: ["Sources/App/Core/**"]
  imports:
    - from: core
      deny-packages: ["UIKit", "Foundation"]
      message: "Core must stay free of UI and platform frameworks"
`,
      {
        "Sources/App/Core/Auth.swift":
          "import UIKit\nimport Foundation\nimport SwiftUI\nimport UIKitten\nimport Session\n",
        "Sources/App/Core/Session.swift": "public struct Session {}\n",
      }
    );

    try {
      const rust = runCliArgs(dir, ["--engine", "rust", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });
      const payload = parseJson(rust) as {
        violations: Array<{ file: string; line: number; message: string }>;
        summary: { errors: number; warnings: number };
      };

      expect(rust.code).toBe(2);
      expect(payload.summary).toEqual({ errors: 2, warnings: 0 });
      expect(payload.violations.map((violation) => violation.line)).toEqual([1, 2]);
      expect(rust.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine rust applies architecture import rules to Rust use declarations", () => {
    const dir = setupNamedFixture(
      `
include: ["crates"]
rules: {}
arch:
  layers:
    syntax: ["crates/app/src/syntax/**"]
    output: ["crates/app/src/output.rs"]
  imports:
    - from: syntax
      deny: output
      message: "Syntax must not depend on output"
`,
      {
        "crates/app/src/lib.rs": "mod output;\npub mod syntax;\n",
        "crates/app/src/output.rs": "pub struct Violation;\n",
        "crates/app/src/syntax/mod.rs":
          "use crate::output::Violation;\n// use crate::output::Ignored;\nuse std::fs;\n",
      }
    );

    try {
      const rust = runCliArgs(dir, ["--engine", "rust", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });
      const payload = parseJson(rust) as {
        violations: Array<{ file: string; line: number; message: string }>;
        summary: { errors: number; warnings: number };
      };

      expect(rust.code).toBe(2);
      expect(payload.summary).toEqual({ errors: 1, warnings: 0 });
      expect(payload.violations[0]?.file).toBe("crates/app/src/syntax/mod.rs");
      expect(payload.violations[0]?.line).toBe(1);
      expect(rust.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine rust checks every target of a braced Rust use list", () => {
    const dir = setupNamedFixture(
      `
include: ["crates"]
rules: {}
arch:
  layers:
    cli: ["crates/app/src/cli.rs"]
    core: ["crates/app/src/core/**"]
  imports:
    - from: cli
      deny: core
      message: "CLI must not reach into core internals"
`,
      {
        "crates/app/src/lib.rs": "mod cli;\nmod core;\n",
        "crates/app/src/core/mod.rs": "pub mod parse;\npub mod render;\n",
        "crates/app/src/core/parse.rs": "pub fn parse() {}\n",
        "crates/app/src/core/render.rs": "pub fn render() {}\n",
        "crates/app/src/cli.rs":
          "use crate::core::{parse::parse, render::render};\nuse std::io;\n",
      }
    );

    try {
      const rust = runCliArgs(dir, ["--engine", "rust", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });
      const payload = parseJson(rust) as {
        violations: Array<{ file: string; line: number }>;
        summary: { errors: number; warnings: number };
      };

      expect(rust.code).toBe(2);
      expect(payload.summary).toEqual({ errors: 2, warnings: 0 });
      expect(payload.violations.map((violation) => violation.line)).toEqual([1, 1]);
      expect(rust.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine rust resolves self and super Rust module paths", () => {
    const dir = setupNamedFixture(
      `
include: ["crates"]
rules: {}
arch:
  layers:
    leaf: ["crates/app/src/feature/leaf.rs"]
    banned: ["crates/app/src/banned.rs", "crates/app/src/feature/helper.rs"]
  imports:
    - from: leaf
      deny: banned
      message: "Leaf must not reach these modules"
`,
      {
        "crates/app/src/lib.rs": "mod banned;\nmod feature;\n",
        "crates/app/src/banned.rs": "pub fn nope() {}\n",
        "crates/app/src/feature/mod.rs": "pub mod helper;\npub mod leaf;\n",
        "crates/app/src/feature/helper.rs": "pub fn help() {}\n",
        "crates/app/src/feature/leaf.rs":
          "use super::helper::help;\nuse crate::banned::nope;\nuse std::fmt;\n",
      }
    );

    try {
      const rust = runCliArgs(dir, ["--engine", "rust", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });
      const payload = parseJson(rust) as {
        violations: Array<{ line: number }>;
        summary: { errors: number; warnings: number };
      };

      expect(rust.code).toBe(2);
      expect(payload.summary).toEqual({ errors: 2, warnings: 0 });
      expect(payload.violations.map((violation) => violation.line)).toEqual([1, 2]);
      expect(rust.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine rust blocks denied external Rust crates", () => {
    const dir = setupNamedFixture(
      `
include: ["crates"]
rules: {}
arch:
  layers:
    core: ["crates/app/src/core.rs"]
  imports:
    - from: core
      deny-packages: ["tokio", "std::process"]
      message: "Core must stay runtime-free and must not spawn processes"
`,
      {
        "crates/app/src/lib.rs": "mod core;\n",
        "crates/app/src/core.rs":
          "use tokio::runtime::Runtime;\nuse std::process::Command;\nuse std::fs;\nuse tokiox::thing;\n",
      }
    );

    try {
      const rust = runCliArgs(dir, ["--engine", "rust", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });
      const payload = parseJson(rust) as {
        violations: Array<{ line: number }>;
        summary: { errors: number; warnings: number };
      };

      expect(rust.code).toBe(2);
      expect(payload.summary).toEqual({ errors: 2, warnings: 0 });
      expect(payload.violations.map((violation) => violation.line)).toEqual([1, 2]);
      expect(rust.stderr).toBe("");
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("--engine rust counts Rust comment kinds and exempts doc-comments", () => {
    const dir = setupNamedFixture(
      `
include: ["crates"]
rules: {}
arch:
  maxCommentBlock:
    - limit: 2
      in: "crates/**"
`,
      {
        "crates/app/src/documented.rs":
          "/// doc\n/// doc\n/// doc\n//! doc\npub fn kept() {}\n",
        "crates/app/src/noisy.rs": "/*\n * block\n * block\n */\npub fn noisy() {}\n",
      }
    );

    try {
      const rust = runCliArgs(dir, ["--engine", "rust", "--json"], {
        KLINT_RUST_BIN: rustBin,
      });
      const payload = parseJson(rust) as {
        violations: Array<{ file: string; rule: string }>;
        summary: { errors: number; warnings: number };
      };

      expect(payload.violations).toHaveLength(1);
      expect(payload.violations[0]?.file).toBe("crates/app/src/noisy.rs");
      expect(payload.violations[0]?.rule).toBe("arch/max-comment-block");
      expect(rust.code).toBe(2);
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

  test("--engine compare honors file and directory include exclusions", () => {
    const dir = setupNamedFixture(
      `
include: ["src", "!**/*.generated.ts", "!src/lib/legacy.ts", "!**/vendor/**"]
rules: {}
arch:
  forbidden:
    - pattern: "console.log("
      in: "src/**"
      message: "Use logger"
`,
      {
        "src/lib/service.ts": `console.log("keep");\n`,
        "src/lib/legacy.ts": `console.log("excluded by path");\n`,
        "src/lib/schema.generated.ts": `console.log("excluded by glob");\n`,
        "src/vendor/dep.ts": `console.log("excluded by directory");\n`,
      }
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

      const payload = parseJson(compare) as {
        violations: Array<{ file: string }>;
      };
      expect(payload.violations.map((v) => v.file)).toEqual(["src/lib/service.ts"]);
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

describe("KLINT_ENGINE=rust — comment budget rules", () => {
  beforeAll(() => {
    rustBin = ensureRustBinary();
  }, RUST_BUILD_TIMEOUT_MS);

  test("matches TypeScript for density and block rules, exempting doc-comments", () => {
    const dir = setupNamedFixture(
      `
include: ["src"]
rules: {}
arch:
  maxCommentDensity:
    - limit: 20
      in: "src/**"
  maxCommentBlock:
    - limit: 2
      in: "src/**"
`,
      {
        "src/noisy.ts": "// a\n// b\n// c\nconst x = 1;\nconst y = 2;\n",
        "src/documented.ts": "/**\n * doc\n * doc\n * doc\n */\nexport const z = 3;\n",
        "src/inline.ts": "const a = 1; // one\nconst b = 2; // two\nconst c = 3;\n",
      }
    );

    try {
      const ts = runCli(dir);
      const rust = runCli(dir, { KLINT_ENGINE: "rust", KLINT_RUST_BIN: rustBin });

      expect(rust.code).toBe(2);
      expect(rust.code).toBe(ts.code);
      expect(parseJson(rust)).toEqual(parseJson(ts));
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("countDocComments: true makes both engines count doc-comments", () => {
    const dir = setupNamedFixture(
      `
include: ["src"]
rules: {}
arch:
  maxCommentBlock:
    - limit: 2
      countDocComments: true
      in: "src/**"
`,
      { "src/documented.ts": "/**\n * doc\n * doc\n * doc\n */\nexport const z = 3;\n" }
    );

    try {
      const ts = runCli(dir);
      const rust = runCli(dir, { KLINT_ENGINE: "rust", KLINT_RUST_BIN: rustBin });

      expect(rust.code).toBe(2);
      expect(rust.code).toBe(ts.code);
      expect(parseJson(rust)).toEqual(parseJson(ts));
    } finally {
      rmSync(dir, { recursive: true });
    }
  });
});

describe("KLINT_ENGINE=rust — schema version advisory", () => {
  const taggedUrl = (version: string) =>
    `https://raw.githubusercontent.com/konvert7/klint/refs/tags/v${version}/klint.schema.json`;

  const archConfig = (schema: string) => `
$schema: ${schema}
include: ["src"]
rules: {}
arch:
  maxLines:
    - limit: 500
      in: "src/**"
`;

  const installedVersion = (): string =>
    JSON.parse(readFileSync(join(ROOT, "package.json"), "utf-8")).version;

  test("warns on a stale schema version without failing the run", () => {
    const dir = setupNamedFixture(archConfig(taggedUrl("0.0.1")), {
      "src/subject.ts": "export const a = 1;\n",
    });

    try {
      const result = runCli(dir);
      const payload = parseJson(result) as {
        violations: Array<{ file: string; rule: string; severity: string; line: number }>;
        summary: { errors: number; warnings: number };
      };

      expect(result.code).toBe(0);
      expect(payload.summary).toEqual({ errors: 0, warnings: 1 });
      expect(payload.violations).toHaveLength(1);
      expect(payload.violations[0].rule).toBe("klint/schema-version");
      expect(payload.violations[0].severity).toBe("warn");
      expect(payload.violations[0].file).toBe("klint.yaml");
      expect(payload.violations[0].line).toBe(2);
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("stays silent when the declared version matches the installed one", () => {
    const dir = setupNamedFixture(archConfig(taggedUrl(installedVersion())), {
      "src/subject.ts": "export const a = 1;\n",
    });

    try {
      const result = runCli(dir);
      expect(result.code).toBe(0);
      expect(parseJson(result)).toEqual({
        violations: [],
        summary: { errors: 0, warnings: 0 },
      });
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("stays silent for a local schema path", () => {
    const dir = setupNamedFixture(archConfig("./klint.schema.json"), {
      "src/subject.ts": "export const a = 1;\n",
    });

    try {
      const result = runCli(dir);
      expect(result.code).toBe(0);
      expect(parseJson(result)).toEqual({
        violations: [],
        summary: { errors: 0, warnings: 0 },
      });
    } finally {
      rmSync(dir, { recursive: true });
    }
  });

  test("reaches every engine exactly once and keeps compare green", () => {
    const dir = setupNamedFixture(archConfig(taggedUrl("0.0.1")), {
      "src/subject.ts": "export const a = 1;\n",
    });

    try {
      for (const engine of ["ts", "rust", "auto", "compare"]) {
        const result = runCliArgs(dir, ["--json", "--engine", engine], {
          KLINT_RUST_BIN: rustBin,
        });
        const payload = parseJson(result) as {
          violations: Array<{ rule: string }>;
          summary: { errors: number; warnings: number };
        };

        expect(result.code).toBe(0);
        expect(payload.violations.map((v) => v.rule)).toEqual(["klint/schema-version"]);
        expect(payload.summary).toEqual({ errors: 0, warnings: 1 });
      }
    } finally {
      rmSync(dir, { recursive: true });
    }
  });
});
