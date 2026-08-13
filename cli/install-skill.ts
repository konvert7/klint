import { existsSync } from "node:fs";
import { cp, lstat, mkdir, readFile, rm, symlink, writeFile } from "node:fs/promises";
import { dirname, join, relative, resolve } from "node:path";
import * as clack from "@clack/prompts";
import {
  AGENT_SKILL_DIRS,
  type AgentKey,
  agentSkillDirs,
  CANONICAL_SKILL_DIR,
  RECEIPT_FILE_NAME,
  SKILL_DIR_NAME,
  SKILL_FILE_NAME,
  type SkillReceipt,
  skillHash,
} from "../core/skill";
import { installedVersion, PACKAGE_ROOT } from "./paths";

const AGENT_TARGETS = [
  { value: "claude", label: "Claude Code" },
  { value: "opencode", label: "opencode" },
  { value: "cursor", label: "Cursor" },
  { value: "codex", label: "Codex" },
] as const satisfies ReadonlyArray<{ value: AgentKey; label: string }>;

interface InstallPlan {
  agents: AgentKey[];
  shared: boolean;
  force: boolean;
}

export async function installSkill(args: string[]): Promise<void> {
  const skillSrc = join(PACKAGE_ROOT, "skill", SKILL_DIR_NAME);
  if (!existsSync(skillSrc)) {
    process.stderr.write(`klint: skill source not found at ${skillSrc}\n`);
    process.exit(1);
  }

  const plan = await resolvePlan(args);
  const dirs = agentSkillDirs(plan.agents);

  try {
    const report = plan.shared
      ? await installShared({ dirs, force: plan.force, skillSrc })
      : await installCopies({ dirs, force: plan.force, skillSrc });
    for (const line of report) process.stdout.write(`klint: ${line}\n`);
  } catch (error) {
    process.stderr.write(`${(error as Error).message}\n`);
    process.exit(1);
  }

  if (process.stdin.isTTY) {
    clack.outro("Done.");
  }
}

async function installShared({
  dirs,
  force,
  skillSrc,
}: {
  dirs: string[];
  force: boolean;
  skillSrc: string;
}): Promise<string[]> {
  const hub = join(CANONICAL_SKILL_DIR, SKILL_DIR_NAME);
  const report = [await installCopy(hub, skillSrc, force)];

  for (const dir of dirs) {
    if (dir === CANONICAL_SKILL_DIR) continue;
    report.push(await linkToHub(join(dir, SKILL_DIR_NAME), hub, skillSrc, force));
  }
  return report;
}

async function installCopies({
  dirs,
  force,
  skillSrc,
}: {
  dirs: string[];
  force: boolean;
  skillSrc: string;
}): Promise<string[]> {
  const report: string[] = [];
  for (const dir of dirs) {
    report.push(await installCopy(join(dir, SKILL_DIR_NAME), skillSrc, force));
  }
  return report;
}

async function installCopy(
  target: string,
  skillSrc: string,
  force: boolean
): Promise<string> {
  const dest = resolve(process.cwd(), target);
  await prepareDestination(dest, target, force);
  await cp(skillSrc, dest, { recursive: true });
  await writeReceipt(dest, skillSrc);
  return `installed ${target}`;
}

async function linkToHub(
  target: string,
  hub: string,
  skillSrc: string,
  force: boolean
): Promise<string> {
  const dest = resolve(process.cwd(), target);
  await prepareDestination(dest, target, force);

  const linkPath = relative(dirname(dest), resolve(process.cwd(), hub));
  try {
    await symlink(linkPath, dest, process.platform === "win32" ? "junction" : "dir");
    return `linked ${target} -> ${linkPath}`;
  } catch {
    await cp(skillSrc, dest, { recursive: true });
    await writeReceipt(dest, skillSrc);
    return `installed ${target} (symlinks unavailable here)`;
  }
}

async function prepareDestination(
  dest: string,
  target: string,
  force: boolean
): Promise<void> {
  await mkdir(dirname(dest), { recursive: true });

  const existing = await lstat(dest).catch(() => undefined);
  if (existing === undefined) return;

  if (!existing.isDirectory() || existsSync(join(dest, RECEIPT_FILE_NAME)) || force) {
    await rm(dest, { force: true, recursive: true });
    return;
  }

  throw new Error(
    `klint: ${target} exists and was not installed by klint — pass --force to replace it`
  );
}

async function writeReceipt(dest: string, skillSrc: string): Promise<void> {
  const receipt: SkillReceipt = {
    version: installedVersion(),
    sha256: skillHash(await readFile(join(skillSrc, SKILL_FILE_NAME))),
  };
  await writeFile(join(dest, RECEIPT_FILE_NAME), `${JSON.stringify(receipt, null, 2)}\n`);
}

async function resolvePlan(args: string[]): Promise<InstallPlan> {
  const flags = parseFlags(args);
  const decided = flags.agents !== undefined || flags.shared !== undefined;

  if (!process.stdin.isTTY || decided) {
    return {
      agents: flags.agents ?? everyAgent(),
      shared: flags.shared ?? true,
      force: flags.force,
    };
  }

  clack.intro("klint install-skill");
  return {
    agents: await promptForAgents(),
    shared: await promptForShared(),
    force: flags.force,
  };
}

function parseFlags(args: string[]): {
  agents?: AgentKey[];
  shared?: boolean;
  force: boolean;
} {
  let agents: AgentKey[] | undefined;
  let shared: boolean | undefined;
  let force = false;

  for (let i = 0; i < args.length; i++) {
    if (args[i] === "--agents" && args[i + 1]) agents = parseAgents(args[++i]);
    else if (args[i] === "--symlink" || args[i] === "--shared") shared = true;
    else if (args[i] === "--copy") shared = false;
    else if (args[i] === "--force") force = true;
    else {
      process.stderr.write(`klint: unknown install-skill argument "${args[i]}"\n`);
      process.exit(1);
    }
  }

  return { agents, force, shared };
}

function parseAgents(value: string): AgentKey[] {
  return value
    .split(",")
    .map((agent) => agent.trim())
    .filter((agent) => agent.length > 0)
    .map((agent) => {
      if (!(agent in AGENT_SKILL_DIRS)) {
        process.stderr.write(
          `klint: unknown agent "${agent}" (expected one of: ${everyAgent().join(", ")})\n`
        );
        process.exit(1);
      }
      return agent as AgentKey;
    });
}

function everyAgent(): AgentKey[] {
  return AGENT_TARGETS.map((target) => target.value);
}

async function promptForAgents(): Promise<AgentKey[]> {
  const agents = await clack.multiselect<AgentKey>({
    message: "Which agents should the skill be installed for?",
    options: AGENT_TARGETS.map((a) => ({ value: a.value, label: a.label })),
    initialValues: everyAgent(),
  });
  if (clack.isCancel(agents)) {
    clack.cancel("Cancelled.");
    process.exit(0);
  }
  return agents as AgentKey[];
}

async function promptForShared(): Promise<boolean> {
  const mode = await clack.select<"shared" | "copy">({
    message: "How should the skill be stored?",
    initialValue: "shared",
    options: [
      {
        value: "shared",
        label: "Shared (recommended)",
        hint: `one skill in ${CANONICAL_SKILL_DIR}, other agents symlink to it`,
      },
      {
        value: "copy",
        label: "Separate copies",
        hint: "an independent copy in every agent directory",
      },
    ],
  });
  if (clack.isCancel(mode)) {
    clack.cancel("Cancelled.");
    process.exit(0);
  }
  return mode === "shared";
}
