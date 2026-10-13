import { AGENT_SKILL_DIRS } from "../core/skill";
import type { Group, OptionValues } from "./command";
import { builtInRuleLines } from "./help";

interface CommandHandlers {
  lint(values: OptionValues): unknown;
  stats(values: OptionValues): unknown;
  installSkill(values: OptionValues): unknown;
}

export function klintCommand(handlers: CommandHandlers): Group {
  return {
    kind: "group",
    summary:
      "Architecture-as-Code linter: layer boundaries, import rules and forbidden patterns",
    aliases: { h: "help" },
    defaultCommand: {
      kind: "leaf",
      summary: "Lints the project",
      options: {
        config: {
          value: "<dir>",
          description:
            "Directory containing klint.yaml or klint.config.json (default: cwd)",
        },
        rules: {
          value: "<file>",
          description:
            "Custom rules file (default: <configDir>/klint.rules.ts if present)",
        },
        engine: {
          value: "<name>",
          description:
            "Engine to use: ts (default), rust, compare, or auto (experimental)",
        },
        fix: { description: "Apply auto-fixes for fixable violations in place" },
        json: { description: "Emit structured JSON to stdout (for agent and CI use)" },
        debug: {
          aliases: ["-debug"],
          description: "Print file resolution and rule progress to stderr",
        },
        hook: { description: "Count this run as an agent hook run in klint stats" },
        version: { short: "V", description: "Print the installed klint version" },
      },
      run: handlers.lint,
    },
    commands: {
      stats: {
        kind: "leaf",
        summary:
          "Print runs, violations reported and errors prevented, in total and per rule",
        options: {
          config: {
            value: "<dir>",
            description: "Directory holding .klint/stats.json (default: cwd)",
          },
        },
        details: () => ["Set KLINT_NO_STATS=1 to stop recording."],
        run: handlers.stats,
      },
      "install-skill": {
        kind: "leaf",
        summary: "Install the rule-authoring skill into agent config directories",
        options: {
          agents: {
            value: "<list>",
            description: `Comma-separated: ${Object.keys(AGENT_SKILL_DIRS).join(",")} (default: all)`,
          },
          symlink: {
            aliases: ["--shared"],
            description:
              "Keep one skill in .agents/skills and symlink the others to it (default)",
          },
          copy: { description: "Give every agent directory its own copy" },
          force: { description: "Replace a skill directory klint did not install" },
        },
        details: () => [
          "Asks which agents and how to store the skill when run on a terminal without",
          "--agents, --symlink or --copy.",
          "",
          "Installed skills record the version and hash they came from. klint warns with",
          "klint/skill-stale once the shipped skill moves on, and with",
          "klint/skill-legacy-link for old symlinks that point into node_modules.",
        ],
        run: handlers.installSkill,
      },
    },
    details: builtInRuleLines,
  };
}
