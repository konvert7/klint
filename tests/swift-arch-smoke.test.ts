import { describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { resolve } from "node:path";

const ROOT = resolve(import.meta.dir, "..");
const FIXTURE = resolve(import.meta.dir, "fixtures", "swift-arch");

describe("Swift architecture smoke fixture", () => {
  test("Rust CLI flags Swift project imports and pattern rules", () => {
    const result = spawnSync(
      "cargo",
      ["run", "--quiet", "-p", "klint-rs", "--", "--config", FIXTURE, "--json"],
      {
        cwd: ROOT,
        encoding: "utf-8",
        timeout: 120000,
      }
    );

    expect(result.status, result.stderr || result.stdout).toBe(2);
    expect(result.stderr).toBe("");

    const output = JSON.parse(result.stdout) as {
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

    expect(output.summary).toEqual({ errors: 3, warnings: 0 });
    expect(output.violations).toEqual([
      {
        file: "Sources/App/UI/ViewModel.swift",
        line: 2,
        rule: "arch/imports",
        message: "UI must not import Core directly",
        severity: "error",
        fix: null,
      },
      {
        file: "Sources/App/UI/ViewModel.swift",
        line: 5,
        rule: "arch/forbidden",
        message: "Use the networking client",
        severity: "error",
        fix: null,
      },
      {
        file: "Sources/App/UI/ViewModel.swift",
        line: 6,
        rule: "arch/singleton",
        message: "Use AppConfig",
        severity: "error",
        fix: null,
      },
    ]);
  });
});
