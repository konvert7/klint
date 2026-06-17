#!/usr/bin/env bun
import { existsSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { parse as parseYaml } from "yaml";
import { printHelp } from "./cli/help";
import { installSkill } from "./cli/install-skill";
import {
  formatDuration,
  toJsonCliResult,
  writeDebugEvent,
  writeTextOutput,
} from "./cli/output";
import { runAutoEngine, runCompareEngine, runRustEngine } from "./cli/rust-engine";
import { applyFixes } from "./core/fixer";
import { runKlint } from "./core/runner";
import type { ArchConfig, KlintConfig, KlintRule, RuleConfigValue } from "./core/types";

interface CliOptions {
  configDir?: string;
  rulesFile?: string;
}

export async function main(opts: CliOptions = {}): Promise<void> {
  const startedAt = performance.now();
  const args = process.argv.slice(2);

  if (args[0] === "help" || args[0] === "h") {
    printHelp();
    process.exit(0);
  }

  if (args[0] === "install-skill") {
    await installSkill(args.slice(1));
    return;
  }

  let configDir = opts.configDir;
  let rulesFile = opts.rulesFile;
  let fix = false;
  let json = false;
  let debug = false;
  let engine = process.env.KLINT_ENGINE;

  for (let i = 0; i < args.length; i++) {
    if (args[i] === "--config" && args[i + 1]) configDir = resolve(args[++i]);
    else if (args[i] === "--rules" && args[i + 1]) rulesFile = resolve(args[++i]);
    else if (args[i] === "--engine" && args[i + 1]) engine = args[++i];
    else if (args[i] === "--fix") fix = true;
    else if (args[i] === "--json") json = true;
    else if (args[i] === "--debug" || args[i] === "-debug") debug = true;
    else if (args[i] === "--help" || args[i] === "-h") {
      printHelp();
      process.exit(0);
    }
  }

  configDir ??= process.cwd();

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
    root?: string;
    include?: string[];
    plugins?: string[];
    rules?: Record<string, RuleConfigValue>;
    arch?: unknown;
  }
  let raw: RawConfig;
  try {
    const text = await readFile(configPath, "utf-8");
    raw = (usingYaml ? parseYaml(text) : JSON.parse(text)) as RawConfig;
  } catch {
    process.stderr.write(`klint: failed to parse ${configPath}\n`);
    process.exit(1);
  }
  const root = resolve(configDir, raw.root ?? ".");

  if (engine === "rust") {
    runRustEngine({ fix, json, raw, root, rulesFile, startedAt });
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
      fix,
      json,
      raw,
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
      fix,
      json,
      raw,
      root,
      rulesFile,
      tsViolations: violations,
    });
    return;
  }

  if (json) {
    const tsResult = toJsonCliResult(violations);
    process.stdout.write(tsResult.stdout);
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
    const unfixed = current.filter((v) => !v.fix).length;
    const msg =
      unfixed > 0
        ? `klint: applied ${totalApplied} fix(es). ${unfixed} violation(s) require manual attention.`
        : `klint: applied ${totalApplied} fix(es). No remaining violations.`;
    process.stdout.write(`${msg} Finished in ${formatDuration(startedAt)}.\n`);
    process.exit(0);
  }
  writeTextOutput(violations, startedAt);
}

if (import.meta.main) await main();
