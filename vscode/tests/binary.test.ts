import { expect, test } from "bun:test";
import { join } from "node:path";
import { resolveKlintBinary } from "../src/binary";

const root = "/work/app";

function lookup(present: string[], configuredPath?: string) {
  return {
    workspaceRoot: root,
    configuredPath,
    platform: "linux" as const,
    exists: (path: string) => present.includes(path),
  };
}

test("the klint.path setting wins over everything else", () => {
  const npm = join(root, "node_modules/.bin/klint");

  expect(resolveKlintBinary(lookup([npm], "/opt/klint"))).toBe("/opt/klint");
});

test("a blank setting is ignored", () => {
  expect(resolveKlintBinary(lookup([], "   "))).toBe("klint");
});

test("node_modules comes before the Python venv", () => {
  const npm = join(root, "node_modules/.bin/klint");
  const venv = join(root, ".venv/bin/klint");

  expect(resolveKlintBinary(lookup([venv, npm]))).toBe(npm);
});

test("the Python venv is used when there is no npm install", () => {
  const venv = join(root, ".venv/bin/klint");

  expect(resolveKlintBinary(lookup([venv]))).toBe(venv);
});

test("falls back to klint on PATH", () => {
  expect(resolveKlintBinary(lookup([]))).toBe("klint");
});

test("Windows looks for the npm .cmd shim and the venv .exe", () => {
  const venv = join(root, ".venv", "Scripts", "klint.exe");
  const found = resolveKlintBinary({ ...lookup([venv]), platform: "win32" });

  expect(found).toBe(venv);
});
