import { describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { KlintConfigSchema } from "../core/config.schema";
import { runKlint } from "../core/runner";
import type { ArchConfig } from "../core/types";

function parseMaxLines(limit: number) {
  return KlintConfigSchema.safeParse({
    include: ["."],
    rules: {},
    arch: { maxLines: [{ limit, in: "src/**" }] },
  });
}

function lint(arch: ArchConfig, files: { path: string[]; content: string }[]) {
  const root = mkdtempSync(join(tmpdir(), "klint-arch-maxlines-"));
  for (const f of files) {
    mkdirSync(join(root, ...f.path.slice(0, -1)), { recursive: true });
    writeFileSync(join(root, ...f.path), f.content);
  }
  const violations = runKlint({ root, include: ["."], rules: {}, arch }, {});
  rmSync(root, { recursive: true });
  return violations.filter((v) => v.rule === "arch/max-lines");
}

describe("arch/max-lines", () => {
  test("flags a file over the limit and reports line limit+1", () => {
    const v = lint({ maxLines: [{ limit: 2, in: "src/**" }] }, [
      { path: ["src", "big.ts"], content: "a\nb\nc\n" },
    ]);
    expect(v).toHaveLength(1);
    expect(v[0].line).toBe(3);
    expect(v[0].message).toBe("File exceeds the maximum of 2 lines");
    expect(v[0].severity).toBe("error");
  });

  test("does not flag a file exactly at the limit", () => {
    const v = lint({ maxLines: [{ limit: 2, in: "src/**" }] }, [
      { path: ["src", "small.ts"], content: "x\ny\n" },
    ]);
    expect(v).toHaveLength(0);
  });

  test("a trailing newline does not add a counted line (Rust .lines() parity)", () => {
    // "a\nb\nc" (no trailing newline) and "a\nb\nc\n" (trailing) both count as 3 lines.
    const withTrailing = lint({ maxLines: [{ limit: 3, in: "src/**" }] }, [
      { path: ["src", "a.ts"], content: "a\nb\nc\n" },
    ]);
    const withoutTrailing = lint({ maxLines: [{ limit: 3, in: "src/**" }] }, [
      { path: ["src", "a.ts"], content: "a\nb\nc" },
    ]);
    expect(withTrailing).toHaveLength(0);
    expect(withoutTrailing).toHaveLength(0);
  });

  test("respects in: scoping — files outside scope are ignored", () => {
    const v = lint({ maxLines: [{ limit: 1, in: "src/**" }] }, [
      { path: ["src", "over.ts"], content: "a\nb\nc\n" },
      { path: ["lib", "alsobig.ts"], content: "a\nb\nc\n" },
    ]);
    expect(v).toHaveLength(1);
    expect(v[0].file).toBe("src/over.ts");
  });

  test("supports a custom message", () => {
    const v = lint(
      { maxLines: [{ limit: 1, in: "src/**", message: "Split this module" }] },
      [{ path: ["src", "big.ts"], content: "a\nb\n" }]
    );
    expect(v).toHaveLength(1);
    expect(v[0].message).toBe("Split this module");
  });

  test("respects severity override", () => {
    const v = lint({ maxLines: [{ limit: 1, in: "src/**", severity: "warn" }] }, [
      { path: ["src", "big.ts"], content: "a\nb\n" },
    ]);
    expect(v).toHaveLength(1);
    expect(v[0].severity).toBe("warn");
  });

  test("allows different limits per scope", () => {
    const v = lint(
      {
        maxLines: [
          { limit: 2, in: "src/**" },
          { limit: 5, in: "tests/**" },
        ],
      },
      [
        { path: ["src", "a.ts"], content: "1\n2\n3\n" },
        { path: ["tests", "a.test.ts"], content: "1\n2\n3\n" },
      ]
    );
    expect(v).toHaveLength(1);
    expect(v[0].file).toBe("src/a.ts");
  });
});

describe("arch/max-lines — schema validation", () => {
  test("rejects a zero or negative limit, accepts a positive one", () => {
    expect(parseMaxLines(0).success).toBe(false);
    expect(parseMaxLines(-3).success).toBe(false);
    expect(parseMaxLines(2.5).success).toBe(false);
    expect(parseMaxLines(300).success).toBe(true);
  });
});
