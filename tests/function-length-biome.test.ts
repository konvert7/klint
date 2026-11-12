import { afterAll, describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { CORPUS, COUNTING_ROWS } from "./function-length-biome-corpus";

const repoRoot = join(import.meta.dir, "..");
const ALLOWED_DIFFERENCES: Record<number, string> = {};

const root = mkdtempSync(join(tmpdir(), "klint-function-length-biome-"));

afterAll(() => {
  rmSync(root, { recursive: true, force: true });
});

function corpusLayout(): { source: string; starts: number[] } {
  const starts: number[] = [];
  let line = 1;
  const parts = CORPUS.map((entry) => {
    starts.push(line + (entry.offset ?? 0));
    line += entry.code.split("\n").length - 1;
    return entry.code;
  });
  return { source: parts.join(""), starts };
}

function biomeCounts(skipBlankLines: boolean): Map<number, number> {
  writeFileSync(
    join(root, "biome.json"),
    JSON.stringify({
      linter: {
        enabled: true,
        rules: {
          complexity: {
            noExcessiveLinesPerFunction: {
              level: "error",
              options: { maxLines: 1, skipBlankLines },
            },
          },
        },
      },
    })
  );
  const result = spawnSync(
    join(repoRoot, "node_modules/.bin/biome"),
    [
      "lint",
      `--config-path=${root}`,
      "--max-diagnostics=none",
      "--colors=off",
      join(root, "corpus.tsx"),
    ],
    { encoding: "utf8" }
  );
  const out = `${result.stdout}\n${result.stderr}`;
  const counts = new Map<number, number>();
  const pattern =
    /corpus\.tsx:(\d+):\d+ lint\/complexity\/noExcessiveLinesPerFunction[\s\S]*?too many lines \((\d+)\)/g;
  for (const match of out.matchAll(pattern)) {
    counts.set(Number(match[1]), Number(match[2]));
  }
  return counts;
}

function klintCounts(skipBlankLines: boolean): Map<number, number> {
  writeFileSync(
    join(root, "klint.yaml"),
    `include: ["."]\nrules:\n  function-length:\n    max: 1\n    skipBlankLines: ${skipBlankLines}\n`
  );
  const result = spawnSync(
    "cargo",
    ["run", "-p", "klint-rs", "--", "--config", root, "--json"],
    { cwd: repoRoot, encoding: "utf8" }
  );
  const json = JSON.parse(result.stdout) as {
    violations: Array<{ rule: string; line: number; message: string }>;
  };
  const counts = new Map<number, number>();
  for (const v of json.violations) {
    if (v.rule !== "function-length") {
      continue;
    }
    const count = /has (\d+) lines/.exec(v.message);
    expect(count, v.message).not.toBeNull();
    counts.set(v.line, Number(count?.[1]));
  }
  return counts;
}

describe("function-length matches Biome 2.6.5 on the TypeScript corpus", () => {
  const { source, starts } = corpusLayout();
  writeFileSync(join(root, "corpus.tsx"), source);

  test("the corpus has at least 25 functions and covers every counting row", () => {
    expect(CORPUS.length).toBeGreaterThanOrEqual(25);
    const covered = new Set(CORPUS.map((entry) => entry.row));
    for (const row of COUNTING_ROWS) {
      expect(covered.has(row), `no corpus function for row ${row}`).toBe(true);
    }
    expect(CORPUS.some((entry) => entry.accessor && entry.code.includes("get "))).toBe(
      true
    );
    expect(CORPUS.some((entry) => entry.accessor && entry.code.includes("set "))).toBe(
      true
    );
  });

  for (const skipBlankLines of [false, true]) {
    describe(`skipBlankLines: ${skipBlankLines}`, () => {
      test("every non-accessor corpus function is reported by both tools and no accessor is", () => {
        const biome = biomeCounts(skipBlankLines);
        const klint = klintCounts(skipBlankLines);
        for (const [index, start] of starts.entries()) {
          const entry = CORPUS[index];
          const expected = !entry?.accessor;
          expect(biome.has(start), `biome ${entry?.row} at ${start}`).toBe(expected);
          expect(klint.has(start), `klint ${entry?.row} at ${start}`).toBe(expected);
        }
      });

      test("counts are equal per function start line", () => {
        const biome = biomeCounts(skipBlankLines);
        const klint = klintCounts(skipBlankLines);
        const lines = new Set([...biome.keys(), ...klint.keys()]);
        const differences: string[] = [];
        for (const line of [...lines].sort((a, b) => a - b)) {
          if (biome.get(line) !== klint.get(line) && !(line in ALLOWED_DIFFERENCES)) {
            differences.push(
              `line ${line}: biome ${biome.get(line)} klint ${klint.get(line)}`
            );
          }
        }
        expect(differences).toEqual([]);
      });
    });
  }

  test("the allow-list of differences carries a reason per entry", () => {
    for (const [line, reason] of Object.entries(ALLOWED_DIFFERENCES)) {
      expect(reason.length, `line ${line}`).toBeGreaterThan(10);
    }
  });
});
