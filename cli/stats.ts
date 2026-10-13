import { resolve } from "node:path";
import { type KlintStats, readStats, STATS_DIR, STATS_FILE } from "../core/stats";

const RULE_HEADING = "by rule";
const REPORTED_HEADING = "reported";
const PREVENTED_HEADING = "prevented";
const LABEL_WIDTH = "violations reported".length;

interface RuleRow {
  rule: string;
  reported: number;
  prevented: number;
}

function ruleRows(stats: KlintStats): RuleRow[] {
  const rules = new Set([
    ...Object.keys(stats.reported.byRule),
    ...Object.keys(stats.prevented.byRule),
  ]);
  const byCodePoint = (a: string, b: string) => Number(a > b) - Number(a < b);
  return [...rules]
    .map((rule) => ({
      rule,
      reported: stats.reported.byRule[rule] ?? 0,
      prevented: stats.prevented.byRule[rule] ?? 0,
    }))
    .sort(
      (a, b) =>
        b.reported - a.reported ||
        b.prevented - a.prevented ||
        byCodePoint(a.rule, b.rule)
    );
}

interface Summary {
  label: string;
  total: number;
  detail: string;
}

function summaryLines(summaries: Summary[]): string[] {
  const width = Math.max(...summaries.map(({ total }) => String(total).length));
  return summaries.map(
    ({ label, total, detail }) =>
      `  ${label.padEnd(LABEL_WIDTH)}  ${String(total).padStart(width)}  (${detail})`
  );
}

function ruleTable(rows: RuleRow[]): string[] {
  const width = Math.max(RULE_HEADING.length - 2, ...rows.map(({ rule }) => rule.length));
  const heading = `  ${RULE_HEADING.padEnd(width + 2)}  ${REPORTED_HEADING}  ${PREVENTED_HEADING}`;
  const lines = rows.map(
    ({ rule, reported, prevented }) =>
      `    ${rule.padEnd(width)}  ${String(reported).padStart(REPORTED_HEADING.length)}  ${String(prevented).padStart(PREVENTED_HEADING.length)}`
  );
  return [heading, ...lines];
}

export function formatStats(stats: KlintStats): string {
  const { runs, reported, prevented } = stats;
  const lines = [
    `klint stats (${STATS_DIR}/${STATS_FILE})`,
    "",
    ...summaryLines([
      {
        label: "runs",
        total: runs.hook + runs.direct,
        detail: `hook ${runs.hook}, direct ${runs.direct}`,
      },
      {
        label: "violations reported",
        total: reported.bySource.hook + reported.bySource.direct,
        detail: `hook ${reported.bySource.hook}, direct ${reported.bySource.direct}`,
      },
      {
        label: "errors prevented",
        total: prevented.bySource.hook + prevented.bySource.direct,
        detail: `caught by hook ${prevented.bySource.hook}, direct ${prevented.bySource.direct}`,
      },
    ]),
  ];
  const rows = ruleRows(stats);
  if (rows.length > 0) lines.push("", ...ruleTable(rows));
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
