import { BUILT_IN_PLUGINS } from "../plugins/index";
import { BUILT_IN_RULES } from "../rules/index";

export function builtInRuleLines(): string[] {
  const pluginRules = new Set(
    Object.values(BUILT_IN_PLUGINS).flatMap((p) => Object.keys(p.rules))
  );
  const standaloneRules = Object.keys(BUILT_IN_RULES).filter((r) => !pluginRules.has(r));
  const pluginEntries = Object.entries(BUILT_IN_PLUGINS);

  return [
    `Built-in rules (${standaloneRules.length}):`,
    ...standaloneRules.map((r) => `  ${r}`),
    "",
    `Plugins (${pluginEntries.length}):`,
    ...pluginEntries.flatMap(([name, plugin]) => [
      `  ${name}`,
      ...Object.keys(plugin.rules).map((r) => `    ${r}`),
    ]),
  ];
}
