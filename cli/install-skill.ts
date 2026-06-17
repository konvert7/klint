import { existsSync } from "node:fs";
import { cp, mkdir, rm, symlink } from "node:fs/promises";
import { dirname, join, relative, resolve } from "node:path";
import * as clack from "@clack/prompts";
import { PACKAGE_ROOT } from "./paths";

const AGENT_TARGETS = [
  { value: "claude", label: "Claude Code" },
  { value: "opencode", label: "opencode" },
  { value: "cursor", label: "Cursor" },
  { value: "codex", label: "Codex" },
] as const;

type AgentKey = (typeof AGENT_TARGETS)[number]["value"];

const AGENT_DIRS: Record<AgentKey, string> = {
  claude: ".claude/skills",
  opencode: ".agents/skills",
  cursor: ".cursor/skills",
  codex: ".agents/skills",
};

export async function installSkill(args: string[]): Promise<void> {
  const skillSrc = join(PACKAGE_ROOT, "skill", "klint-rules");
  if (!existsSync(skillSrc)) {
    process.stderr.write(`klint: skill source not found at ${skillSrc}\n`);
    process.exit(1);
  }

  // Parse non-interactive flags
  let flagAgents: AgentKey[] | undefined;
  let flagSymlink: boolean | undefined;
  for (let i = 0; i < args.length; i++) {
    if (args[i] === "--agents" && args[i + 1])
      flagAgents = args[++i].split(",") as AgentKey[];
    else if (args[i] === "--symlink") flagSymlink = true;
    else if (args[i] === "--copy") flagSymlink = false;
  }

  let selectedAgents: AgentKey[];
  let useSymlink: boolean;

  if (!process.stdin.isTTY || flagAgents !== undefined || flagSymlink !== undefined) {
    selectedAgents = flagAgents ?? (AGENT_TARGETS.map((a) => a.value) as AgentKey[]);
    useSymlink = flagSymlink ?? false;
  } else {
    clack.intro("klint install-skill");

    const agents = await clack.multiselect<AgentKey>({
      message: "Which agents should the skill be installed for?",
      options: AGENT_TARGETS.map((a) => ({ value: a.value, label: a.label })),
      initialValues: AGENT_TARGETS.map((a) => a.value) as AgentKey[],
    });
    if (clack.isCancel(agents)) {
      clack.cancel("Cancelled.");
      process.exit(0);
    }
    selectedAgents = agents as AgentKey[];

    const mode = await clack.select<"symlink" | "copy">({
      message: "Install as symlink or copy?",
      options: [
        {
          value: "symlink",
          label: "Symlink",
          hint: "stays in sync when klint updates",
        },
        {
          value: "copy",
          label: "Copy",
          hint: "one-time snapshot, no ongoing dependency",
        },
      ],
    });
    if (clack.isCancel(mode)) {
      clack.cancel("Cancelled.");
      process.exit(0);
    }
    useSymlink = mode === "symlink";
  }

  const cwd = process.cwd();
  const linkType = process.platform === "win32" ? "junction" : "dir";
  for (const key of selectedAgents) {
    const dest = resolve(cwd, AGENT_DIRS[key], "klint-rules");
    await mkdir(dirname(dest), { recursive: true });
    try {
      await rm(dest, { recursive: true, force: true });
    } catch {
      /* already gone */
    }
    if (useSymlink) {
      await symlink(relative(dirname(dest), skillSrc), dest, linkType);
    } else {
      await cp(skillSrc, dest, { recursive: true });
    }
  }

  if (process.stdin.isTTY) {
    clack.outro("Done.");
  }
}
