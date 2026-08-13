import { beforeAll, describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import {
  agentSkillDirs,
  RECEIPT_FILE_NAME,
  SKILL_DIR_NAME,
  SKILL_FILE_NAME,
  type SkillReceipt,
  skillHash,
  skillStalenessAdvisories,
} from "../core/skill";

const ROOT = join(import.meta.dir, "..");
const SHIPPED_SKILL_PATH = join(ROOT, "skill", SKILL_DIR_NAME, SKILL_FILE_NAME);
const RUST_BUILD_TIMEOUT_MS = 300_000;
const STALE_HASH = "0".repeat(64);

function tempRoot(): string {
  return mkdtempSync(join(tmpdir(), "klint-skill-"));
}

function writeReceipt(root: string, dir: string, receipt: SkillReceipt): void {
  const path = join(root, dir, SKILL_DIR_NAME, RECEIPT_FILE_NAME);
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, JSON.stringify(receipt));
}

function advisoriesFor(configDir: string): ReturnType<typeof skillStalenessAdvisories> {
  return skillStalenessAdvisories({
    configDir,
    installed: "9.9.9",
    shippedSkillPath: SHIPPED_SKILL_PATH,
  });
}

function readReceipt(root: string, dir: string): SkillReceipt {
  return JSON.parse(
    readFileSync(join(root, dir, SKILL_DIR_NAME, RECEIPT_FILE_NAME), "utf-8")
  ) as SkillReceipt;
}

describe("skill staleness advisories", () => {
  test("a project without an installed skill is silent", () => {
    const root = tempRoot();
    expect(advisoriesFor(root)).toEqual([]);
    rmSync(root, { recursive: true, force: true });
  });

  test("a receipt matching the shipped skill is silent", () => {
    const root = tempRoot();
    writeReceipt(root, ".claude/skills", {
      version: "0.1.0",
      sha256: skillHash(readFileSync(SHIPPED_SKILL_PATH)),
    });

    expect(advisoriesFor(root)).toEqual([]);
    rmSync(root, { recursive: true, force: true });
  });

  test("a receipt from an older skill warns", () => {
    const root = tempRoot();
    writeReceipt(root, ".claude/skills", { version: "0.1.0", sha256: STALE_HASH });

    const advisories = advisoriesFor(root);

    expect(advisories).toHaveLength(1);
    expect(advisories[0].rule).toBe("klint/skill-stale");
    expect(advisories[0].severity).toBe("warn");
    expect(advisories[0].file).toBe(".claude/skills/klint-rules/SKILL.md");
    expect(advisories[0].line).toBe(1);
    expect(advisories[0].message).toContain("installed from klint 0.1.0");
    expect(advisories[0].message).toContain("klint 9.9.9");
    expect(advisories[0].message).toContain("klint install-skill");
    rmSync(root, { recursive: true, force: true });
  });

  test("each stale agent directory warns once", () => {
    const root = tempRoot();
    for (const dir of agentSkillDirs()) {
      writeReceipt(root, dir, { version: "0.1.0", sha256: STALE_HASH });
    }

    expect(advisoriesFor(root)).toHaveLength(agentSkillDirs().length);
    rmSync(root, { recursive: true, force: true });
  });

  test("agents sharing a directory are counted once", () => {
    expect(agentSkillDirs(["codex", "opencode"])).toEqual([".agents/skills"]);
  });

  test("an unreadable receipt is left alone", () => {
    const root = tempRoot();
    const path = join(root, ".claude/skills", SKILL_DIR_NAME, RECEIPT_FILE_NAME);
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, "{ not json");

    expect(advisoriesFor(root)).toEqual([]);
    rmSync(root, { recursive: true, force: true });
  });

  test("a hand-copied skill without a receipt is left alone", () => {
    const root = tempRoot();
    const path = join(root, ".claude/skills", SKILL_DIR_NAME, SKILL_FILE_NAME);
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, "hand written");

    expect(advisoriesFor(root)).toEqual([]);
    rmSync(root, { recursive: true, force: true });
  });
});

describe("install-skill across distributions", () => {
  let rustBin: string;

  beforeAll(() => {
    const build = spawnSync("cargo", ["build", "-p", "klint-rs"], {
      cwd: ROOT,
      encoding: "utf-8",
      timeout: RUST_BUILD_TIMEOUT_MS,
    });
    expect(build.status, build.stderr || build.stdout).toBe(0);
    rustBin = join(
      ROOT,
      "target",
      "debug",
      process.platform === "win32" ? "klint-rs.exe" : "klint-rs"
    );
    expect(existsSync(rustBin)).toBe(true);
  }, RUST_BUILD_TIMEOUT_MS);

  test("the native engine installs the skill without a node_modules tree", () => {
    const root = tempRoot();
    const result = spawnSync(rustBin, ["install-skill", "--agents", "claude"], {
      cwd: root,
      encoding: "utf-8",
    });

    expect(result.status, result.stderr).toBe(0);
    expect(
      readFileSync(join(root, ".claude/skills", SKILL_DIR_NAME, SKILL_FILE_NAME), "utf-8")
    ).toBe(readFileSync(SHIPPED_SKILL_PATH, "utf-8"));
    expect(advisoriesFor(root)).toEqual([]);
    rmSync(root, { recursive: true, force: true });
  });

  test("both engines write the same receipt hash", () => {
    const nativeRoot = tempRoot();
    const bunRoot = tempRoot();

    const native = spawnSync(rustBin, ["install-skill", "--agents", "claude"], {
      cwd: nativeRoot,
      encoding: "utf-8",
    });
    expect(native.status, native.stderr).toBe(0);

    const bun = spawnSync(
      "bun",
      [join(ROOT, "cli.ts"), "install-skill", "--agents", "claude", "--copy"],
      { cwd: bunRoot, encoding: "utf-8" }
    );
    expect(bun.status, bun.stderr).toBe(0);

    const shipped = skillHash(readFileSync(SHIPPED_SKILL_PATH));
    expect(readReceipt(nativeRoot, ".claude/skills").sha256).toBe(shipped);
    expect(readReceipt(bunRoot, ".claude/skills").sha256).toBe(shipped);
    rmSync(nativeRoot, { recursive: true, force: true });
    rmSync(bunRoot, { recursive: true, force: true });
  });

  test("the native engine rejects an unknown agent", () => {
    const root = tempRoot();
    const result = spawnSync(rustBin, ["install-skill", "--agents", "emacs"], {
      cwd: root,
      encoding: "utf-8",
    });

    expect(result.status).toBe(1);
    expect(result.stderr).toContain('unknown agent "emacs"');
    rmSync(root, { recursive: true, force: true });
  });

  test("the native engine rejects an unknown argument instead of silently linting", () => {
    const root = tempRoot();
    writeFileSync(join(root, "klint.yaml"), 'include: ["."]\n');

    const result = spawnSync(rustBin, ["--config", root, "--bogus"], {
      encoding: "utf-8",
    });

    expect(result.status).toBe(1);
    expect(result.stderr).toContain('unknown argument "--bogus"');
    rmSync(root, { recursive: true, force: true });
  });
});
