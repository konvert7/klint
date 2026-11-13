import { afterAll, describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  bunLockDirs,
  cargoLockError,
  driftedBunLocks,
  isLockInput,
  lockInputsChanged,
  pathsInPorcelain,
} from "../tools/check-lock";

const repoRoot = join(import.meta.dir, "..");
const scratch = mkdtempSync(join(tmpdir(), "klint-check-lock-"));
let fixturesMade = 0;

afterAll(() => {
  rmSync(scratch, { recursive: true, force: true });
});

function run(cwd: string, command: string, args: string[]): void {
  const r = spawnSync(command, args, { cwd, encoding: "utf8" });
  if (r.status !== 0) throw new Error(`${command} ${args.join(" ")}: ${r.stderr}`);
}

function commitAll(root: string): void {
  run(root, "git", ["init", "-q"]);
  run(root, "git", ["add", "-A"]);
  run(root, "git", [
    "-c",
    "user.name=t",
    "-c",
    "user.email=t@t",
    "commit",
    "-qm",
    "init",
  ]);
}

function newFixtureDir(): string {
  fixturesMade += 1;
  const dir = join(scratch, String(fixturesMade));
  mkdirSync(dir, { recursive: true });

  return dir;
}

const LOCKED = { helper: "file:./helper" };
const DRIFTED = { helper: "file:./helper", other: "file:./helper" };

function writeBunManifest(dir: string, dependencies: Record<string, string>): void {
  mkdirSync(join(dir, "helper"), { recursive: true });
  writeFileSync(join(dir, "helper", "package.json"), JSON.stringify({ name: "helper" }));
  writeFileSync(join(dir, "package.json"), JSON.stringify({ name: "app", dependencies }));
}

function bunRepo(): string {
  const root = newFixtureDir();
  writeBunManifest(root, LOCKED);
  writeBunManifest(join(root, "nested"), LOCKED);
  run(root, process.execPath, ["install", "--lockfile-only"]);
  run(join(root, "nested"), process.execPath, ["install", "--lockfile-only"]);
  commitAll(root);

  return root;
}

function writeCrate(dir: string, version: string): void {
  mkdirSync(join(dir, "src"), { recursive: true });
  writeFileSync(join(dir, "src", "lib.rs"), "");
  writeFileSync(
    join(dir, "Cargo.toml"),
    `[package]\nname = "fixture"\nversion = "${version}"\nedition = "2021"\n`
  );
}

function cargoRepo(): string {
  const root = newFixtureDir();
  writeCrate(root, "1.0.0");
  run(root, "cargo", ["generate-lockfile", "--offline"]);
  commitAll(root);

  return root;
}

describe("isLockInput", () => {
  test("accepts every bun and cargo manifest or lockfile, in any folder", () => {
    const inputs = [
      "package.json",
      "bun.lock",
      "bunfig.toml",
      ".npmrc",
      "a/b/package.json",
    ];

    for (const path of [...inputs, "Cargo.toml", "Cargo.lock", "crates/x/Cargo.toml"]) {
      expect(isLockInput(path)).toBe(true);
    }
  });

  test("rejects source files and lookalike names", () => {
    for (const path of [
      "src/index.ts",
      "package.json.bak",
      "bun.lockb",
      "Cargo.toml.orig",
    ]) {
      expect(isLockInput(path)).toBe(false);
    }
  });
});

describe("pathsInPorcelain", () => {
  test("reads modified, untracked and both sides of a rename", () => {
    const porcelain =
      " M package.json\n?? a/Cargo.toml\nR  old/bun.lock -> new/bun.lock\n";
    expect(pathsInPorcelain(porcelain)).toEqual([
      "package.json",
      "a/Cargo.toml",
      "old/bun.lock",
      "new/bun.lock",
    ]);
  });
});

describe("bunLockDirs", () => {
  test("names the folder of every bun.lock and nothing else", () => {
    expect(bunLockDirs(["bun.lock", "package.json", "a/demo/bun.lock"])).toEqual([
      ".",
      "a/demo",
    ]);
  });
});

describe("driftedBunLocks", () => {
  test("passes when every lock matches its manifest", () => {
    expect(driftedBunLocks(bunRepo())).toEqual([]);
  });

  test("names only the project whose manifest changed without its lock", () => {
    const root = bunRepo();
    writeBunManifest(join(root, "nested"), DRIFTED);
    expect(driftedBunLocks(root)).toEqual(["nested"]);
  });

  test("flags a lock bun would delete because no dependency is left", () => {
    const root = bunRepo();
    writeBunManifest(root, {});
    expect(driftedBunLocks(root)).toEqual(["."]);
  });

  test("leaves the checkout's lock untouched", () => {
    const root = bunRepo();
    writeBunManifest(root, DRIFTED);
    driftedBunLocks(root);
    const status = spawnSync("git", ["status", "--porcelain"], { cwd: root });
    expect(status.stdout.toString()).toBe(" M package.json\n");
  });

  test("every bun.lock in this repo matches its package.json files", () => {
    expect(driftedBunLocks(repoRoot)).toEqual([]);
  });
});

describe("cargoLockError", () => {
  test("passes when Cargo.lock matches Cargo.toml", () => {
    expect(cargoLockError(cargoRepo())).toBeNull();
  });

  test("reports a Cargo.toml change that Cargo.lock does not reflect", () => {
    const root = cargoRepo();
    writeCrate(root, "2.0.0");
    expect(cargoLockError(root)).toContain("--locked");
  });

  test("passes a folder without a Cargo.lock", () => {
    expect(cargoLockError(newFixtureDir())).toBeNull();
  });

  test("this repo's Cargo.lock matches its Cargo.toml files", () => {
    expect(cargoLockError(repoRoot)).toBeNull();
  });
});

describe("lockInputsChanged", () => {
  test("ignores a change outside the lock inputs", () => {
    const root = bunRepo();
    writeFileSync(join(root, "index.ts"), "export {};\n");
    expect(lockInputsChanged(root)).toBe(false);
  });

  test("sees an edited Cargo.toml", () => {
    const root = cargoRepo();
    writeCrate(root, "2.0.0");
    expect(lockInputsChanged(root)).toBe(true);
  });
});
