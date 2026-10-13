#!/usr/bin/env bun
import { existsSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { parse as parseYaml } from "yaml";
import { type OptionValues, runCommand } from "./cli/command";
import { klintCommand } from "./cli/commands";
import { installSkill } from "./cli/install-skill";
import {
  formatDuration,
  toJsonCliResult,
  withAdvisories,
  writeDebugEvent,
  writeTextOutput,
} from "./cli/output";
import { installedVersion, SHIPPED_SKILL_PATH } from "./cli/paths";
import { runAutoEngine, runCompareEngine, runRustEngine } from "./cli/rust-engine";
import { printStats } from "./cli/stats";
import { applyFixes } from "./core/fixer";
import { runKlint } from "./core/runner";
import { schemaVersionAdvisory } from "./core/schema-version";
import { skillAdvisories } from "./core/skill";
import { runRecorder } from "./core/stats";
import type { ArchConfig, KlintConfig, KlintRule, RuleConfigValue } from "./core/types";

export async function main(): Promise<number> {
  const startedAt = performance.now();
  const command = klintCommand({
    lint: (values) => lint(values, startedAt),
    stats: printStats,
    installSkill,
  });
  return runCommand(command, process.argv.slice(2), ["klint"]);
}

function optionalPath(value: string | true | undefined): string | undefined {
  return typeof value === "string" ? resolve(value) : undefined;
}

async function lint(values: OptionValues, startedAt: number): Promise<void> {
  if (values.version) {
    process.stdout.write(`klint ${installedVersion()}\n`);
    return;
  }
  const rulesFile = optionalPath(values.rules);
  const fix = values.fix === true;
  const json = values.json === true;
  const debug = values.debug === true;
  const hook = values.hook === true;
  const engine =
    typeof values.engine === "string" ? values.engine : process.env.KLINT_ENGINE;
  const configDir = optionalPath(values.config) ?? process.cwd();
  const record = runRecorder(configDir, hook ? "hook" : "direct");

  const yamlPath = resolve(configDir, "klint.yaml");
  const jsonPath = resolve(configDir, "klint.config.json");
  const usingYaml = existsSync(yamlPath);
  const configPath = usingYaml ? yamlPath : jsonPath;

  if (!existsSync(configPath)) {
    process.stderr.write(
      `klint: no config file found — create klint.yaml (or klint.config.json) at ${configDir}\n`
    );
    process.exit(1);
  }

  interface RawConfig {
    $schema?: string;
    root?: string;
    include?: string[];
    plugins?: string[];
    rules?: Record<string, RuleConfigValue>;
    arch?: unknown;
  }
  let raw: RawConfig;
  let configText = "";
  try {
    configText = await readFile(configPath, "utf-8");
    raw = (usingYaml ? parseYaml(configText) : JSON.parse(configText)) as RawConfig;
  } catch {
    process.stderr.write(`klint: failed to parse ${configPath}\n`);
    process.exit(1);
  }
  const root = resolve(configDir, raw.root ?? ".");

  const installed = installedVersion();
  const advisories = [
    ...schemaVersionAdvisory({
      schema: raw.$schema,
      installed,
      configFile: usingYaml ? "klint.yaml" : "klint.config.json",
      configText,
    }),
    ...skillAdvisories({
      configDir,
      installed,
      shippedSkillPath: SHIPPED_SKILL_PATH,
    }),
  ];

  if (engine === "rust") {
    runRustEngine({ advisories, fix, json, raw, record, root, rulesFile, startedAt });
    return;
  }
  if (
    engine !== undefined &&
    engine !== "ts" &&
    engine !== "rust" &&
    engine !== "compare" &&
    engine !== "auto"
  ) {
    process.stderr.write(
      `klint: unknown engine "${engine}" (expected "ts", "rust", "compare", or "auto")\n`
    );
    process.exit(1);
  }

  let customRules: Record<string, KlintRule> = {};
  const defaultRulesPath = resolve(configDir, "klint.rules.ts");
  const rulesPath =
    rulesFile ?? (existsSync(defaultRulesPath) ? defaultRulesPath : undefined);
  if (rulesPath) {
    const mod = await import(rulesPath);
    customRules = (mod.default ?? {}) as Record<string, KlintRule>;
  }

  const customRulesMap: Record<string, RuleConfigValue> = Object.fromEntries(
    Object.keys(customRules).map((name) => [name, "error" as const])
  );
  const allRules: KlintConfig["rules"] = { ...customRulesMap, ...(raw.rules ?? {}) };

  if (engine === "auto") {
    runAutoEngine({
      advisories,
      fix,
      json,
      raw,
      record,
      root,
      rulesFile,
      startedAt,
      customRules,
      customRulesMap,
    });
    return;
  }

  const violations = runKlint(
    {
      root,
      include: raw.include ?? ["."],
      plugins: raw.plugins,
      rules: allRules,
      arch: raw.arch as ArchConfig | undefined,
    },
    customRules,
    { onDebug: debug ? writeDebugEvent : undefined }
  );

  if (engine === "compare") {
    runCompareEngine({
      advisories,
      fix,
      json,
      raw,
      record,
      root,
      rulesFile,
      tsViolations: violations,
    });
    return;
  }

  if (json) {
    record(violations);
    const tsResult = toJsonCliResult(violations);
    process.stdout.write(withAdvisories(tsResult.stdout, advisories));
    process.exit(tsResult.status);
  }

  if (fix) {
    let totalApplied = 0;
    let current = violations;
    while (true) {
      const applied = applyFixes(current, root);
      totalApplied += applied;
      if (applied === 0) break;
      current = runKlint(
        {
          root,
          include: raw.include ?? ["."],
          plugins: raw.plugins,
          rules: allRules,
          arch: raw.arch as ArchConfig | undefined,
        },
        customRules,
        { onDebug: debug ? writeDebugEvent : undefined }
      );
      if (current.every((v) => !v.fix)) break;
    }
    record(current);
    const unfixed = current.filter((v) => !v.fix).length;
    const msg =
      unfixed > 0
        ? `klint: applied ${totalApplied} fix(es). ${unfixed} violation(s) require manual attention.`
        : `klint: applied ${totalApplied} fix(es). No remaining violations.`;
    process.stdout.write(`${msg} Finished in ${formatDuration(startedAt)}.\n`);
    process.exit(0);
  }
  record(violations);
  writeTextOutput([...violations, ...advisories], startedAt);
}

if (import.meta.main) process.exitCode = await main();
