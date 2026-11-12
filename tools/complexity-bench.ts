import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { performance } from "node:perf_hooks";

const repoRoot = join(import.meta.dir, "..");
const binary = join(repoRoot, "target/release/klint-rs");
const FILES_PER_LANGUAGE = 200;
const FUNCTIONS_PER_FILE = 10;
const ROUNDS = 5;
const MAX_RATIO = 1.05;

const BLANK_RULES = [
  "  blank-line-before-control-flow: warn",
  "  blank-line-after-block: warn",
];
const COMPLEXITY_RULE = "  cognitive-complexity: warn";
const LENGTH_RULE = "  function-length: warn";

type Language = {
  dir: string;
  ext: string;
  wrap: (index: number, body: string[]) => string[];
  template: (k: number) => string[];
};

function functionsOf(language: Language): string {
  const functions: string[] = [];
  for (let k = 0; k < FUNCTIONS_PER_FILE; k++) {
    functions.push(language.template(k).join("\n"));
  }
  return functions.join("\n\n");
}

function fileContent(language: Language, index: number): string {
  const body = functionsOf(language).split("\n");
  return `${language.wrap(index, body).join("\n")}\n`;
}

const LANGUAGES: Language[] = [
  {
    dir: "ts",
    ext: "ts",
    wrap: (_index, body) => body,
    template: (k) => [
      `function f${k}(a, b, xs) {`,
      "  for (const x of xs) {",
      "    if (a && x > 1) {",
      "      g(x);",
      "    } else if (b || x < 0) {",
      "      h(x);",
      "    }",
      "  }",
      "  return a ? 1 : 2;",
      "}",
    ],
  },
  {
    dir: "py",
    ext: "py",
    wrap: (_index, body) => body,
    template: (k) => [
      `def f${k}(a, b, xs):`,
      "    for x in xs:",
      "        if a and x > 1:",
      "            g(x)",
      "        elif b or x < 0:",
      "            h(x)",
      "        else:",
      "            pass",
      "    return 1 if a else 2",
      "",
    ],
  },
  {
    dir: "rs",
    ext: "rs",
    wrap: (_index, body) => body,
    template: (k) => [
      `fn f${k}(a: bool, b: bool, xs: &[i32]) -> i32 {`,
      "    for x in xs {",
      "        if a && *x > 1 {",
      "            g(*x);",
      "        } else if b || *x < 0 {",
      "            h(*x);",
      "        }",
      "    }",
      "    if a { 1 } else { 2 }",
      "}",
    ],
  },
  {
    dir: "swift",
    ext: "swift",
    wrap: (_index, body) => body,
    template: (k) => [
      `func f${k}(_ a: Bool, _ b: Bool, _ xs: [Int]) -> Int {`,
      "    for x in xs {",
      "        if a && x > 1 {",
      "            g(x)",
      "        } else if b || x < 0 {",
      "            h(x)",
      "        }",
      "    }",
      "    return a ? 1 : 2",
      "}",
    ],
  },
  {
    dir: "cs",
    ext: "cs",
    wrap: (index, body) => [
      `class C${index} {`,
      ...body.map((line) => (line === "" ? line : `    ${line}`)),
      "}",
    ],
    template: (k) => [
      `int F${k}(bool a, bool b, int[] xs) {`,
      "    foreach (var x in xs) {",
      "        if (a && x > 1) {",
      "            G(x);",
      "        } else if (b || x < 0) {",
      "            H(x);",
      "        }",
      "    }",
      "    return a ? 1 : 2;",
      "}",
    ],
  },
];

function writeCorpus(files: string): void {
  for (const language of LANGUAGES) {
    const dir = join(files, language.dir);
    mkdirSync(dir, { recursive: true });
    for (let i = 0; i < FILES_PER_LANGUAGE; i++) {
      writeFileSync(join(dir, `f${i}.${language.ext}`), fileContent(language, i));
    }
  }
}

function writeConfig(dir: string, lines: string[]): void {
  mkdirSync(dir, { recursive: true });
  writeFileSync(
    join(dir, "klint.yaml"),
    ["root: ../files", 'include: ["."]', "rules:", ...lines, ""].join("\n")
  );
}

function buildRelease(): void {
  const result = spawnSync("cargo", ["build", "--release", "-p", "klint-rs"], {
    cwd: repoRoot,
    stdio: "inherit",
  });
  if (result.status !== 0) {
    throw new Error(`cargo build failed with status ${result.status}`);
  }
}

function timedRun(configDir: string): number {
  const start = performance.now();
  const result = spawnSync(binary, ["--config", configDir], {
    env: { ...process.env, KLINT_NO_STATS: "1" },
    encoding: "utf8",
    maxBuffer: Number.POSITIVE_INFINITY,
  });
  const elapsed = performance.now() - start;
  if (result.status === null || (result.status !== 0 && result.status !== 1)) {
    throw new Error(
      `klint exited with ${result.status} (${result.signal}, ${result.error}) for ${configDir}`
    );
  }
  return elapsed;
}

function median(values: number[]): number {
  const sorted = [...values].sort((a, b) => a - b);
  const middle = Math.floor(sorted.length / 2);
  if (sorted.length % 2 === 1) {
    return sorted[middle];
  }
  return (sorted[middle - 1] + sorted[middle]) / 2;
}

const root = mkdtempSync(join(tmpdir(), "klint-complexity-bench-"));
try {
  const files = join(root, "files");
  writeCorpus(files);
  writeConfig(join(root, "blank"), BLANK_RULES);
  writeConfig(join(root, "complexity"), [...BLANK_RULES, COMPLEXITY_RULE]);
  writeConfig(join(root, "length"), [...BLANK_RULES, COMPLEXITY_RULE, LENGTH_RULE]);
  buildRelease();

  const blankTimes: number[] = [];
  const complexityTimes: number[] = [];
  const lengthTimes: number[] = [];
  for (let round = 0; round < ROUNDS; round++) {
    blankTimes.push(timedRun(join(root, "blank")));
    complexityTimes.push(timedRun(join(root, "complexity")));
    lengthTimes.push(timedRun(join(root, "length")));
  }

  const blankMedian = median(blankTimes);
  const complexityMedian = median(complexityTimes);
  const lengthMedian = median(lengthTimes);
  const ratio = complexityMedian / blankMedian;
  const lengthRatio = lengthMedian / complexityMedian;
  console.log(
    `blank-line rules only: ${blankMedian.toFixed(1)} ms (median of ${ROUNDS})`
  );
  console.log(
    `with cognitive-complexity: ${complexityMedian.toFixed(1)} ms (median of ${ROUNDS})`
  );
  console.log(
    `with function-length and cognitive-complexity: ${lengthMedian.toFixed(1)} ms (median of ${ROUNDS})`
  );
  console.log(`ratio: ${ratio.toFixed(3)}`);
  console.log(`function-length ratio: ${lengthRatio.toFixed(3)}`);
  process.exitCode = ratio > MAX_RATIO || lengthRatio > MAX_RATIO ? 1 : 0;
} finally {
  rmSync(root, { recursive: true, force: true });
}
