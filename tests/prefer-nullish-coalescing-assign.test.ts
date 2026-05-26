import { describe, expect, test } from "bun:test";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { applyFixes } from "../core/fixer";
import { runKlint } from "../core/runner";

function lint(code: string) {
  const root = mkdtempSync(join(tmpdir(), "klint-test-"));
  writeFileSync(join(root, "subject.ts"), code);
  const violations = runKlint({
    root,
    include: ["."],
    rules: { "sonar/prefer-nullish-coalescing-assign": "error" },
  });
  rmSync(root, { recursive: true });
  return violations;
}

function lintAndFix(code: string): string {
  const root = mkdtempSync(join(tmpdir(), "klint-test-"));
  const file = join(root, "subject.ts");
  writeFileSync(file, code);
  const violations = runKlint({
    root,
    include: ["."],
    rules: { "sonar/prefer-nullish-coalescing-assign": "error" },
  });
  applyFixes(violations, root);
  const result = readFileSync(file, "utf-8");
  rmSync(root, { recursive: true });
  return result;
}

describe("sonar/prefer-nullish-coalescing-assign", () => {
  test("flags if (x == null) x = value", () => {
    const v = lint("let x: object | undefined; if (x == null) x = {};");
    expect(v).toHaveLength(1);
    expect(v[0].rule).toBe("sonar/prefer-nullish-coalescing-assign");
  });

  test("flags if (obj.prop == null) obj.prop = value", () => {
    const v = lint(
      "declare const result: { hooks?: object }; if (result.hooks == null) result.hooks = {};"
    );
    expect(v).toHaveLength(1);
  });

  test("flags block form: if (x == null) { x = value; }", () => {
    const v = lint("let x: object | undefined; if (x == null) { x = {}; }");
    expect(v).toHaveLength(1);
  });

  test("flags strict nullish pair", () => {
    const v = lint(
      "let x: object | undefined; if (x === null || x === undefined) x = {};"
    );
    expect(v).toHaveLength(1);
  });

  test("does not flag when else branch is present", () => {
    const v = lint(
      "let x: object | undefined; if (x == null) { x = {}; } else { console.log(x); }"
    );
    expect(v).toHaveLength(0);
  });

  test("does not flag when condition operand differs from assignment target", () => {
    const v = lint("let x: object | undefined; let y: object; if (x == null) y = {};");
    expect(v).toHaveLength(0);
  });

  test("does not flag boolean negation because falsy is not nullish", () => {
    const v = lint("let x: object | undefined; if (!x) x = {};");
    expect(v).toHaveLength(0);
  });

  test("does not flag non-nullish strict pair", () => {
    const v = lint("let x: object | undefined; if (x === null || x === false) x = {};");
    expect(v).toHaveLength(0);
  });

  test("does not flag ??= already in use", () => {
    const v = lint("let x: object | undefined; x ??= {};");
    expect(v).toHaveLength(0);
  });

  test("fix rewrites if (x == null) x = value to x ??= value", () => {
    const result = lintAndFix("let x: object | undefined;\nif (x == null) x = {};\n");
    expect(result).toBe("let x: object | undefined;\nx ??= {};\n");
  });

  test("fix preserves indentation", () => {
    const result = lintAndFix(
      "function f() {\n  let x: object | undefined;\n  if (x == null) x = {};\n}\n"
    );
    expect(result).toContain("  x ??= {};");
  });

  test("fix has populated fix field", () => {
    const root = mkdtempSync(join(tmpdir(), "klint-test-"));
    writeFileSync(
      join(root, "subject.ts"),
      "let x: object | undefined;\nif (x == null) x = {};\n"
    );
    const violations = runKlint({
      root,
      include: ["."],
      rules: { "sonar/prefer-nullish-coalescing-assign": "error" },
    });
    rmSync(root, { recursive: true });
    expect(violations[0].fix).toBeDefined();
    expect(violations[0].fix?.replacement).toBe("x ??= {};");
  });
});
