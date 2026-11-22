import { afterAll, describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  bunLockDirs,
  cargoLockError,
  checkLock,
  driftedBunLocks,
  isLockInput,
  lockInputsChanged,
  pathsInPorcelain,
} from "../tools/lib/check-lock";

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
      "patches/@scope%2Fname@1.0.0.patch",
      "a/patches/x.patch",
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
      "mypatches/x.patch",
      "patches/x.patch.orig",
      "patches/sub/x.patch",
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

  test("skips a tracked input deleted from the working tree", () => {
    const root = bunRepo();
    writeFileSync(join(root, ".npmrc"), "");
    commitAll(root);
    rmSync(join(root, ".npmrc"));
    expect(driftedBunLocks(root)).toEqual([]);
  });

  test("names the project when bun install fails", () => {
    const root = bunRepo();
    writeFileSync(join(root, "nested", "package.json"), "{");
    expect(() => driftedBunLocks(root)).toThrow(/^bun install in .*nested failed:\n./);
  });

  test("removes its scratch copy", () => {
    const scratchCopies = () =>
      readdirSync(tmpdir()).filter((name) => name.startsWith("lock-check-")).length;
    const before = scratchCopies();
    driftedBunLocks(bunRepo());
    expect(scratchCopies()).toBe(before);
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
    const error = cargoLockError(root);
    expect(error).toContain("because --locked was passed");
    expect(error).toBe(error?.trim() ?? null);
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

  test("sees a lock input among other changes", () => {
    const root = bunRepo();
    writeFileSync(join(root, "index.ts"), "export {};\n");
    writeBunManifest(root, DRIFTED);
    expect(lockInputsChanged(root)).toBe(true);
  });

  test("sees an edited Cargo.toml", () => {
    const root = cargoRepo();
    writeCrate(root, "2.0.0");
    expect(lockInputsChanged(root)).toBe(true);
  });
});

describe("checkLock", () => {
  test("passes a checkout whose locks match", () => {
    expect(checkLock(bunRepo(), false)).toEqual({
      exitCode: 0,
      stdout: "Every lockfile matches its manifests.\n",
      stderr: "",
    });
  });

  test("with changedOnly, skips a checkout whose lock inputs are unchanged", () => {
    const root = bunRepo();
    writeBunManifest(join(root, "nested"), DRIFTED);
    run(root, "git", [
      "-c",
      "user.name=t",
      "-c",
      "user.email=t@t",
      "commit",
      "-qam",
      "drift",
    ]);

    expect(checkLock(root, true)).toEqual({
      exitCode: 0,
      stdout: "skipped: no package manifest or lockfile changed\n",
      stderr: "",
    });
  });

  test("checks the whole checkout from a subfolder and names each drifted lock", () => {
    const root = bunRepo();
    writeBunManifest(join(root, "nested"), DRIFTED);

    expect(checkLock(join(root, "nested"), true)).toEqual({
      exitCode: 1,
      stdout: "",
      stderr:
        "nested/bun.lock is out of date: run `bun install` in nested.\n" +
        "Commit the updated lockfile.\n",
    });
  });

  test("lists a drifted bun.lock before a drifted Cargo.lock", () => {
    const root = cargoRepo();
    writeBunManifest(root, LOCKED);
    run(root, process.execPath, ["install", "--lockfile-only"]);
    commitAll(root);
    writeBunManifest(root, DRIFTED);
    writeCrate(root, "2.0.0");

    const result = checkLock(root, false);
    const [bun, cargo] = result.stderr.split("\n");

    expect(result.exitCode).toBe(1);
    expect(bun).toBe("bun.lock is out of date: run `bun install` in ..");
    expect(cargo).toBe(
      "Cargo.lock does not match Cargo.toml: run `cargo build` to update it."
    );
    expect(result.stderr).toContain(
      `${cargoLockError(root)}\nCommit the updated lockfile.\n`
    );
  });

  test("fails with git's own message outside a git checkout", () => {
    expect(() => checkLock(newFixtureDir(), false)).toThrow(/not a git repository/);
  });
});
