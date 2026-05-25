import { readdirSync, readFileSync } from "node:fs";
import { join, relative, resolve } from "node:path";
import { BUILT_IN_PLUGINS } from "../plugins/index";
import { BUILT_IN_RULES } from "../rules/index";
import { runArchRules } from "./arch";
import { clearAstCache } from "./ast";
import type {
  KlintConfig,
  KlintRule,
  RuleConfigValue,
  Severity,
  Violation,
} from "./types";

export type KlintDebugEvent =
  | { type: "walk:start"; dir: string }
  | { type: "walk:done"; dir: string; files: number }
  | { type: "files:resolved"; files: number }
  | { type: "rule:start"; rule: string; files: number }
  | { type: "rule:done"; rule: string; violations: number }
  | { type: "arch:start"; files: number }
  | { type: "arch:done"; violations: number };

export interface RunKlintOptions {
  onDebug?: (event: KlintDebugEvent) => void;
}

function walk(dir: string, root: string, excludes: string[] = []): string[] {
  const out: string[] = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    const rel = relative(root, full).replaceAll("\\", "/");
    if (entry.isDirectory()) {
      if (
        excludes.some(
          (pattern) => matchPattern(rel, pattern) || matchPattern(`${rel}/`, pattern)
        )
      )
        continue;
      out.push(...walk(full, root, excludes));
    } else if (/\.(tsx?|jsx?|mts|cts)$/.test(entry.name))
      out.push(full.replaceAll("\\", "/"));
  }
  return out;
}

function resolveFiles(
  include: string[],
  root: string,
  onDebug?: (event: KlintDebugEvent) => void
): string[] {
  const files = new Set<string>();
  const excludes = include.filter((p) => p.startsWith("!")).map((p) => p.slice(1));
  for (const pattern of include) {
    if (pattern.startsWith("!")) continue;
    const base = pattern.split("/**")[0].split("/*")[0];
    const dir = resolve(root, base === "**" ? "." : base);
    try {
      onDebug?.({ type: "walk:start", dir });
      const before = files.size;
      for (const file of walk(dir, root, excludes)) files.add(file);
      onDebug?.({ type: "walk:done", dir, files: files.size - before });
    } catch {
      // directory doesn't exist — skip
    }
  }
  return [...files];
}

function matchPattern(relPath: string, pattern: string): boolean {
  const norm = relPath.replaceAll("\\", "/");
  const p = pattern.replaceAll("\\", "/");
  if (p === "." || p === "**") return true;
  if (!p.includes("*")) return norm === p || norm.startsWith(`${p}/`);
  return globToRegExp(p).test(norm);
}

function globToRegExp(pattern: string): RegExp {
  let source = "";
  for (let i = 0; i < pattern.length; i++) {
    const char = pattern[i];
    const next = pattern[i + 1];

    if (char === "*" && next === "*") {
      const after = pattern[i + 2];
      if (after === "/") {
        source += "(?:.*/)?";
        i += 2;
      } else {
        source += ".*";
        i++;
      }
      continue;
    }

    if (char === "*") {
      source += "[^/]*";
      continue;
    }

    source += /[.+?^${}()|[\]\\]/.test(char) ? `\\${char}` : char;
  }
  return new RegExp(`^${source}$`);
}

function applyPatterns(files: string[], patterns: string[], root: string): string[] {
  const includes = patterns.filter((p) => !p.startsWith("!"));
  const excludes = patterns.filter((p) => p.startsWith("!")).map((p) => p.slice(1));
  return files.filter((file) => {
    const rel = relative(root, file).replaceAll("\\", "/");
    const included = includes.length === 0 || includes.some((p) => matchPattern(rel, p));
    const excluded = excludes.some((p) => matchPattern(rel, p));
    return included && !excluded;
  });
}

function resolveSeverity(value: RuleConfigValue): Severity {
  return typeof value === "string" ? value : (value.severity ?? "error");
}

function resolveInclude(value: RuleConfigValue): string[] | undefined {
  return typeof value === "object" ? value.include : undefined;
}

export function runKlint(
  config: KlintConfig,
  customRules: Record<string, KlintRule> = {},
  options: RunKlintOptions = {}
): Violation[] {
  clearAstCache();

  // All plugin implementations are always available by their prefixed names
  const pluginImpls: Record<string, KlintRule> = Object.assign(
    {},
    ...Object.values(BUILT_IN_PLUGINS).map((p) => p.implementations)
  );
  const registry: Record<string, KlintRule> = {
    ...BUILT_IN_RULES,
    ...pluginImpls,
    ...customRules,
  };

  // Plugin defaults applied first; explicit rules take precedence
  const pluginDefaults: Record<string, RuleConfigValue> = {};
  for (const pluginName of config.plugins ?? []) {
    const plugin = BUILT_IN_PLUGINS[pluginName];
    if (!plugin) throw new Error(`Unknown klint plugin: "${pluginName}"`);
    Object.assign(pluginDefaults, plugin.rules);
  }
  const effectiveRules: Record<string, RuleConfigValue> = {
    ...pluginDefaults,
    ...config.rules,
  };

  const allFiles = applyPatterns(
    resolveFiles(config.include, config.root, options.onDebug),
    config.include,
    config.root
  );
  options.onDebug?.({ type: "files:resolved", files: allFiles.length });
  const fileContents = new Map(allFiles.map((f) => [f, readFileSync(f, "utf-8")]));
  const violations: Violation[] = [];

  for (const [ruleName, configValue] of Object.entries(effectiveRules)) {
    const severity = resolveSeverity(configValue);
    if (severity === "off") continue;

    const rule = registry[ruleName];
    if (!rule) throw new Error(`Unknown klint rule: "${ruleName}"`);

    const include = resolveInclude(configValue);
    const files = include ? applyPatterns(allFiles, include, config.root) : allFiles;

    const batch: Omit<Violation, "severity">[] = [];
    options.onDebug?.({ type: "rule:start", rule: ruleName, files: files.length });
    rule.check({ files, root: config.root, fileContents }, batch);
    options.onDebug?.({ type: "rule:done", rule: ruleName, violations: batch.length });
    for (const v of batch) violations.push({ ...v, rule: ruleName, severity });
  }

  if (config.arch) {
    options.onDebug?.({ type: "arch:start", files: allFiles.length });
    const archViolations = runArchRules(config.arch, allFiles, fileContents, config.root);
    options.onDebug?.({ type: "arch:done", violations: archViolations.length });
    violations.push(...archViolations);
  }

  return violations;
}
