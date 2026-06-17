import { type SpawnSyncReturns, spawnSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { runKlint } from "../core/runner";
import type { KlintRule, RuleConfigValue } from "../core/types";
import {
  emptyJsonPayload,
  isJsonPayload,
  type JsonPayload,
  jsonPayloadEquals,
  mergeJsonOutputs,
  parseJsonPayload,
  toJsonCliResult,
  toJsonPayload,
  writeTextOutput,
} from "./output";
import { PACKAGE_ROOT } from "./paths";
import {
  resolveEffectiveRules,
  resolveRustEngineCommand,
  ruleConfigSeverity,
  rustEngineUnsupportedReason,
  splitRulesForAuto,
} from "./rust-config";

export function runRustEngine({
  fix,
  json,
  raw,
  root,
  rulesFile,
  startedAt,
}: {
  fix: boolean;
  json: boolean;
  raw: {
    plugins?: string[];
    rules?: Record<string, RuleConfigValue>;
    arch?: unknown;
  };
  root: string;
  rulesFile?: string;
  startedAt: number;
}): void {
  const unsupportedReason = rustEngineUnsupportedReason({
    fix,
    raw,
    rulesFile,
  });

  if (unsupportedReason) {
    process.stderr.write(`klint: ${unsupportedReason}\n`);
    process.exit(1);
  }

  const { configDir: rustConfigDir, cleanup } = writeRustEngineConfig({ raw, root });
  const command = resolveRustEngineCommand(rustConfigDir);
  let result: SpawnSyncReturns<string>;
  try {
    if (process.argv.includes("--debug") || process.argv.includes("-debug")) {
      process.stderr.write(
        `[klint:debug] rust engine: ${command.bin} ${command.args.join(" ")}\n`
      );
    }
    result = spawnSync(command.bin, command.args, {
      cwd: PACKAGE_ROOT,
      encoding: "utf-8",
    });
  } finally {
    cleanup();
  }

  if (result.stderr) process.stderr.write(result.stderr);
  if (json) {
    if (result.stdout) process.stdout.write(result.stdout);
    process.exit(result.status ?? 1);
  }

  const payload = parseJsonPayload(result.stdout);
  if (!isJsonPayload(payload)) {
    process.stderr.write("klint: --engine rust failed to parse Rust JSON output\n");
    process.exit(1);
  }
  writeTextOutput(payload.violations, startedAt);
}

export function runCompareEngine({
  fix,
  json,
  raw,
  root,
  rulesFile,
  tsViolations,
}: {
  fix: boolean;
  json: boolean;
  raw: {
    plugins?: string[];
    rules?: Record<string, RuleConfigValue>;
    arch?: unknown;
  };
  root: string;
  rulesFile?: string;
  tsViolations: ReturnType<typeof runKlint>;
}): void {
  if (!json) {
    process.stderr.write("klint: --engine compare currently requires --json\n");
    process.exit(1);
  }

  const unsupportedReason = rustEngineUnsupportedReason({
    fix,
    raw,
    rulesFile,
  });

  if (unsupportedReason) {
    process.stderr.write(`klint: ${unsupportedReason}\n`);
    process.exit(1);
  }

  const tsResult = toJsonCliResult(tsViolations);
  const { configDir: rustConfigDir, cleanup } = writeRustEngineConfig({ raw, root });
  const rustCommand = resolveRustEngineCommand(rustConfigDir);
  let rustResult: SpawnSyncReturns<string>;
  try {
    rustResult = spawnSync(rustCommand.bin, rustCommand.args, {
      cwd: PACKAGE_ROOT,
      encoding: "utf-8",
    });
  } finally {
    cleanup();
  }

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

function writeRustEngineConfig({
  raw,
  root,
}: {
  raw: {
    include?: string[];
    plugins?: string[];
    rules?: Record<string, RuleConfigValue>;
    arch?: unknown;
  };
  root: string;
}): { configDir: string; cleanup: () => void } {
  const tempDir = mkdtempSync(join(tmpdir(), "klint-rust-engine-"));
  writeFileSync(
    join(tempDir, "klint.config.json"),
    JSON.stringify({
      root,
      include: raw.include ?? ["."],
      plugins: [],
      rules: resolveEffectiveRules(raw.plugins, raw.rules ?? {}),
      arch: raw.arch,
    })
  );
  return {
    configDir: tempDir,
    cleanup: () => rmSync(tempDir, { recursive: true, force: true }),
  };
}

export function runAutoEngine({
  fix,
  json,
  raw,
  root,
  rulesFile,
  startedAt,
  customRules,
  customRulesMap,
}: {
  fix: boolean;
  json: boolean;
  raw: {
    root?: string;
    include?: string[];
    plugins?: string[];
    rules?: Record<string, RuleConfigValue>;
    arch?: unknown;
  };
  root: string;
  rulesFile?: string;
  startedAt: number;
  customRules: Record<string, KlintRule>;
  customRulesMap: Record<string, RuleConfigValue>;
}): void {
  if (fix) {
    process.stderr.write("klint: --engine auto does not support --fix\n");
    process.exit(1);
  }
  if (rulesFile) {
    process.stderr.write("klint: --engine auto does not support --rules\n");
    process.exit(1);
  }

  const effectiveRules = resolveEffectiveRules(raw.plugins, raw.rules ?? {});
  const { rustRules, tsRules } = splitRulesForAuto(effectiveRules);
  const tsViolations = runKlint(
    {
      root,
      include: raw.include ?? ["."],
      plugins: [],
      rules: { ...customRulesMap, ...tsRules },
    },
    customRules
  );

  const rustOutput = runAutoRustSubset({
    raw,
    root,
    rustRules,
  });
  const tsOutput = toJsonPayload(tsViolations);
  const merged = mergeJsonOutputs(rustOutput, tsOutput);
  if (json) {
    process.stdout.write(JSON.stringify(merged));
    process.exit(merged.summary.errors > 0 ? 2 : 0);
  }
  writeTextOutput(merged.violations, startedAt);
}

function runAutoRustSubset({
  raw,
  root,
  rustRules,
}: {
  raw: {
    include?: string[];
    arch?: unknown;
  };
  root: string;
  rustRules: Record<string, RuleConfigValue>;
}): JsonPayload {
  const hasRustRules = Object.values(rustRules).some(
    (value) => ruleConfigSeverity(value) !== "off"
  );
  if (!raw.arch && !hasRustRules) {
    return emptyJsonPayload();
  }

  const tempDir = mkdtempSync(join(tmpdir(), "klint-auto-rust-"));
  try {
    writeFileSync(
      join(tempDir, "klint.config.json"),
      JSON.stringify({
        root,
        include: raw.include ?? ["."],
        rules: rustRules,
        arch: raw.arch,
      })
    );

    const command = resolveRustEngineCommand(tempDir);
    const result = spawnSync(command.bin, command.args, {
      cwd: PACKAGE_ROOT,
      encoding: "utf-8",
    });
    if (result.stderr) process.stderr.write(result.stderr);
    if ((result.status ?? 1) !== 0 && (result.status ?? 1) !== 2) {
      process.stderr.write(result.stdout);
      process.exit(result.status ?? 1);
    }

    const payload = parseJsonPayload(result.stdout);
    if (!isJsonPayload(payload)) {
      process.stderr.write("klint: --engine auto failed to parse Rust JSON output\n");
      process.exit(1);
    }
    return payload;
  } finally {
    rmSync(tempDir, { recursive: true, force: true });
  }
}
