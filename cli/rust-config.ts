import { existsSync } from "node:fs";
import { join } from "node:path";
import { resolveNativePackageBinary } from "../core/native-binary";
import type { RuleConfigValue } from "../core/types";
import { BUILT_IN_PLUGINS } from "../plugins/index";
import { PACKAGE_ROOT } from "./paths";

const RUST_SUPPORTED_RULES = new Set([
  "no-consecutive-array-push",
  "no-nested-template-literals",
  "no-string-match",
  "no-sync-in-async",
  "no-unguarded-json-parse",
  "sonar/no-single-char-class",
  "sonar/prefer-at",
  "sonar/prefer-nullish-coalescing-assign",
  "sonar/prefer-string-replaceall",
  "sonar/prefer-string-raw",
  "sonar/prefer-string-raw-regexp",
]);

export function splitRulesForAuto(rules: Record<string, RuleConfigValue>): {
  rustRules: Record<string, RuleConfigValue>;
  tsRules: Record<string, RuleConfigValue>;
} {
  const rustRules: Record<string, RuleConfigValue> = {};
  const tsRules: Record<string, RuleConfigValue> = {};

  for (const [name, value] of Object.entries(rules)) {
    if (RUST_SUPPORTED_RULES.has(name)) rustRules[name] = value;
    else tsRules[name] = value;
  }

  return { rustRules, tsRules };
}

export interface RustEngineCommand {
  bin: string;
  args: string[];
}

export function resolveRustEngineCommand(configDir: string): RustEngineCommand {
  const args = ["--config", configDir, "--json"];
  const explicitBin = process.env.KLINT_RUST_BIN;
  if (explicitBin) {
    return { bin: explicitBin, args };
  }

  const localBin = join(
    PACKAGE_ROOT,
    "target",
    "debug",
    process.platform === "win32" ? "klint-rs.exe" : "klint-rs"
  );
  if (isSourceCheckout() && existsSync(localBin)) {
    return { bin: localBin, args };
  }

  const nativePackageBin = resolveNativePackageBinary({ packageRoot: PACKAGE_ROOT });
  if (nativePackageBin) {
    return { bin: nativePackageBin, args };
  }

  if (!isSourceCheckout()) {
    process.stderr.write(
      "klint: native Rust engine binary is unavailable for this package version\n"
    );
    process.exit(1);
  }

  return {
    bin: "cargo",
    args: ["run", "--quiet", "-p", "klint-rs", "--", ...args],
  };
}

function isSourceCheckout(): boolean {
  return existsSync(join(PACKAGE_ROOT, "crates", "klint-rs", "Cargo.toml"));
}

export function rustEngineUnsupportedReason({
  fix,
  raw,
  rulesFile,
}: {
  fix: boolean;
  raw: {
    plugins?: string[];
    rules?: Record<string, RuleConfigValue>;
    arch?: unknown;
  };
  rulesFile?: string;
}): string | undefined {
  if (fix) return "KLINT_ENGINE=rust does not support --fix";
  if (rulesFile) return "KLINT_ENGINE=rust does not support --rules";
  let effectiveRules: Record<string, RuleConfigValue>;
  try {
    effectiveRules = resolveEffectiveRules(raw.plugins, raw.rules ?? {});
  } catch (error) {
    return error instanceof Error
      ? error.message
      : "KLINT_ENGINE=rust failed to load plugins";
  }
  const activeSupportedRules = Object.entries(effectiveRules).filter(
    ([name, value]) =>
      ruleConfigSeverity(value) !== "off" && RUST_SUPPORTED_RULES.has(name)
  );
  const unsupportedRules = Object.entries(effectiveRules)
    .filter(
      ([name, value]) =>
        ruleConfigSeverity(value) !== "off" && !RUST_SUPPORTED_RULES.has(name)
    )
    .map(([name]) => name);
  if (!raw.arch && activeSupportedRules.length === 0 && unsupportedRules.length === 0) {
    return "KLINT_ENGINE=rust requires an arch config or supported rule";
  }
  if (unsupportedRules.length > 0) {
    return [
      "Rust engine currently supports arch rules and selected rules only",
      "",
      "Unsupported rules:",
      ...unsupportedRules.map((rule) => `- ${rule}`),
    ].join("\n");
  }
  return undefined;
}

export function resolveEffectiveRules(
  plugins: string[] | undefined,
  rules: Record<string, RuleConfigValue>
): Record<string, RuleConfigValue> {
  const pluginDefaults: Record<string, RuleConfigValue> = {};
  for (const pluginName of plugins ?? []) {
    const plugin = BUILT_IN_PLUGINS[pluginName];
    if (!plugin) throw new Error(`Unknown klint plugin: "${pluginName}"`);
    Object.assign(pluginDefaults, plugin.rules);
  }
  return { ...pluginDefaults, ...rules };
}

export function ruleConfigSeverity(value: RuleConfigValue): string | undefined {
  if (typeof value === "string") return value;
  return value.severity;
}
