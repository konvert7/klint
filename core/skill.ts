import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import type { Violation } from "./types";

export const AGENT_SKILL_DIRS = {
  claude: ".claude/skills",
  codex: ".agents/skills",
  cursor: ".cursor/skills",
  opencode: ".agents/skills",
} as const;

export type AgentKey = keyof typeof AGENT_SKILL_DIRS;

export const SKILL_DIR_NAME = "klint-rules";
export const SKILL_FILE_NAME = "SKILL.md";
export const RECEIPT_FILE_NAME = ".klint-skill.json";

const STALE_RULE = "klint/skill-stale";

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

export function skillStalenessAdvisories({
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

  return agentSkillDirs().flatMap((dir) => {
    const receipt = readReceipt(join(configDir, dir, SKILL_DIR_NAME, RECEIPT_FILE_NAME));
    if (receipt === undefined || receipt.sha256 === shipped) return [];
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
