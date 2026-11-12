import { afterAll, describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { BIOME_ELSE_REASON, CORPUS, SCORING_ROWS } from "./complexity-biome-corpus";

const repoRoot = join(import.meta.dir, "..");

const root = mkdtempSync(join(tmpdir(), "klint-complexity-biome-"));

afterAll(() => {
  rmSync(root, { recursive: true, force: true });
});

function knownDifferences(): Map<
  number,
  { biome: number; klint: number; reason: string }
> {
  const known = new Map<number, { biome: number; klint: number; reason: string }>();
  for (const [index, start] of corpusLayout().starts.entries()) {
    const entry = CORPUS[index];
    if (entry?.known) {
      known.set(start, entry.known);
    }
  }
  return known;
}

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

function biomeScores(): Map<number, number> {
  writeFileSync(
    join(root, "biome.json"),
    JSON.stringify({
      linter: {
        enabled: true,
        rules: {
          complexity: {
            noExcessiveCognitiveComplexity: {
              level: "error",
              options: { maxAllowedComplexity: 1 },
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
      join(root, "corpus.ts"),
    ],
    { encoding: "utf8" }
  );
  const out = `${result.stdout}\n${result.stderr}`;
  const scores = new Map<number, number>();
  const pattern =
    /corpus\.ts:(\d+):\d+ lint\/complexity\/noExcessiveCognitiveComplexity[\s\S]*?Excessive complexity of (\d+) detected/g;
  for (const match of out.matchAll(pattern)) {
    scores.set(Number(match[1]), Number(match[2]));
  }
  return scores;
}

function klintScores(): Map<number, number> {
  writeFileSync(
    join(root, "klint.yaml"),
    'include: ["."]\nrules:\n  cognitive-complexity:\n    max: 1\n'
  );
  const result = spawnSync(
    "cargo",
    ["run", "-p", "klint-rs", "--", "--config", root, "--json"],
    { cwd: repoRoot, encoding: "utf8" }
  );
  const json = JSON.parse(result.stdout) as {
    violations: Array<{ rule: string; line: number; message: string }>;
  };
  const scores = new Map<number, number>();
  for (const v of json.violations) {
    if (v.rule !== "cognitive-complexity") {
      continue;
    }
    const score = /cognitive complexity (\d+) /.exec(v.message);
    expect(score, v.message).not.toBeNull();
    scores.set(v.line, Number(score?.[1]));
  }
  return scores;
}

describe("cognitive-complexity matches Biome 2.6.5 on the TypeScript corpus", () => {
  const { source, starts } = corpusLayout();
  writeFileSync(join(root, "corpus.ts"), source);

  test("the corpus has at least 30 functions and covers every scoring row", () => {
    expect(CORPUS.length).toBeGreaterThanOrEqual(30);
    const covered = new Set(CORPUS.map((entry) => entry.row));
    for (const row of SCORING_ROWS) {
      expect(covered.has(row), `no corpus function for row ${row}`).toBe(true);
    }
  });

  test("every corpus function is reported by both tools", () => {
    const biome = biomeScores();
    const klint = klintScores();
    for (const [index, start] of starts.entries()) {
      expect(biome.has(start), `biome missed ${CORPUS[index]?.row} at ${start}`).toBe(
        true
      );
      expect(klint.has(start), `klint missed ${CORPUS[index]?.row} at ${start}`).toBe(
        true
      );
    }
  });

  test("scores are equal per function start line, except the known differences", () => {
    const biome = biomeScores();
    const klint = klintScores();
    const known = knownDifferences();
    const lines = new Set([...biome.keys(), ...klint.keys()]);
    const differences: string[] = [];
    for (const line of [...lines].sort((a, b) => a - b)) {
      if (biome.get(line) !== klint.get(line) && !known.has(line)) {
        differences.push(
          `line ${line}: biome ${biome.get(line)} klint ${klint.get(line)}`
        );
      }
    }
    expect(differences).toEqual([]);
  });

  test("each known difference is exactly the recorded biome and klint score", () => {
    const biome = biomeScores();
    const klint = klintScores();
    const known = knownDifferences();
    expect(known.size).toBeGreaterThanOrEqual(2);
    for (const [line, expected] of known) {
      expect({ biome: biome.get(line), klint: klint.get(line) }, `line ${line}`).toEqual({
        biome: expected.biome,
        klint: expected.klint,
      });
    }
  });

  test("known differences carry the plain-else reason and a biome score below klint", () => {
    for (const [line, known] of knownDifferences()) {
      expect(known.reason, `line ${line}`).toBe(BIOME_ELSE_REASON);
      expect(known.biome, `line ${line}`).toBeLessThan(known.klint);
    }
  });

  test("the ?? entries have no known difference", () => {
    const withNullish = CORPUS.filter((entry) => entry.code.includes("??"));
    expect(withNullish.length).toBeGreaterThanOrEqual(3);
    for (const entry of withNullish) {
      expect(entry.known).toBeUndefined();
    }
  });
});
