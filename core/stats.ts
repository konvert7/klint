import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import type { Violation } from "./types";

export const STATS_DIR = ".klint";
export const STATS_FILE = "stats.json";
const SELF_IGNORE = "# Created by klint automatically.\n*\n";
const STATS_VERSION = 1;

export type RunSource = "hook" | "direct";

type CountedViolation = Pick<Violation, "file" | "rule" | "message" | "severity">;

export type RunRecorder = (violations: readonly CountedViolation[]) => void;

interface OpenError {
  rule: string;
  file: string;
  message: string;
  caughtBy: RunSource;
}

export interface KlintStats {
  version: typeof STATS_VERSION;
  runs: Record<RunSource, number>;
  prevented: {
    byRule: Record<string, number>;
    bySource: Record<RunSource, number>;
  };
  open: OpenError[];
}

export function emptyStats(): KlintStats {
  return {
    version: STATS_VERSION,
    runs: { hook: 0, direct: 0 },
    prevented: { byRule: {}, bySource: { hook: 0, direct: 0 } },
    open: [],
  };
}

function sameError(a: Omit<OpenError, "caughtBy">, b: Omit<OpenError, "caughtBy">) {
  return a.rule === b.rule && a.file === b.file && a.message === b.message;
}

function errorsOf(
  violations: readonly CountedViolation[]
): Omit<OpenError, "caughtBy">[] {
  return violations
    .filter((v) => v.severity === "error")
    .map(({ rule, file, message }) => ({ rule, file, message }));
}

export function nextStats(
  previous: KlintStats,
  violations: readonly CountedViolation[],
  source: RunSource
): KlintStats {
  const stats = structuredClone(previous);
  stats.runs[source] += 1;

  const unmatched = errorsOf(violations);
  const stillOpen: OpenError[] = [];
  for (const error of previous.open) {
    const match = unmatched.findIndex((current) => sameError(current, error));
    if (match === -1) {
      stats.prevented.byRule[error.rule] = (stats.prevented.byRule[error.rule] ?? 0) + 1;
      stats.prevented.bySource[error.caughtBy] += 1;
      continue;
    }
    unmatched.splice(match, 1);
    stillOpen.push(error);
  }
  stats.open = [
    ...stillOpen,
    ...unmatched.map((error) => ({ ...error, caughtBy: source })),
  ];
  return stats;
}

function isStats(value: unknown): value is KlintStats {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Partial<KlintStats>;
  return (
    candidate.version === STATS_VERSION &&
    typeof candidate.runs?.hook === "number" &&
    typeof candidate.runs.direct === "number" &&
    typeof candidate.prevented?.byRule === "object" &&
    typeof candidate.prevented.bySource?.hook === "number" &&
    typeof candidate.prevented.bySource.direct === "number" &&
    Array.isArray(candidate.open)
  );
}

export function readStats(configDir: string): KlintStats {
  try {
    const parsed: unknown = JSON.parse(
      readFileSync(join(configDir, STATS_DIR, STATS_FILE), "utf-8")
    );
    return isStats(parsed) ? parsed : emptyStats();
  } catch {
    return emptyStats();
  }
}

function writeStats(configDir: string, stats: KlintStats): void {
  const dir = join(configDir, STATS_DIR);
  mkdirSync(dir, { recursive: true });
  const ignoreFile = join(dir, ".gitignore");
  if (!existsSync(ignoreFile)) writeFileSync(ignoreFile, SELF_IGNORE);
  const target = join(dir, STATS_FILE);
  const staging = `${target}.${process.pid}.tmp`;
  writeFileSync(staging, `${JSON.stringify(stats, null, 2)}\n`);
  renameSync(staging, target);
}

function statsDisabled(): boolean {
  return Boolean(process.env.KLINT_NO_STATS);
}

export function runRecorder(configDir: string, source: RunSource): RunRecorder {
  return (violations) => {
    if (statsDisabled()) return;
    try {
      writeStats(configDir, nextStats(readStats(configDir), violations, source));
    } catch {}
  };
}
