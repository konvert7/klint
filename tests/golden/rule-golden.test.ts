import { afterEach, describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import YAML from "yaml";
import cases from "./rule-cases.json";

interface Violation {
  file: string;
  line: number;
  rule: string;
  severity: string;
  message: string;
  fix?: { startLine: number; endLine: number; replacement: string } | null;
}

type KlintConfig = Record<string, unknown>;

interface GoldenCase {
  name: string;
  config: KlintConfig;
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
  const root = mkdtempSync(join(tmpdir(), "klint-golden-rule-"));
  roots.push(root);

  writeFileSync(join(root, "klint.yaml"), YAML.stringify(testCase.config));

  for (const [relPath, content] of Object.entries(testCase.files)) {
    const absPath = join(root, relPath);
    mkdirSync(dirname(absPath), { recursive: true });
    writeFileSync(absPath, content);
  }

  return root;
}

function runRustKlint(root: string, testCase: GoldenCase): GoldenEnvelope {
  const result = spawnSync(
    "cargo",
    ["run", "-p", "klint-rs", "--", "--config", root, "--json"],
    {
      cwd: join(import.meta.dir, "..", ".."),
      encoding: "utf8",
    }
  );

  const expectedStatus = testCase.expected.summary.errors > 0 ? 2 : 0;
  expect(result.status, result.stderr || result.stdout).toBe(expectedStatus);
  return JSON.parse(result.stdout) as GoldenEnvelope;
}

describe("golden cases — rules", () => {
  for (const testCase of cases as unknown as GoldenCase[]) {
    test(testCase.name, () => {
      const root = writeCaseFixture(testCase);
      expect(runRustKlint(root, testCase)).toEqual(testCase.expected);
    });
  }
});
