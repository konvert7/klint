import { beforeAll, describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { formatStats } from "../cli/stats";
import { emptyStats, nextStats, readStats, runRecorder } from "../core/stats";
import type { Violation } from "../core/types";

const CLI = resolve(import.meta.dir, "../cli.ts");
const ROOT = resolve(import.meta.dir, "..");
const RUST_BUILD_TIMEOUT_MS = 120000;
const RUST_BIN = join(
  ROOT,
  "target",
  "debug",
  process.platform === "win32" ? "klint-rs.exe" : "klint-rs"
);

const CONFIG = `include: ["src/**/*.ts"]
rules: {}
arch:
  layers:
    app: ["src/**"]
  forbidden:
    - pattern: "debugger"
      in: app
      message: "Remove the debugger statement."
`;

function error(rule: string, file = "a.ts", message = "m"): Violation {
  return { file, line: 1, rule, message, severity: "error" };
}

function fixture(source: string): string {
  const dir = mkdtempSync(join(tmpdir(), "klint-stats-"));
  writeFileSync(join(dir, "klint.yaml"), CONFIG);
  mkdirSync(join(dir, "src"));
  writeFileSync(join(dir, "src", "a.ts"), source);
  return dir;
}

function klint(dir: string, args: string[], env: Record<string, string> = {}) {
  const { KLINT_NO_STATS: _inherited, ...base } = process.env;
  const result = spawnSync("bun", [CLI, "--config", dir, ...args], {
    encoding: "utf-8",
    env: { ...base, ...env },
    timeout: 30000,
  });
  return { stdout: result.stdout ?? "", code: result.status ?? -1 };
}

describe("nextStats", () => {
  test("an error gone on the next run counts as prevented by the run that caught it", () => {
    const flagged = nextStats(emptyStats(), [error("arch/imports")], "hook");
    const fixed = nextStats(flagged, [], "direct");

    expect(fixed.runs).toEqual({ hook: 1, direct: 1 });
    expect(fixed.prevented.byRule).toEqual({ "arch/imports": 1 });
    expect(fixed.prevented.bySource).toEqual({ hook: 1, direct: 0 });
    expect(fixed.open).toEqual([]);
  });

  test("an error still present is not counted again", () => {
    const first = nextStats(emptyStats(), [error("r")], "hook");
    const movedLine: Violation = { ...error("r"), line: 9 };
    const second = nextStats(first, [movedLine], "hook");

    expect(second.prevented.bySource).toEqual({ hook: 0, direct: 0 });
    expect(second.open).toHaveLength(1);
  });

  test("duplicate errors are matched one for one", () => {
    const first = nextStats(emptyStats(), [error("r"), error("r")], "direct");
    const second = nextStats(first, [error("r")], "direct");

    expect(second.prevented.byRule).toEqual({ r: 1 });
    expect(second.open).toHaveLength(1);
  });

  test("warnings are not tracked", () => {
    const stats = nextStats(
      emptyStats(),
      [{ ...error("r"), severity: "warn" }],
      "direct"
    );
    expect(stats.open).toEqual([]);
  });
});

describe("stats storage", () => {
  test("recording creates a .klint directory that git ignores entirely", () => {
    const dir = mkdtempSync(join(tmpdir(), "klint-stats-store-"));
    runRecorder(dir, "hook")([error("r")]);

    const ignore = readFileSync(join(dir, ".klint", ".gitignore"), "utf-8");
    expect(ignore.split("\n")).toContain("*");
    expect(readStats(dir).runs).toEqual({ hook: 1, direct: 0 });
    rmSync(dir, { recursive: true, force: true });
  });

  test("a corrupt stats file starts over", () => {
    const dir = mkdtempSync(join(tmpdir(), "klint-stats-corrupt-"));
    mkdirSync(join(dir, ".klint"));
    writeFileSync(join(dir, ".klint", "stats.json"), "{not json");

    expect(readStats(dir)).toEqual(emptyStats());
    rmSync(dir, { recursive: true, force: true });
  });

  test("report lists rules by count, then by name", () => {
    const stats = emptyStats();
    stats.runs = { hook: 3, direct: 1 };
    stats.prevented.byRule = { "b-rule": 1, "a-rule": 1, "arch/imports": 2 };
    stats.prevented.bySource = { hook: 3, direct: 1 };

    expect(formatStats(stats)).toBe(
      "klint stats (.klint/stats.json)\n\n" +
        "  runs              4  (hook 3, direct 1)\n" +
        "  errors prevented  4  (caught by hook 3, direct 1)\n\n" +
        "  prevented by rule\n" +
        "    arch/imports  2\n" +
        "    a-rule        1\n" +
        "    b-rule        1\n"
    );
  });
});

describe("klint stats CLI", () => {
  test("a hook run that fails, then a clean direct run, counts one prevented error", () => {
    const dir = fixture("debugger;\n");
    expect(klint(dir, ["--json", "--hook"]).code).toBe(2);
    writeFileSync(join(dir, "src", "a.ts"), "export const ok = 1;\n");
    expect(klint(dir, ["--json"]).code).toBe(0);

    const stats = readStats(dir);
    expect(stats.runs).toEqual({ hook: 1, direct: 1 });
    expect(stats.prevented.bySource).toEqual({ hook: 1, direct: 0 });
    expect(Object.values(stats.prevented.byRule)).toEqual([1]);

    const report = spawnSync("bun", [CLI, "stats", "--config", dir], {
      encoding: "utf-8",
    });
    expect(report.stdout).toContain("errors prevented  1  (caught by hook 1, direct 0)");
    rmSync(dir, { recursive: true, force: true });
  });

  test("KLINT_NO_STATS=1 records nothing", () => {
    const dir = fixture("debugger;\n");
    klint(dir, ["--json"], { KLINT_NO_STATS: "1" });

    expect(readStats(dir)).toEqual(emptyStats());
    rmSync(dir, { recursive: true, force: true });
  });
});

describe("stats across engines", () => {
  beforeAll(() => {
    const build = spawnSync("cargo", ["build", "-p", "klint-rs"], {
      cwd: ROOT,
      encoding: "utf-8",
      timeout: RUST_BUILD_TIMEOUT_MS,
    });
    expect(build.status, build.stderr || build.stdout).toBe(0);
  }, RUST_BUILD_TIMEOUT_MS);

  test("a TypeScript run delegating to Rust is counted once", () => {
    const dir = fixture("debugger;\n");
    klint(dir, ["--json", "--hook"], { KLINT_ENGINE: "rust", KLINT_RUST_BIN: RUST_BIN });

    expect(readStats(dir).runs).toEqual({ hook: 1, direct: 0 });
    rmSync(dir, { recursive: true, force: true });
  });

  test("the Rust binary records direct runs and prints the same report", () => {
    const dir = fixture("debugger;\n");
    klint(dir, ["--json", "--hook"]);
    writeFileSync(join(dir, "src", "a.ts"), "export const ok = 1;\n");
    const { KLINT_NO_STATS: _inherited, ...env } = process.env;
    spawnSync(RUST_BIN, ["--config", dir, "--json"], { encoding: "utf-8", env });

    const tsReport = spawnSync("bun", [CLI, "stats", "--config", dir], {
      encoding: "utf-8",
    });
    const rustReport = spawnSync(RUST_BIN, ["stats", "--config", dir], {
      encoding: "utf-8",
    });
    expect(readStats(dir).runs).toEqual({ hook: 1, direct: 1 });
    expect(rustReport.stdout).toBe(tsReport.stdout);
    expect(rustReport.stdout).toContain(
      "errors prevented  1  (caught by hook 1, direct 0)"
    );
    rmSync(dir, { recursive: true, force: true });
  });
});
