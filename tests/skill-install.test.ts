import { beforeAll, describe, expect, test } from "bun:test";
import { type SpawnSyncReturns, spawnSync } from "node:child_process";
import {
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readlinkSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import {
  agentSkillDirs,
  CANONICAL_SKILL_DIR,
  RECEIPT_FILE_NAME,
  SKILL_DIR_NAME,
  SKILL_FILE_NAME,
  type SkillReceipt,
  skillAdvisories,
  skillHash,
} from "../core/skill";

const ROOT = join(import.meta.dir, "..");
const SHIPPED_SKILL_PATH = join(ROOT, "skill", SKILL_DIR_NAME, SKILL_FILE_NAME);
const RUST_BUILD_TIMEOUT_MS = 300_000;
const STALE_HASH = "0".repeat(64);
const LEGACY_TARGET = "../../node_modules/@konvert7/klint/skill/klint-rules";
const HUB_LINK = `../../${CANONICAL_SKILL_DIR}/${SKILL_DIR_NAME}`;

function tempRoot(): string {
  return mkdtempSync(join(tmpdir(), "klint-skill-"));
}

function skillPath(root: string, dir: string): string {
  return join(root, dir, SKILL_DIR_NAME);
}

function writeReceipt(root: string, dir: string, receipt: SkillReceipt): void {
  const path = join(skillPath(root, dir), RECEIPT_FILE_NAME);
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, JSON.stringify(receipt));
}

function linkSkill(root: string, dir: string, target: string): void {
  const path = skillPath(root, dir);
  mkdirSync(dirname(path), { recursive: true });
  symlinkSync(target, path, "dir");
}

function advisoriesFor(configDir: string): ReturnType<typeof skillAdvisories> {
  return skillAdvisories({
    configDir,
    installed: "9.9.9",
    shippedSkillPath: SHIPPED_SKILL_PATH,
  });
}

function rulesIn(configDir: string): string[] {
  return advisoriesFor(configDir).map((advisory) => advisory.rule);
}

function installViaBun(root: string, args: string[]): SpawnSyncReturns<string> {
  return spawnSync("bun", [join(ROOT, "cli.ts"), "install-skill", ...args], {
    cwd: root,
    encoding: "utf-8",
  });
}

describe("skill advisories", () => {
  test("a project without an installed skill is silent", () => {
    const root = tempRoot();
    expect(advisoriesFor(root)).toEqual([]);
    rmSync(root, { force: true, recursive: true });
  });

  test("a receipt matching the shipped skill is silent", () => {
    const root = tempRoot();
    writeReceipt(root, CANONICAL_SKILL_DIR, {
      version: "0.1.0",
      sha256: skillHash(readFileSync(SHIPPED_SKILL_PATH)),
    });

    expect(advisoriesFor(root)).toEqual([]);
    rmSync(root, { force: true, recursive: true });
  });

  test("a receipt from an older skill warns", () => {
    const root = tempRoot();
    writeReceipt(root, ".claude/skills", { version: "0.1.0", sha256: STALE_HASH });

    const advisories = advisoriesFor(root);

    expect(advisories).toHaveLength(1);
    expect(advisories[0].rule).toBe("klint/skill-stale");
    expect(advisories[0].severity).toBe("warn");
    expect(advisories[0].file).toBe(".claude/skills/klint-rules/SKILL.md");
    expect(advisories[0].message).toContain("installed from klint 0.1.0");
    expect(advisories[0].message).toContain("klint 9.9.9");
    rmSync(root, { force: true, recursive: true });
  });

  test("separate stale copies warn once each", () => {
    const root = tempRoot();
    for (const dir of agentSkillDirs()) {
      writeReceipt(root, dir, { version: "0.1.0", sha256: STALE_HASH });
    }

    expect(advisoriesFor(root)).toHaveLength(agentSkillDirs().length);
    rmSync(root, { force: true, recursive: true });
  });

  test("a shared skill warns once, not once per symlink", () => {
    const root = tempRoot();
    writeReceipt(root, CANONICAL_SKILL_DIR, { version: "0.1.0", sha256: STALE_HASH });
    linkSkill(root, ".claude/skills", HUB_LINK);
    linkSkill(root, ".cursor/skills", HUB_LINK);

    const advisories = advisoriesFor(root);

    expect(advisories).toHaveLength(1);
    expect(advisories[0].file).toContain(CANONICAL_SKILL_DIR);
    rmSync(root, { force: true, recursive: true });
  });

  test("a symlink into node_modules is reported as a legacy install", () => {
    const root = tempRoot();
    linkSkill(root, ".claude/skills", LEGACY_TARGET);

    const advisories = advisoriesFor(root);

    expect(advisories).toHaveLength(1);
    expect(advisories[0].rule).toBe("klint/skill-legacy-link");
    expect(advisories[0].severity).toBe("warn");
    expect(advisories[0].file).toBe(".claude/skills/klint-rules");
    expect(advisories[0].message).toContain("node_modules");
    expect(advisories[0].message).toContain("klint install-skill");
    rmSync(root, { force: true, recursive: true });
  });

  test("a dangling node_modules symlink is still reported", () => {
    const root = tempRoot();
    linkSkill(root, ".claude/skills", "../../node_modules/gone/klint-rules");

    expect(rulesIn(root)).toEqual(["klint/skill-legacy-link"]);
    rmSync(root, { force: true, recursive: true });
  });

  test("a hand-copied skill without a receipt is left alone", () => {
    const root = tempRoot();
    const path = join(skillPath(root, ".claude/skills"), SKILL_FILE_NAME);
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, "hand written");

    expect(advisoriesFor(root)).toEqual([]);
    rmSync(root, { force: true, recursive: true });
  });

  test("an unreadable receipt is left alone", () => {
    const root = tempRoot();
    const path = join(skillPath(root, ".claude/skills"), RECEIPT_FILE_NAME);
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, "{ not json");

    expect(advisoriesFor(root)).toEqual([]);
    rmSync(root, { force: true, recursive: true });
  });
});

describe("install-skill across distributions", () => {
  let rustBin: string;

  function installViaNative(root: string, args: string[]): SpawnSyncReturns<string> {
    return spawnSync(rustBin, ["install-skill", ...args], {
      cwd: root,
      encoding: "utf-8",
    });
  }

  function expectSharedLayout(root: string): void {
    expect(lstatSync(skillPath(root, CANONICAL_SKILL_DIR)).isDirectory()).toBe(true);
    expect(
      existsSync(join(skillPath(root, CANONICAL_SKILL_DIR), RECEIPT_FILE_NAME))
    ).toBe(true);
    for (const dir of [".claude/skills", ".cursor/skills"]) {
      expect(lstatSync(skillPath(root, dir)).isSymbolicLink()).toBe(true);
      expect(readlinkSync(skillPath(root, dir))).toBe(HUB_LINK);
      expect(readFileSync(join(skillPath(root, dir), SKILL_FILE_NAME), "utf-8")).toBe(
        readFileSync(SHIPPED_SKILL_PATH, "utf-8")
      );
    }
  }

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

  test("the native engine defaults to the shared layout without a node_modules tree", () => {
    const root = tempRoot();
    const result = installViaNative(root, []);

    expect(result.status, result.stderr).toBe(0);
    expectSharedLayout(root);
    expect(advisoriesFor(root)).toEqual([]);
    rmSync(root, { force: true, recursive: true });
  });

  test("the bun CLI defaults to the same shared layout", () => {
    const root = tempRoot();
    const result = installViaBun(root, []);

    expect(result.status, result.stderr).toBe(0);
    expectSharedLayout(root);
    expect(advisoriesFor(root)).toEqual([]);
    rmSync(root, { force: true, recursive: true });
  });

  test("both engines write the same receipt hash", () => {
    const nativeRoot = tempRoot();
    const bunRoot = tempRoot();
    expect(installViaNative(nativeRoot, []).status).toBe(0);
    expect(installViaBun(bunRoot, []).status).toBe(0);

    const shipped = skillHash(readFileSync(SHIPPED_SKILL_PATH));
    for (const root of [nativeRoot, bunRoot]) {
      const receipt = JSON.parse(
        readFileSync(
          join(skillPath(root, CANONICAL_SKILL_DIR), RECEIPT_FILE_NAME),
          "utf-8"
        )
      ) as SkillReceipt;
      expect(receipt.sha256).toBe(shipped);
      rmSync(root, { force: true, recursive: true });
    }
  });

  test("--copy gives every agent directory its own receipt", () => {
    const root = tempRoot();
    expect(installViaNative(root, ["--copy"]).status).toBe(0);

    for (const dir of agentSkillDirs()) {
      expect(lstatSync(skillPath(root, dir)).isDirectory()).toBe(true);
      expect(existsSync(join(skillPath(root, dir), RECEIPT_FILE_NAME))).toBe(true);
    }
    rmSync(root, { force: true, recursive: true });
  });

  test.each([
    ["live", LEGACY_TARGET],
    ["dangling", "../../node_modules/gone/klint-rules"],
  ])("a %s node_modules link is migrated without --force", (_label, target) => {
    for (const install of [installViaNative, installViaBun]) {
      const root = tempRoot();
      for (const dir of agentSkillDirs()) linkSkill(root, dir, target);

      expect(install(root, []).status).toBe(0);

      expectSharedLayout(root);
      expect(advisoriesFor(root)).toEqual([]);
      rmSync(root, { force: true, recursive: true });
    }
  });

  test("a skill klint did not install is refused without --force", () => {
    for (const install of [installViaNative, installViaBun]) {
      const root = tempRoot();
      const path = join(skillPath(root, CANONICAL_SKILL_DIR), SKILL_FILE_NAME);
      mkdirSync(dirname(path), { recursive: true });
      writeFileSync(path, "hand written");

      const result = install(root, []);

      expect(result.status).toBe(1);
      expect(result.stderr).toContain("--force");
      expect(readFileSync(path, "utf-8")).toBe("hand written");
      rmSync(root, { force: true, recursive: true });
    }
  });

  test("--force replaces a skill klint did not install", () => {
    for (const install of [installViaNative, installViaBun]) {
      const root = tempRoot();
      const path = join(skillPath(root, CANONICAL_SKILL_DIR), SKILL_FILE_NAME);
      mkdirSync(dirname(path), { recursive: true });
      writeFileSync(path, "hand written");

      expect(install(root, ["--force"]).status).toBe(0);

      expect(readFileSync(path, "utf-8")).toBe(readFileSync(SHIPPED_SKILL_PATH, "utf-8"));
      rmSync(root, { force: true, recursive: true });
    }
  });

  test("no install path ever points into node_modules", () => {
    for (const install of [installViaNative, installViaBun]) {
      const root = tempRoot();
      expect(install(root, []).status).toBe(0);

      for (const dir of [".claude/skills", ".cursor/skills"]) {
        expect(readlinkSync(skillPath(root, dir))).not.toContain("node_modules");
      }
      rmSync(root, { force: true, recursive: true });
    }
  });

  test("the subcommand still installs when engine flags precede it", () => {
    const root = tempRoot();
    const result = spawnSync(
      "bun",
      [join(ROOT, "cli.ts"), "--engine", "auto", "install-skill"],
      { cwd: root, encoding: "utf-8" }
    );

    expect(result.status, result.stderr).toBe(0);
    expectSharedLayout(root);
    rmSync(root, { force: true, recursive: true });
  });

  test("the bun CLI rejects an unknown argument instead of silently linting", () => {
    const root = tempRoot();
    writeFileSync(join(root, "klint.yaml"), 'include: ["."]\n');

    const result = spawnSync("bun", [join(ROOT, "cli.ts"), "--config", root, "--bogus"], {
      encoding: "utf-8",
    });

    expect(result.status).toBe(1);
    expect(result.stderr).toContain('unknown argument "--bogus"');
    rmSync(root, { force: true, recursive: true });
  });

  test("the native engine rejects an unknown agent", () => {
    const root = tempRoot();
    const result = installViaNative(root, ["--agents", "emacs"]);

    expect(result.status).toBe(1);
    expect(result.stderr).toContain('unknown agent "emacs"');
    rmSync(root, { force: true, recursive: true });
  });

  test("the native engine rejects an unknown argument instead of silently linting", () => {
    const root = tempRoot();
    writeFileSync(join(root, "klint.yaml"), 'include: ["."]\n');

    const result = spawnSync(rustBin, ["--config", root, "--bogus"], {
      encoding: "utf-8",
    });

    expect(result.status).toBe(1);
    expect(result.stderr).toContain('unknown argument "--bogus"');
    rmSync(root, { force: true, recursive: true });
  });
});
