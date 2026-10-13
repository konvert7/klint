import { beforeAll, describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const ROOT = join(import.meta.dir, "..");
const RUST_BUILD_TIMEOUT_MS = 300_000;
const HELP_FORMS = [["help"], ["-h"], ["--help"]];
const RUST_BIN = join(
  ROOT,
  "target",
  "debug",
  process.platform === "win32" ? "klint-rs.exe" : "klint-rs"
);

interface Engine {
  name: string;
  command: string[];
}

const ENGINES: Engine[] = [
  { name: "klint", command: ["bun", join(ROOT, "cli.ts")] },
  { name: "klint-rs", command: [RUST_BIN] },
];

function lintableRoot(): string {
  const root = mkdtempSync(join(tmpdir(), "klint-help-"));
  writeFileSync(join(root, "klint.yaml"), 'include: ["."]\n');
  return root;
}

function runIn(engine: Engine, cwd: string, args: string[]) {
  const [bin, ...prefix] = engine.command;
  return spawnSync(bin, [...prefix, ...args], { cwd, encoding: "utf-8" });
}

beforeAll(() => {
  const build = spawnSync("cargo", ["build", "-p", "klint-rs"], {
    cwd: ROOT,
    encoding: "utf-8",
    timeout: RUST_BUILD_TIMEOUT_MS,
  });
  expect(build.status, build.stderr).toBe(0);
}, RUST_BUILD_TIMEOUT_MS);

describe.each(ENGINES)("$name CLI", (engine) => {
  test.each(HELP_FORMS)("root %s lists every subcommand", (form) => {
    const root = lintableRoot();
    const result = runIn(engine, root, [form]);

    expect(result.status, result.stderr).toBe(0);
    expect(result.stdout).toStartWith(`Usage: ${engine.name} [<command>] [options]`);
    expect(result.stdout).toMatch(/Commands:\n {2}stats +\S.*\n {2}install-skill +\S/);
    expect(result.stderr).toBe("");
    expect(existsSync(join(root, ".klint"))).toBe(false);
    rmSync(root, { force: true, recursive: true });
  });

  test.each(["stats", "install-skill"])("%s answers help, -h and --help", (command) => {
    const root = lintableRoot();
    for (const [form] of HELP_FORMS) {
      const result = runIn(engine, root, [command, form]);

      expect(result.status, result.stderr).toBe(0);
      expect(result.stdout).toStartWith(`Usage: ${engine.name} ${command} [options]`);
      expect(result.stdout).toContain("-h, --help");
    }
    expect(existsSync(join(root, ".agents"))).toBe(false);
    rmSync(root, { force: true, recursive: true });
  });

  test("an invalid lint flag shows root help without linting", () => {
    const root = lintableRoot();
    const result = runIn(engine, root, ["--json", "--bogus"]);

    expect(result.status).toBe(1);
    expect(result.stdout).toBe("");
    expect(result.stderr).toStartWith(
      `error: unknown option '--bogus'\n\nUsage: ${engine.name} [<command>]`
    );
    expect(existsSync(join(root, ".klint"))).toBe(false);
    rmSync(root, { force: true, recursive: true });
  });

  test("an invalid stats flag shows stats help without printing stats", () => {
    const root = lintableRoot();
    const result = runIn(engine, root, ["stats", "--bogus"]);

    expect(result.status).toBe(1);
    expect(result.stdout).toBe("");
    expect(result.stderr).toStartWith(
      `error: unknown option '--bogus'\n\nUsage: ${engine.name} stats [options]`
    );
    rmSync(root, { force: true, recursive: true });
  });

  test("an invalid install-skill flag shows its help without installing", () => {
    const root = lintableRoot();
    const result = runIn(engine, root, [
      "install-skill",
      "--agents",
      "claude",
      "--bogus",
    ]);

    expect(result.status).toBe(1);
    expect(result.stderr).toStartWith(
      `error: unknown option '--bogus'\n\nUsage: ${engine.name} install-skill [options]`
    );
    expect(existsSync(join(root, ".agents"))).toBe(false);
    expect(existsSync(join(root, ".claude"))).toBe(false);
    rmSync(root, { force: true, recursive: true });
  });

  test("an unknown subcommand shows root help", () => {
    const root = lintableRoot();
    const result = runIn(engine, root, ["nope"]);

    expect(result.status).toBe(1);
    expect(result.stderr).toStartWith(
      `error: unknown command 'nope'\n\nUsage: ${engine.name} [<command>]`
    );
    rmSync(root, { force: true, recursive: true });
  });

  test("a value option without its value is a usage error", () => {
    const root = lintableRoot();
    const result = runIn(engine, root, ["stats", "--config"]);

    expect(result.status).toBe(1);
    expect(result.stderr).toStartWith("error: option '--config' needs <dir>\n\nUsage:");
    rmSync(root, { force: true, recursive: true });
  });
});
