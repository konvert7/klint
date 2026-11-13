import { spawnSync } from "node:child_process";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, join } from "node:path";

const BUN_LOCK_INPUTS = new Set(["package.json", "bun.lock", "bunfig.toml", ".npmrc"]);
const CARGO_LOCK_INPUTS = new Set(["Cargo.toml", "Cargo.lock"]);

export function isLockInput(path: string): boolean {
  const name = basename(path);

  return BUN_LOCK_INPUTS.has(name) || CARGO_LOCK_INPUTS.has(name);
}

function git(root: string, args: string[]): string {
  const r = spawnSync("git", args, { cwd: root, encoding: "utf8" });
  if (r.status !== 0) throw new Error(r.stderr || `git ${args.join(" ")} failed`);

  return r.stdout;
}

function filesInCheckout(root: string): string[] {
  return git(root, ["ls-files", "--cached", "--others", "--exclude-standard"])
    .split("\n")
    .filter((path) => path && existsSync(join(root, path)));
}

export function pathsInPorcelain(porcelain: string): string[] {
  return porcelain
    .split("\n")
    .filter(Boolean)
    .flatMap((line) => line.slice(3).split(" -> "));
}

export function lockInputsChanged(root: string): boolean {
  return pathsInPorcelain(git(root, ["status", "--porcelain=v1"])).some(isLockInput);
}

export function bunLockDirs(paths: string[]): string[] {
  return paths.filter((path) => basename(path) === "bun.lock").map(dirname);
}

function copyInto(target: string, root: string, paths: string[]): void {
  for (const path of paths) {
    mkdirSync(join(target, dirname(path)), { recursive: true });
    copyFileSync(join(root, path), join(target, path));
  }
}

function regenerateBunLock(projectDir: string): void {
  const r = spawnSync(
    process.execPath,
    ["install", "--lockfile-only", "--ignore-scripts"],
    { cwd: projectDir, encoding: "utf8" }
  );
  if (r.status !== 0)
    throw new Error(`bun install in ${projectDir} failed:\n${r.stderr}`);
}

function bunLockChangedOnRegenerate(projectDir: string): boolean {
  const lockPath = join(projectDir, "bun.lock");
  const committed = readFileSync(lockPath, "utf8");
  regenerateBunLock(projectDir);

  return !existsSync(lockPath) || readFileSync(lockPath, "utf8") !== committed;
}

export function driftedBunLocks(root: string): string[] {
  const inputs = filesInCheckout(root).filter((path) =>
    BUN_LOCK_INPUTS.has(basename(path))
  );
  const scratch = mkdtempSync(join(tmpdir(), "lock-check-"));

  try {
    copyInto(scratch, root, inputs);

    return bunLockDirs(inputs).filter((dir) =>
      bunLockChangedOnRegenerate(join(scratch, dir))
    );
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
}

export function cargoLockError(root: string): string | null {
  if (!existsSync(join(root, "Cargo.lock"))) return null;
  const r = spawnSync("cargo", ["metadata", "--locked", "--format-version", "1"], {
    cwd: root,
    encoding: "utf8",
    stdio: ["ignore", "ignore", "pipe"],
  });

  return r.status === 0 ? null : r.stderr.trim() || "cargo metadata --locked failed";
}

function lockProblems(root: string): string[] {
  const bunProblems = driftedBunLocks(root).map(
    (dir) => `${join(dir, "bun.lock")} is out of date: run \`bun install\` in ${dir}.`
  );
  const cargoError = cargoLockError(root);
  const cargoProblems = cargoError
    ? [
        `Cargo.lock does not match Cargo.toml: run \`cargo build\` to update it.\n${cargoError}`,
      ]
    : [];

  return [...bunProblems, ...cargoProblems];
}

if (import.meta.main) {
  const root = git(".", ["rev-parse", "--show-toplevel"]).trim();
  if (process.argv.includes("--changed") && !lockInputsChanged(root)) {
    process.stdout.write("skipped: no package manifest or lockfile changed\n");
    process.exit(0);
  }

  const problems = lockProblems(root);
  if (problems.length === 0) {
    process.stdout.write("Every lockfile matches its manifests.\n");
    process.exit(0);
  }

  process.stderr.write(`${problems.join("\n")}\nCommit the updated lockfile.\n`);
  process.exit(1);
}
