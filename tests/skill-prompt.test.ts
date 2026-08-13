import { beforeAll, describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { existsSync, lstatSync, mkdtempSync, readdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { CANONICAL_SKILL_DIR, SKILL_DIR_NAME } from "../core/skill";

const ROOT = join(import.meta.dir, "..");
const DRIVER = join(import.meta.dir, "support", "pty-driver.py");
const RUST_BUILD_TIMEOUT_MS = 300_000;
const PROMPT_TIMEOUT_MS = 30_000;
const AGENTS_QUESTION = "Which agents should the skill be installed for?";
const STORAGE_QUESTION = "How should the skill be stored?";

const PTY_AVAILABLE =
  process.platform !== "win32" &&
  spawnSync("python3", ["-c", "import pty"], { encoding: "utf-8" }).status === 0;

function tempRoot(): string {
  return mkdtempSync(join(tmpdir(), "klint-prompt-"));
}

function agentDirsIn(root: string): string[] {
  return readdirSync(root).filter((entry) =>
    [".claude", ".agents", ".cursor"].includes(entry)
  );
}

describe.skipIf(!PTY_AVAILABLE)("install-skill prompts on a terminal", () => {
  let rustBin: string;

  function drive(root: string, args: string[], steps: string[]): string {
    const result = spawnSync(
      "python3",
      [DRIVER, rustBin, root, ["install-skill", ...args].join(","), ...steps],
      { encoding: "utf-8", timeout: 60_000 }
    );
    expect(result.status, result.stderr).toBe(0);
    return result.stdout;
  }

  beforeAll(() => {
    const build = spawnSync("cargo", ["build", "-p", "klint-rs"], {
      cwd: ROOT,
      encoding: "utf-8",
      timeout: RUST_BUILD_TIMEOUT_MS,
    });
    expect(build.status, build.stderr || build.stdout).toBe(0);
    rustBin = join(ROOT, "target", "debug", "klint-rs");
    expect(existsSync(rustBin)).toBe(true);
  }, RUST_BUILD_TIMEOUT_MS);

  test(
    "asks both questions and installs the shared layout on defaults",
    () => {
      const root = tempRoot();
      const output = drive(
        root,
        [],
        [
          `expect:${AGENTS_QUESTION}`,
          "send:enter",
          `expect:${STORAGE_QUESTION}`,
          "send:enter",
          "expect:installed .agents/skills/klint-rules",
        ]
      );

      expect(output).toContain("Shared (recommended)");
      expect(
        lstatSync(join(root, CANONICAL_SKILL_DIR, SKILL_DIR_NAME)).isDirectory()
      ).toBe(true);
      expect(
        lstatSync(join(root, ".claude/skills", SKILL_DIR_NAME)).isSymbolicLink()
      ).toBe(true);
      rmSync(root, { force: true, recursive: true });
    },
    PROMPT_TIMEOUT_MS
  );

  test(
    "the second question chooses separate copies",
    () => {
      const root = tempRoot();
      drive(
        root,
        [],
        [
          `expect:${AGENTS_QUESTION}`,
          "send:enter",
          `expect:${STORAGE_QUESTION}`,
          "send:down",
          "send:enter",
          "expect:installed .cursor/skills/klint-rules",
        ]
      );

      expect(
        lstatSync(join(root, ".claude/skills", SKILL_DIR_NAME)).isSymbolicLink()
      ).toBe(false);
      rmSync(root, { force: true, recursive: true });
    },
    PROMPT_TIMEOUT_MS
  );

  test(
    "escaping cancels without installing anything",
    () => {
      const root = tempRoot();
      const output = drive(
        root,
        [],
        [`expect:${AGENTS_QUESTION}`, "send:esc", "expect:klint: cancelled"]
      );

      expect(output).not.toContain("installed .agents");
      expect(agentDirsIn(root)).toEqual([]);
      rmSync(root, { force: true, recursive: true });
    },
    PROMPT_TIMEOUT_MS
  );

  test(
    "deselecting every agent installs nothing",
    () => {
      const root = tempRoot();
      drive(
        root,
        [],
        [
          `expect:${AGENTS_QUESTION}`,
          "send:left",
          "send:enter",
          "expect:no agents selected",
        ]
      );

      expect(agentDirsIn(root)).toEqual([]);
      rmSync(root, { force: true, recursive: true });
    },
    PROMPT_TIMEOUT_MS
  );

  test(
    "an explicit mode flag skips the questions even on a terminal",
    () => {
      const root = tempRoot();
      const output = drive(
        root,
        ["--copy"],
        ["expect:installed .agents/skills/klint-rules"]
      );

      expect(output).not.toContain(AGENTS_QUESTION);
      expect(output).not.toContain(STORAGE_QUESTION);
      rmSync(root, { force: true, recursive: true });
    },
    PROMPT_TIMEOUT_MS
  );
});
