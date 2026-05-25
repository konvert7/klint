import { afterEach, describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { runKlint } from "../../core/runner";
import type { KlintConfig, Violation } from "../../core/types";
import cases from "./arch-cases.json";

interface GoldenCase {
  name: string;
  tsconfig?: object;
  config: Omit<KlintConfig, "root">;
  files: Record<string, string>;
  expected: GoldenEnvelope;
}

interface GoldenEnvelope {
  violations: Array<Omit<Violation, "fix"> & { fix: Violation["fix"] | null }>;
  summary: {
    errors: number;
    warnings: number;
  };
}

const roots: string[] = [];

afterEach(() => {
  for (const root of roots.splice(0)) {
    rmSync(root, { recursive: true, force: true });
  }
});

function writeCaseFixture(testCase: GoldenCase): string {
  const root = mkdtempSync(join(tmpdir(), "klint-golden-arch-"));
  roots.push(root);

  if (testCase.tsconfig) {
    writeFileSync(
      join(root, "tsconfig.json"),
      JSON.stringify(testCase.tsconfig, null, 2)
    );
  }

  for (const [relPath, content] of Object.entries(testCase.files)) {
    const absPath = join(root, relPath);
    mkdirSync(dirname(absPath), { recursive: true });
    writeFileSync(absPath, content);
  }

  return root;
}

function normalize(violations: Violation[]): GoldenEnvelope {
  const normalized = violations
    .map((v) => ({ ...v, fix: v.fix ?? null }))
    .sort(
      (a, b) =>
        a.file.localeCompare(b.file) ||
        a.line - b.line ||
        a.rule.localeCompare(b.rule) ||
        a.message.localeCompare(b.message)
    );
  const errors = normalized.filter((v) => v.severity === "error").length;
  return {
    violations: normalized,
    summary: {
      errors,
      warnings: normalized.length - errors,
    },
  };
}

describe("golden parity — architecture rules", () => {
  for (const testCase of cases as unknown as GoldenCase[]) {
    test(testCase.name, () => {
      const root = writeCaseFixture(testCase);
      const actual = normalize(runKlint({ ...testCase.config, root }, {}));
      expect(actual).toEqual(testCase.expected);
    });
  }
});
