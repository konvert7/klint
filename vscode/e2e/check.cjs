const assert = require("node:assert/strict");
const { readFileSync } = require("node:fs");

const result = JSON.parse(readFileSync(process.argv[2], "utf8"));
const link = "https://klint.dev/rules/arch/forbidden";

assert.deepEqual(result.initial, [
  { line: 0, severity: "Error", message: "Use logger", source: "klint", code: "arch/forbidden", target: link },
  { line: 1, severity: "Warning", message: "No debugger", source: "klint", code: "arch/forbidden", target: link },
]);
assert.deepEqual(result.afterFixAndSave, []);
console.log("e2e: diagnostics appear on open and clear after a fixing save");
