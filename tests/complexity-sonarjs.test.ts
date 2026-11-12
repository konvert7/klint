import { afterAll, describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import { pathToFileURL } from "node:url";
import { CORPUS, SCORING_ROWS } from "./complexity-sonarjs-corpus";

const repoRoot = join(import.meta.dir, "..");

const root = mkdtempSync(join(tmpdir(), "klint-complexity-sonarjs-"));

afterAll(() => {
  rmSync(root, { recursive: true, force: true });
});

const JSX_CORPUS: Array<{ code: string }> = [
  {
    code: "function j1(a) {\n  if (a) {\n  }\n  if (a) {\n  }\n  return <div>{a && <X/>}</div>;\n}\n",
  },
  {
    code: "function j2(a, b) {\n  if (a) {\n  }\n  if (a) {\n  }\n  return <div>{a && b && <X/>}</div>;\n}\n",
  },
  {
    code: "function j3(a, b) {\n  if (a) {\n  }\n  if (a) {\n  }\n  return <div>{a && b || <X/>}</div>;\n}\n",
  },
  {
    code: "function j4(a, b) {\n  if (a) {\n  }\n  if (a) {\n  }\n  return <div>{a && (b ? <X/> : <Y/>)}</div>;\n}\n",
  },
  {
    code: "function j5(a, b) {\n  if (a) {\n  }\n  if (a) {\n  }\n  return a && b;\n}\n",
  },
];

function corpusLayout(corpus: Array<{ code: string; offset?: number }> = CORPUS): {
  source: string;
  starts: number[];
} {
  const starts: number[] = [];
  let line = 1;
  const parts = corpus.map((entry) => {
    starts.push(line + (entry.offset ?? 0));
    line += entry.code.split("\n").length - 1;
    return entry.code;
  });
  return { source: parts.join(""), starts };
}

function computeSonarScores(file: string): Map<number, number> {
  const plugin = pathToFileURL(
    join(repoRoot, "node_modules/eslint-plugin-sonarjs/cjs/plugin.js")
  ).href;
  const parser = pathToFileURL(
    join(repoRoot, "node_modules/@typescript-eslint/parser/dist/index.js")
  ).href;
  writeFileSync(
    join(root, "eslint.config.mjs"),
    `import sonarjs from ${JSON.stringify(plugin)};
import * as parser from ${JSON.stringify(parser)};
export default [{
  files: ["**/*.ts", "**/*.tsx"],
  languageOptions: { parser },
  plugins: { sonarjs },
  rules: { "sonarjs/cognitive-complexity": ["error", 1] },
}];
`
  );
  const result = spawnSync(
    join(repoRoot, "node_modules/.bin/eslint"),
    ["--config", join(root, "eslint.config.mjs"), "--format", "json", join(root, file)],
    { encoding: "utf8", cwd: root }
  );
  expect(result.stdout, result.stderr).not.toBe("");
  const files = JSON.parse(result.stdout) as Array<{
    messages: Array<{ ruleId: string; line: number; message: string }>;
  }>;
  const scores = new Map<number, number>();
  for (const message of files.flatMap((file) => file.messages)) {
    const score = /Cognitive Complexity from (\d+) /.exec(message.message);
    expect(score, message.message).not.toBeNull();
    scores.set(message.line, Number(score?.[1]));
  }
  return scores;
}

function computeKlintScores(file: string): Map<number, number> {
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
    violations: Array<{ rule: string; file: string; line: number; message: string }>;
  };
  const scores = new Map<number, number>();
  for (const v of json.violations) {
    if (v.rule !== "cognitive-complexity" || basename(v.file) !== file) {
      continue;
    }
    const score = /cognitive complexity (\d+) /.exec(v.message);
    expect(score, v.message).not.toBeNull();
    scores.set(v.line, Number(score?.[1]));
  }
  return scores;
}

const memo = <T>(compute: (file: string) => T): ((file?: string) => T) => {
  const values = new Map<string, T>();
  return (file = "corpus.ts") => {
    if (!values.has(file)) {
      values.set(file, compute(file));
    }
    return values.get(file) as T;
  };
};
const sonarScores = memo(computeSonarScores);
const klintScores = memo(computeKlintScores);
const SLOW = 300_000;

describe("cognitive-complexity matches SonarJS 3.0.5 on the TypeScript corpus", () => {
  const { source, starts } = corpusLayout();
  writeFileSync(join(root, "corpus.ts"), source);
  writeFileSync(join(root, "corpus.tsx"), corpusLayout(JSX_CORPUS).source);

  test(
    "the corpus has at least 30 functions and covers every scoring row",
    () => {
      expect(CORPUS.length).toBeGreaterThanOrEqual(30);
      const covered = new Set(CORPUS.map((entry) => entry.row));
      for (const row of SCORING_ROWS) {
        expect(covered.has(row), `no corpus function for row ${row}`).toBe(true);
      }
    },
    SLOW
  );

  test(
    "every corpus function is reported by both tools",
    () => {
      const sonar = sonarScores();
      const klint = klintScores();
      for (const [index, start] of starts.entries()) {
        expect(sonar.has(start), `sonarjs missed ${CORPUS[index]?.row} at ${start}`).toBe(
          true
        );
        expect(klint.has(start), `klint missed ${CORPUS[index]?.row} at ${start}`).toBe(
          true
        );
      }
    },
    SLOW
  );

  test(
    "scores are equal per function start line with no known differences",
    () => {
      const sonar = sonarScores();
      const klint = klintScores();
      const lines = new Set([...sonar.keys(), ...klint.keys()]);
      const differences: string[] = [];
      for (const line of [...lines].sort((a, b) => a - b)) {
        if (sonar.get(line) !== klint.get(line)) {
          differences.push(
            `line ${line}: sonarjs ${sonar.get(line)} klint ${klint.get(line)}`
          );
        }
      }
      expect(differences).toEqual([]);
    },
    SLOW
  );

  test(
    "the six PO-probe shapes score their SonarJS values in both tools",
    () => {
      const sonar = sonarScores();
      const klint = klintScores();
      const probes = [...starts.entries()].filter(
        ([index]) => CORPUS[index]?.expected !== undefined
      );
      expect(probes.length).toBe(6);
      for (const [index, start] of probes) {
        const expected = CORPUS[index]?.expected;
        expect(sonar.get(start), `sonarjs at ${start}`).toBe(expected);
        expect(klint.get(start), `klint at ${start}`).toBe(expected);
      }
    },
    SLOW
  );

  test(
    "the corpus keeps a ?? entry and an || entry",
    () => {
      expect(
        CORPUS.filter((entry) => entry.code.includes("??")).length
      ).toBeGreaterThanOrEqual(3);
      expect(
        CORPUS.filter((entry) => entry.code.includes("||")).length
      ).toBeGreaterThanOrEqual(3);
    },
    SLOW
  );
});

const TS_STARTS = corpusLayout().starts;
const JSX_STARTS = corpusLayout(JSX_CORPUS).starts;

function corpusStart(name: string): number {
  const index = CORPUS.findIndex((entry) => entry.code.startsWith(`function ${name}(`));
  const start = TS_STARTS[index];
  if (start === undefined) {
    throw new Error(`no corpus function named ${name}`);
  }
  return start;
}

type TableRow = { label: string; functions: string[]; offset: number; sonarjs: number };

const TS_TABLE_ROWS: TableRow[] = [
  { label: "h1", functions: ["h1"], offset: 0, sonarjs: 4 },
  { label: "h2", functions: ["h2"], offset: 0, sonarjs: 4 },
  { label: "h3", functions: ["h3"], offset: 0, sonarjs: 4 },
  { label: "h4", functions: ["h4"], offset: 0, sonarjs: 4 },
  { label: "h5", functions: ["h5"], offset: 0, sonarjs: 4 },
  { label: "h6", functions: ["h6"], offset: 0, sonarjs: 5 },
  { label: "t1-t3 outer", functions: ["t1", "t2", "t3"], offset: 0, sonarjs: 3 },
  { label: "t1-t3 inner", functions: ["t1", "t2", "t3"], offset: 5, sonarjs: 3 },
  { label: "t4 outer", functions: ["t4"], offset: 0, sonarjs: 6 },
  { label: "t4 inner", functions: ["t4"], offset: 1, sonarjs: 3 },
  { label: "n1 outer", functions: ["n1"], offset: 0, sonarjs: 3 },
  { label: "n1 A", functions: ["n1"], offset: 5, sonarjs: 3 },
  { label: "n1 B", functions: ["n1"], offset: 6, sonarjs: 3 },
];

describe("corpus rows of the scoring table score their SonarJS values", () => {
  for (const row of TS_TABLE_ROWS) {
    test(
      `${row.label} scores ${row.sonarjs} in SonarJS and klint`,
      () => {
        const sonar = sonarScores();
        const klint = klintScores();
        for (const name of row.functions) {
          const line = corpusStart(name) + row.offset;
          expect(sonar.get(line), `sonarjs at ${line}`).toBe(row.sonarjs);
          expect(klint.get(line), `klint at ${line}`).toBe(row.sonarjs);
        }
      },
      SLOW
    );
  }
});

const JSX_TABLE_ROWS: Array<{ label: string; line: number; sonarjs: number }> = [
  { label: "j1", line: 1, sonarjs: 2 },
  { label: "j2", line: 8, sonarjs: 2 },
  { label: "j3", line: 15, sonarjs: 3 },
];

describe("JSX rows of the scoring table score their SonarJS values", () => {
  for (const row of JSX_TABLE_ROWS) {
    test(
      `${row.label} at tsx line ${row.line} scores ${row.sonarjs} in SonarJS and klint`,
      () => {
        const sonar = sonarScores("corpus.tsx");
        const klint = klintScores("corpus.tsx");
        expect(sonar.get(row.line), `sonarjs at ${row.line}`).toBe(row.sonarjs);
        expect(klint.get(row.line), `klint at ${row.line}`).toBe(row.sonarjs);
      },
      SLOW
    );
  }

  test(
    "j4 and j5 score the same in SonarJS and klint",
    () => {
      const sonar = sonarScores("corpus.tsx");
      const klint = klintScores("corpus.tsx");
      for (const start of JSX_STARTS.slice(3)) {
        expect(sonar.get(start), `sonarjs at ${start}`).toBeDefined();
        expect(klint.get(start), `klint at ${start}`).toBe(sonar.get(start));
      }
    },
    SLOW
  );
});

describe("cognitive-complexity matches SonarJS 3.0.5 on the JSX corpus", () => {
  const { source } = corpusLayout(JSX_CORPUS);
  writeFileSync(join(root, "corpus.tsx"), source);

  test(
    "JSX scores are equal per function start line with no known differences",
    () => {
      const sonar = sonarScores("corpus.tsx");
      const klint = klintScores("corpus.tsx");
      const lines = new Set([...sonar.keys(), ...klint.keys()]);
      const differences: string[] = [];
      for (const line of [...lines].sort((a, b) => a - b)) {
        if (sonar.get(line) !== klint.get(line)) {
          differences.push(
            `line ${line}: sonarjs ${sonar.get(line)} klint ${klint.get(line)}`
          );
        }
      }
      expect(differences).toEqual([]);
    },
    SLOW
  );
});
