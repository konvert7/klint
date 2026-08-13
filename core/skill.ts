import { createHash } from "node:crypto";
import { lstatSync, readFileSync, readlinkSync, realpathSync } from "node:fs";
import { join } from "node:path";
import type { Violation } from "./types";

export const AGENT_SKILL_DIRS = {
  claude: ".claude/skills",
  codex: ".agents/skills",
  copilot: ".agents/skills",
  cursor: ".cursor/skills",
  opencode: ".agents/skills",
} as const;

export type AgentKey = keyof typeof AGENT_SKILL_DIRS;

export const CANONICAL_SKILL_DIR = ".agents/skills";
export const SKILL_DIR_NAME = "klint-rules";
export const SKILL_FILE_NAME = "SKILL.md";
export const RECEIPT_FILE_NAME = ".klint-skill.json";

const STALE_RULE = "klint/skill-stale";
const LEGACY_RULE = "klint/skill-legacy-link";
const LEGACY_TARGET = "node_modules";

export interface SkillReceipt {
  version: string;
  sha256: string;
}

export function skillHash(contents: Buffer | string): string {
  return createHash("sha256").update(contents).digest("hex");
}

export function agentSkillDirs(agents?: readonly AgentKey[]): string[] {
  const selected = (agents ?? (Object.keys(AGENT_SKILL_DIRS) as AgentKey[])).map(
    (agent) => AGENT_SKILL_DIRS[agent]
  );
  return [...new Set(selected)];
}

export function skillAdvisories({
  configDir,
  installed,
  shippedSkillPath,
}: {
  configDir: string;
  installed: string;
  shippedSkillPath: string;
}): Array<Omit<Violation, "fix">> {
  return [
    ...legacyLinkAdvisories(configDir),
    ...stalenessAdvisories({ configDir, installed, shippedSkillPath }),
  ];
}

function legacyLinkAdvisories(configDir: string): Array<Omit<Violation, "fix">> {
  return agentSkillDirs().flatMap((dir) => {
    const target = symlinkTarget(join(configDir, dir, SKILL_DIR_NAME));
    if (target === undefined || !target.includes(LEGACY_TARGET)) return [];
    return [
      {
        file: `${dir}/${SKILL_DIR_NAME}`,
        line: 1,
        rule: LEGACY_RULE,
        severity: "warn",
        message: `this skill is a symlink into ${LEGACY_TARGET} (${target}), which breaks as soon as dependencies are reinstalled — run: klint install-skill`,
      },
    ];
  });
}

function stalenessAdvisories({
  configDir,
  installed,
  shippedSkillPath,
}: {
  configDir: string;
  installed: string;
  shippedSkillPath: string;
}): Array<Omit<Violation, "fix">> {
  const shipped = readShippedHash(shippedSkillPath);
  if (shipped === undefined) return [];

  const reported = new Set<string>();
  return realSkillDirsFirst(configDir).flatMap((dir) => {
    const skillDir = join(configDir, dir, SKILL_DIR_NAME);
    const canonical = resolveSkillDir(skillDir);
    if (canonical === undefined || reported.has(canonical)) return [];

    const receipt = readReceipt(join(skillDir, RECEIPT_FILE_NAME));
    if (receipt === undefined || receipt.sha256 === shipped) return [];

    reported.add(canonical);
    return [staleAdvisory(dir, receipt.version, installed)];
  });
}

function staleAdvisory(
  dir: string,
  installedFrom: string,
  installed: string
): Omit<Violation, "fix"> {
  return {
    file: `${dir}/${SKILL_DIR_NAME}/${SKILL_FILE_NAME}`,
    line: 1,
    rule: STALE_RULE,
    severity: "warn",
    message: `this klint-rules skill was installed from klint ${installedFrom} and no longer matches klint ${installed} — reinstall it with: klint install-skill`,
  };
}

function realSkillDirsFirst(configDir: string): string[] {
  const dirs = agentSkillDirs();
  const linked = new Set(
    dirs.filter(
      (dir) => symlinkTarget(join(configDir, dir, SKILL_DIR_NAME)) !== undefined
    )
  );
  return [...dirs.filter((dir) => !linked.has(dir)), ...linked];
}

function symlinkTarget(path: string): string | undefined {
  try {
    if (!lstatSync(path).isSymbolicLink()) return undefined;
    return readlinkSync(path);
  } catch {
    return undefined;
  }
}

function resolveSkillDir(path: string): string | undefined {
  try {
    return realpathSync(path);
  } catch {
    return undefined;
  }
}

function readShippedHash(shippedSkillPath: string): string | undefined {
  try {
    return skillHash(readFileSync(shippedSkillPath));
  } catch {
    return undefined;
  }
}

function readReceipt(path: string): SkillReceipt | undefined {
  let text: string;
  try {
    text = readFileSync(path, "utf-8");
  } catch {
    return undefined;
  }

  try {
    const parsed = JSON.parse(text) as Partial<SkillReceipt>;
    if (typeof parsed.version !== "string" || typeof parsed.sha256 !== "string") {
      return undefined;
    }
    return { version: parsed.version, sha256: parsed.sha256 };
  } catch {
    return undefined;
  }
}
