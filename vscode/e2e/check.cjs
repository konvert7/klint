const assert = require("node:assert/strict");
const { readFileSync } = require("node:fs");

const result = JSON.parse(readFileSync(process.argv[2], "utf8"));
const docs = (rule) => `https://klint.dev/rules/${rule}`;
const wholeLine = Number.MAX_SAFE_INTEGER;

assert.deepEqual(result.initial, [
  {
    line: 2,
    start: 10,
    end: 22,
    severity: "Error",
    message: "Use RegExp.exec() instead of String.match() for non-global regexes — use /x/.exec(s) instead.",
    source: "klint",
    code: "no-string-match",
    target: docs("no-string-match"),
  },
  { line: 0, start: 0, end: wholeLine, severity: "Error", message: "Use logger", source: "klint", code: "arch/forbidden", target: docs("arch/forbidden") },
  { line: 1, start: 0, end: wholeLine, severity: "Warning", message: "No debugger", source: "klint", code: "arch/forbidden", target: docs("arch/forbidden") },
]);
assert.deepEqual(result.afterFixAndSave, []);
console.log("e2e: exact and whole-line diagnostics appear on open and clear after a fixing save");
