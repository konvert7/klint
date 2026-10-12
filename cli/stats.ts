import { resolve } from "node:path";
import { type KlintStats, readStats, STATS_DIR, STATS_FILE } from "../core/stats";

function rulesByCount(byRule: Record<string, number>): Array<[string, number]> {
  const byCodePoint = (a: string, b: string) => Number(a > b) - Number(a < b);
  return Object.entries(byRule).sort(([a, x], [b, y]) => y - x || byCodePoint(a, b));
}

export function formatStats(stats: KlintStats): string {
  const runs = stats.runs.hook + stats.runs.direct;
  const prevented = stats.prevented.bySource.hook + stats.prevented.bySource.direct;
  const rules = rulesByCount(stats.prevented.byRule);
  const width = Math.max(0, ...rules.map(([rule]) => rule.length));
  const lines = [
    `klint stats (${STATS_DIR}/${STATS_FILE})`,
    "",
    `  runs              ${runs}  (hook ${stats.runs.hook}, direct ${stats.runs.direct})`,
    `  errors prevented  ${prevented}  (caught by hook ${stats.prevented.bySource.hook}, direct ${stats.prevented.bySource.direct})`,
  ];
  if (rules.length > 0) {
    lines.push("", "  prevented by rule");
    for (const [rule, count] of rules) lines.push(`    ${rule.padEnd(width)}  ${count}`);
  }
  return `${lines.join("\n")}\n`;
}

export function printStats(args: string[]): void {
  const configFlag = args.indexOf("--config");
  const configDir =
    configFlag !== -1 && args[configFlag + 1]
      ? resolve(args[configFlag + 1])
      : process.cwd();
  process.stdout.write(formatStats(readStats(configDir)));
}
