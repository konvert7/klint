#!/usr/bin/env bun
import { spawnSync } from "node:child_process";
import {
  cpSync,
  existsSync,
  mkdirSync,
  readFileSync,
  rmSync,
  symlinkSync,
} from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import * as clack from "@clack/prompts";
import { parse as parseYaml } from "yaml";
import { applyFixes } from "./core/fixer";
import { resolveNativePackageBinary } from "./core/native-binary";
import { type KlintDebugEvent, runKlint } from "./core/runner";
import type { ArchConfig, KlintConfig, KlintRule, RuleConfigValue } from "./core/types";
import { BUILT_IN_PLUGINS } from "./plugins/index";
import { BUILT_IN_RULES } from "./rules/index";

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
    const text = readFileSync(configPath, "utf-8");
    raw = (usingYaml ? parseYaml(text) : JSON.parse(text)) as RawConfig;
  } catch {
    process.stderr.write(`klint: failed to parse ${configPath}\n`);
    process.exit(1);
  }
  const root = resolve(configDir, raw.root ?? ".");

  if (engine === "rust") {
    runRustEngine({ configDir, fix, json, raw, rulesFile });
    return;
  }
  if (engine !== undefined && engine !== "ts" && engine !== "compare") {
    process.stderr.write(
      `klint: unknown engine "${engine}" (expected "ts", "rust", or "compare")\n`
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
    runCompareEngine({ configDir, fix, json, raw, rulesFile, tsViolations: violations });
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

  const errors = violations.filter((v) => v.severity === "error");
  const warns = violations.filter((v) => v.severity === "warn");

  if (errors.length === 0 && warns.length === 0) {
    process.stdout.write(`klint: 0 violations in ${formatDuration(startedAt)}\n`);
    process.exit(0);
  }

  const formatBlock = (v: (typeof violations)[number]) => {
    const prefix = v.severity === "warn" ? "⚠" : "×";
    const header = `${v.file}:${v.line}  [${v.rule}]`;
    const sep = "━".repeat(Math.max(0, 80 - header.length));
    return `${header} ${sep}\n\n  ${prefix} ${v.message}\n`;
  };

  if (warns.length > 0) {
    process.stderr.write(
      `klint: ${warns.length} warning(s)\n\n${warns.map(formatBlock).join("\n")}`
    );
  }
  if (errors.length > 0) {
    process.stderr.write(
      `klint: ${errors.length} error(s)\n\n${errors.map(formatBlock).join("\n")}`
    );
    process.stderr.write(`\nklint: finished in ${formatDuration(startedAt)}\n`);
    process.exit(2);
  }
  process.stderr.write(`\nklint: finished in ${formatDuration(startedAt)}\n`);
  process.exit(0);
}

function formatDuration(startedAt: number): string {
  const elapsedMs = performance.now() - startedAt;
  if (elapsedMs < 1000) return `${Math.round(elapsedMs)}ms`;
  return `${(elapsedMs / 1000).toFixed(elapsedMs < 10_000 ? 2 : 1)}s`;
}

function runRustEngine({
  configDir,
  fix,
  json,
  raw,
  rulesFile,
}: {
  configDir: string;
  fix: boolean;
  json: boolean;
  raw: {
    plugins?: string[];
    rules?: Record<string, RuleConfigValue>;
    arch?: unknown;
  };
  rulesFile?: string;
}): void {
  const unsupportedReason = rustEngineUnsupportedReason({
    fix,
    json,
    raw,
    rulesFile,
  });

  if (unsupportedReason) {
    process.stderr.write(`klint: ${unsupportedReason}\n`);
    process.exit(1);
  }

  const command = resolveRustEngineCommand(configDir);
  if (process.argv.includes("--debug") || process.argv.includes("-debug")) {
    process.stderr.write(
      `[klint:debug] rust engine: ${command.bin} ${command.args.join(" ")}\n`
    );
  }
  const result = spawnSync(command.bin, command.args, {
    cwd: import.meta.dir,
    encoding: "utf-8",
  });

  if (result.stdout) process.stdout.write(result.stdout);
  if (result.stderr) process.stderr.write(result.stderr);
  process.exit(result.status ?? 1);
}

function runCompareEngine({
  configDir,
  fix,
  json,
  raw,
  rulesFile,
  tsViolations,
}: {
  configDir: string;
  fix: boolean;
  json: boolean;
  raw: {
    plugins?: string[];
    rules?: Record<string, RuleConfigValue>;
    arch?: unknown;
  };
  rulesFile?: string;
  tsViolations: ReturnType<typeof runKlint>;
}): void {
  const unsupportedReason = rustEngineUnsupportedReason({
    fix,
    json,
    raw,
    rulesFile,
  });

  if (unsupportedReason) {
    process.stderr.write(`klint: ${unsupportedReason}\n`);
    process.exit(1);
  }

  const tsResult = toJsonCliResult(tsViolations);
  const rustCommand = resolveRustEngineCommand(configDir);
  const rustResult = spawnSync(rustCommand.bin, rustCommand.args, {
    cwd: import.meta.dir,
    encoding: "utf-8",
  });

  const rustStatus = rustResult.status ?? 1;
  if (rustResult.stderr) process.stderr.write(rustResult.stderr);

  const tsPayload = parseJsonPayload(tsResult.stdout);
  const rustPayload = parseJsonPayload(rustResult.stdout);

  if (
    rustStatus !== tsResult.status ||
    tsPayload === undefined ||
    rustPayload === undefined ||
    !jsonPayloadEquals(tsPayload, rustPayload)
  ) {
    const mismatchLines = [
      "klint: compare engine mismatch",
      `TypeScript exit: ${tsResult.status}`,
      `Rust exit: ${rustStatus}`,
    ];
    process.stderr.write(`${mismatchLines.join("\n")}\n`);
    process.exit(1);
  }

  process.stdout.write(tsResult.stdout);
  process.exit(tsResult.status);
}

function toJsonCliResult(violations: ReturnType<typeof runKlint>): {
  stdout: string;
  status: number;
} {
  const errors = violations.filter((v) => v.severity === "error");
  return {
    stdout: JSON.stringify({
      violations: violations.map((v) => ({ ...v, fix: v.fix ?? null })),
      summary: { errors: errors.length, warnings: violations.length - errors.length },
    }),
    status: errors.length > 0 ? 2 : 0,
  };
}

function writeDebugEvent(event: KlintDebugEvent): void {
  switch (event.type) {
    case "walk:start":
      process.stderr.write(`[klint:debug] walk ${event.dir}\n`);
      break;
    case "walk:done":
      process.stderr.write(`[klint:debug] walked ${event.dir} (${event.files} files)\n`);
      break;
    case "files:resolved":
      process.stderr.write(`[klint:debug] resolved ${event.files} files\n`);
      break;
    case "rule:start":
      process.stderr.write(
        `[klint:debug] rule ${event.rule} start (${event.files} files)\n`
      );
      break;
    case "rule:done":
      process.stderr.write(
        `[klint:debug] rule ${event.rule} done (${event.violations} violations)\n`
      );
      break;
    case "arch:start":
      process.stderr.write(`[klint:debug] arch start (${event.files} files)\n`);
      break;
    case "arch:done":
      process.stderr.write(`[klint:debug] arch done (${event.violations} violations)\n`);
      break;
  }
}

interface RustEngineCommand {
  bin: string;
  args: string[];
}

function resolveRustEngineCommand(configDir: string): RustEngineCommand {
  const args = ["--config", configDir, "--json"];
  const explicitBin = process.env.KLINT_RUST_BIN;
  if (explicitBin) {
    return { bin: explicitBin, args };
  }

  const nativePackageBin = resolveNativePackageBinary({ packageRoot: import.meta.dir });
  if (nativePackageBin) {
    return { bin: nativePackageBin, args };
  }

  const localBin = join(
    import.meta.dir,
    "target",
    "debug",
    process.platform === "win32" ? "klint-rs.exe" : "klint-rs"
  );
  if (existsSync(localBin)) {
    return { bin: localBin, args };
  }

  return {
    bin: "cargo",
    args: ["run", "--quiet", "-p", "klint-rs", "--", ...args],
  };
}

function rustEngineUnsupportedReason({
  fix,
  json,
  raw,
  rulesFile,
}: {
  fix: boolean;
  json: boolean;
  raw: {
    plugins?: string[];
    rules?: Record<string, RuleConfigValue>;
    arch?: unknown;
  };
  rulesFile?: string;
}): string | undefined {
  if (!json) return "KLINT_ENGINE=rust currently requires --json";
  if (fix) return "KLINT_ENGINE=rust does not support --fix";
  if (rulesFile) return "KLINT_ENGINE=rust does not support --rules";
  if (!raw.arch) return "KLINT_ENGINE=rust requires an arch config";
  if ((raw.plugins?.length ?? 0) > 0) return "KLINT_ENGINE=rust does not support plugins";
  const unsupportedRules = Object.entries(raw.rules ?? {})
    .filter(([, value]) => ruleConfigSeverity(value) !== "off")
    .map(([name]) => name);
  if (unsupportedRules.length > 0) {
    return [
      "Rust engine currently supports arch rules only",
      "",
      "Unsupported TypeScript rules:",
      ...unsupportedRules.map((rule) => `- ${rule}`),
    ].join("\n");
  }
  return undefined;
}

function ruleConfigSeverity(value: RuleConfigValue): string | undefined {
  if (typeof value === "string") return value;
  return value.severity;
}

function parseJsonPayload(value: string): unknown {
  try {
    return JSON.parse(value);
  } catch {
    return undefined;
  }
}

function jsonPayloadEquals(a: unknown, b: unknown): boolean {
  return JSON.stringify(canonicalJson(a)) === JSON.stringify(canonicalJson(b));
}

function canonicalJson(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(canonicalJson);
  if (!value || typeof value !== "object") return value;
  return Object.fromEntries(
    Object.entries(value)
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([key, entry]) => [key, canonicalJson(entry)])
  );
}

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

async function installSkill(args: string[]): Promise<void> {
  const skillSrc = join(import.meta.dir, "skill", "klint-rules");
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
    mkdirSync(dirname(dest), { recursive: true });
    try {
      rmSync(dest, { recursive: true, force: true });
    } catch {
      /* already gone */
    }
    if (useSymlink) {
      symlinkSync(relative(dirname(dest), skillSrc), dest, linkType);
    } else {
      cpSync(skillSrc, dest, { recursive: true });
    }
  }

  if (process.stdin.isTTY) {
    clack.outro("Done.");
  }
}

function printHelp(): void {
  const pluginRules = new Set(
    Object.values(BUILT_IN_PLUGINS).flatMap((p) => Object.keys(p.rules))
  );
  const standaloneRules = Object.keys(BUILT_IN_RULES).filter((r) => !pluginRules.has(r));
  const pluginEntries = Object.entries(BUILT_IN_PLUGINS);

  process.stdout.write(
    [
      "klint — agent harness for TypeScript architecture rules",
      "",
      "Usage: klint [--config <dir>] [--rules <file>] [--engine ts|rust|compare] [--fix] [--json]",
      "       klint install-skill [--agents <list>] [--symlink | --copy]",
      "",
      "  --config <dir>   directory containing klint.yaml or klint.config.json (default: cwd)",
      "  --rules  <file>  custom rules file (default: <configDir>/klint.rules.ts if present)",
      "  --engine <name>  engine to use: ts (default), rust, or compare (experimental, requires --json)",
      "  --fix            apply auto-fixes for fixable violations in-place",
      "  --json           emit structured JSON to stdout (for agent/CI consumption)",
      "  --debug          print file resolution and rule progress to stderr",
      "",
      "  install-skill    install the rule-authoring skill into agent config directories",
      "                   --agents <list>  comma-separated: claude,opencode,cursor,codex (default: all)",
      "                   --symlink        install as symlink (stays in sync with updates)",
      "                   --copy           install as copy (default in non-TTY)",
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

if (import.meta.main) await main();
