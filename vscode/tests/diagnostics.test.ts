import { describe, expect, test } from "bun:test";
import { diagnosticsByFile, ruleDocsUrl } from "../src/diagnostics";

const output = JSON.stringify({
  violations: [
    {
      file: "src/a.ts",
      line: 3,
      rule: "arch/imports",
      message: "UI must not import data",
      severity: "error",
      fix: null,
      column: 11,
      endColumn: 23,
    },
    {
      file: "src/a.ts",
      line: 9,
      rule: "function-length",
      message: "Too long",
      severity: "warn",
      fix: null,
    },
    {
      file: "lib/b.py",
      line: 1,
      rule: "arch/forbidden",
      message: "m",
      severity: "error",
    },
  ],
  summary: { errors: 2, warnings: 1 },
});

describe("diagnosticsByFile", () => {
  test("groups violations by file", () => {
    const byFile = diagnosticsByFile(output);

    expect([...byFile.keys()]).toEqual(["src/a.ts", "lib/b.py"]);
    expect(byFile.get("src/a.ts")).toHaveLength(2);
  });

  test("maps klint's 1-based line to VS Code's 0-based line", () => {
    expect(diagnosticsByFile(output).get("src/a.ts")?.[0]?.line).toBe(2);
  });

  test("maps warn to warning and error to error", () => {
    const [error, warning] = diagnosticsByFile(output).get("src/a.ts") ?? [];

    expect(error?.severity).toBe("error");
    expect(warning?.severity).toBe("warning");
  });

  test("turns klint's 1-based, end-exclusive columns into 0-based characters", () => {
    const [spanned] = diagnosticsByFile(output).get("src/a.ts") ?? [];

    expect(spanned?.characters).toEqual({ start: 10, end: 22 });
  });

  test("a violation without columns covers the whole line", () => {
    const [, lineOnly] = diagnosticsByFile(output).get("src/a.ts") ?? [];

    expect(lineOnly?.characters).toBeUndefined();
  });

  test("links each diagnostic to its rule page", () => {
    const [error] = diagnosticsByFile(output).get("src/a.ts") ?? [];

    expect(error?.rule).toBe("arch/imports");
    expect(error?.docsUrl).toBe("https://klint.dev/rules/arch/imports");
  });

  test("a clean run gives no diagnostics", () => {
    const clean = JSON.stringify({ violations: [], summary: { errors: 0, warnings: 0 } });

    expect(diagnosticsByFile(clean).size).toBe(0);
  });

  test("output that is not JSON is an error, not an empty result", () => {
    expect(() => diagnosticsByFile("klint: config error")).toThrow("not JSON");
  });

  test("JSON without a violations array is an error", () => {
    expect(() => diagnosticsByFile("{}")).toThrow("no violations array");
  });
});

test("ruleDocsUrl keeps the rule's group in the path", () => {
  expect(ruleDocsUrl("no-floating-promise")).toBe(
    "https://klint.dev/rules/no-floating-promise"
  );
});
