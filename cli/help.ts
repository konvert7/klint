import { BUILT_IN_PLUGINS } from "../plugins/index";
import { BUILT_IN_RULES } from "../rules/index";

export function printHelp(): void {
  const pluginRules = new Set(
    Object.values(BUILT_IN_PLUGINS).flatMap((p) => Object.keys(p.rules))
  );
  const standaloneRules = Object.keys(BUILT_IN_RULES).filter((r) => !pluginRules.has(r));
  const pluginEntries = Object.entries(BUILT_IN_PLUGINS);

  process.stdout.write(
    [
      "klint — agent harness for TypeScript architecture rules",
      "",
      "Usage: klint [--config <dir>] [--rules <file>] [--engine ts|rust|compare|auto] [--fix] [--json]",
      "       klint install-skill [--agents <list>] [--symlink | --copy] [--force]",
      "",
      "  --config <dir>   directory containing klint.yaml or klint.config.json (default: cwd)",
      "  --rules  <file>  custom rules file (default: <configDir>/klint.rules.ts if present)",
      "  --engine <name>  engine to use: ts (default), rust, compare, or auto (experimental)",
      "  --fix            apply auto-fixes for fixable violations in-place",
      "  --json           emit structured JSON to stdout (for agent/CI consumption)",
      "  --debug          print file resolution and rule progress to stderr",
      "",
      "  install-skill    install the rule-authoring skill into agent config directories",
      "                   --agents <list>  comma-separated: claude,opencode,cursor,codex (default: all)",
      "                   --symlink        keep one skill in .agents/skills and symlink the",
      "                                    other agents to it (default)",
      "                   --copy           give every agent directory its own copy",
      "                   --force          replace a skill directory klint did not install",
      "",
      "  Installed skills record the version and hash they came from. klint warns with",
      "  klint/skill-stale once the shipped skill moves on, and with",
      "  klint/skill-legacy-link for old symlinks that point into node_modules.",
      "",
      `Built-in rules (${standaloneRules.length}):`,
      ...standaloneRules.map((r) => `  ${r}`),
      "",
      `Plugins (${pluginEntries.length}):`,
      ...pluginEntries.flatMap(([name, plugin]) => [
        `  ${name}`,
        ...Object.keys(plugin.rules).map((r) => `    ${r}`),
      ]),
      "",
    ].join("\n")
  );
}
