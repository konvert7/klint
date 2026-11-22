import { readdirSync } from "node:fs";

// Stryker switches a mutant on through an in-process global, which a child process never
// sees, so suites that drive the Rust binary (the golden cases among them), Biome or the
// npm launcher can never observe one.
const SUBPROCESS_SUITES = new Set([
  "tests/complexity-sonarjs.test.ts",
  "tests/function-length-biome.test.ts",
  "tests/golden/arch-golden.test.ts",
  "tests/golden/rule-golden.test.ts",
  "tests/npm-launcher.test.ts",
  "tests/swift-arch-smoke.test.ts",
]);

// These read .github/workflows, which ignorePatterns below leaves out of the sandbox.
const SUITES_THAT_READ_WORKFLOWS = new Set([
  "tests/python-wheel.test.ts",
  "tests/release-workflows.test.ts",
]);

function isInProcessSuite(file) {
  return (
    file.endsWith(".test.ts") &&
    !SUBPROCESS_SUITES.has(file) &&
    !SUITES_THAT_READ_WORKFLOWS.has(file)
  );
}

// The bun runner passes testFiles to `bun test` verbatim and never expands globs, so a
// pattern here silently matches nothing and the run dies on an inspector timeout.
const testFiles = readdirSync("tests", { recursive: true })
  .map(String)
  .map((file) => `tests/${file}`)
  .filter(isInProcessSuite)
  .sort();

/** @type {import('@stryker-mutator/api/core').PartialStrykerOptions} */
export default {
  plugins: ["@hughescr/stryker-bun-runner"],
  testRunner: "bun",
  coverageAnalysis: "perTest",
  // The ring is the release and repo tooling the suite imports. The Rust engine has its
  // own run (bun run test:mutate:rust). Left out are the entrypoints, which only parse
  // argv, print and exit; their decisions live in tools/lib/, which is measured. The
  // runner's preload imports every mutated module, so an entrypoint would also run.
  mutate: [
    "tools/**/*.ts",
    "tools/**/*.js",
    "!tools/check-lf.ts",
    "!tools/check-lock.ts",
    "!tools/complexity-bench.ts",
    "!tools/exit-with.ts",
    "!tools/generate-homebrew-formula.ts",
    "!tools/prepare-native-release.ts",
    "!tools/prepare-nuget-release.ts",
    "!tools/prepare-python-release.ts",
  ],
  concurrency: Number(process.env.STRYKER_CONCURRENCY ?? 4),
  bun: {
    testFiles,
    bunArgs: ["--isolate"],
    timeout: 120000,
    inspectorTimeout: 60000,
  },
  reporters: ["clear-text", "progress", "html"],
  // Ratchet. CI runs the whole ring on every pull request (under a minute), so `break`
  // gates the total; raise it as the score rises, lower it only with the reason here.
  //   rung 1 — 2026-11-22: 60.41% ring -> break 55 (diff runs)
  //   rung 2 — 2026-11-22: 97.88% ring -> break 95 (whole ring)
  thresholds: { high: 98, low: 95, break: 95 },
  // Stryker copies the project into a sandbox with fs.copyFile, which throws ENOTSUP on a
  // symlink; target/ is the Rust build output, tens of gigabytes the suite never reads.
  ignorePatterns: [
    ".agents/skills/klint-rules",
    ".claude",
    ".codex",
    ".cursor",
    ".github",
    ".husky",
    "**/node_modules",
    "target",
    "vscode",
  ],
  tempDirName: ".stryker-tmp",
  cleanTempDir: "always",
  htmlReporter: { fileName: "reports/mutation.html" },
};
