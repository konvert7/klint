import { describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const REPO = resolve(import.meta.dir, "..");
const MANIFEST = "tests/config-keys.json";

function git(...args: string[]): {
  status: number | null;
  stdout: string;
  stderr: string;
} {
  const result = spawnSync("git", args, { cwd: REPO, encoding: "utf-8" });
  return { status: result.status, stdout: result.stdout, stderr: result.stderr };
}

function mustGit(...args: string[]): string {
  const result = git(...args);
  if (result.status !== 0) {
    throw new Error(`git ${args.join(" ")} failed: ${result.stderr.trim()}`);
  }
  return result.stdout;
}

function everyListedKey(json: string): Set<string> {
  const { current, retired } = JSON.parse(json) as {
    current: string[];
    retired: string[];
  };
  return new Set([...current, ...retired]);
}

function pullRequestBase(baseBranch: string): string {
  const shallow = mustGit("rev-parse", "--is-shallow-repository").trim() === "true";
  if (shallow) mustGit("fetch", "--depth=1", "origin", baseBranch);
  else mustGit("fetch", "origin", baseBranch);
  return "FETCH_HEAD";
}

function localBase(): string {
  return mustGit("merge-base", "HEAD", "origin/main").trim();
}

function baseManifest(): Set<string> {
  const branch = process.env.GITHUB_BASE_REF;
  const base = branch ? pullRequestBase(branch) : localBase();
  const shown = git("show", `${base}:${MANIFEST}`);
  return shown.status === 0 ? everyListedKey(shown.stdout) : new Set();
}

const isCiPush = Boolean(process.env.CI) && !process.env.GITHUB_BASE_REF;

describe("the config-keys manifest", () => {
  test.skipIf(isCiPush)(
    "drops nothing it listed: a key that leaves v2 moves to retired",
    () => {
      const now = everyListedKey(readFileSync(resolve(REPO, MANIFEST), "utf-8"));
      expect([...baseManifest()].filter((key) => !now.has(key))).toEqual([]);
    }
  );
});
