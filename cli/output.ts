import type { KlintDebugEvent, runKlint } from "../core/runner";
import type { Violation } from "../core/types";

export function formatDuration(startedAt: number): string {
  const elapsedMs = performance.now() - startedAt;
  if (elapsedMs < 1000) return `${Math.round(elapsedMs)}ms`;
  return `${(elapsedMs / 1000).toFixed(elapsedMs < 10_000 ? 2 : 1)}s`;
}

export function writeTextOutput(
  violations: Array<Omit<Violation, "fix">>,
  startedAt: number
): never {
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

export interface JsonPayload {
  violations: Array<Omit<Violation, "fix"> & { fix: unknown }>;
  summary: {
    errors: number;
    warnings: number;
  };
}

export function emptyJsonPayload(): JsonPayload {
  return {
    violations: [],
    summary: { errors: 0, warnings: 0 },
  };
}

export function toJsonPayload(violations: ReturnType<typeof runKlint>): JsonPayload {
  const errors = violations.filter((v) => v.severity === "error");
  return {
    violations: violations.map((v) => ({ ...v, fix: v.fix ?? null })),
    summary: { errors: errors.length, warnings: violations.length - errors.length },
  };
}

export function mergeJsonOutputs(...outputs: JsonPayload[]): JsonPayload {
  const violations = outputs
    .flatMap((output) => output.violations)
    .sort(
      (a, b) =>
        a.file.localeCompare(b.file) ||
        a.line - b.line ||
        a.rule.localeCompare(b.rule) ||
        a.message.localeCompare(b.message)
    );
  const errors = violations.filter((violation) => violation.severity === "error").length;
  return {
    violations,
    summary: {
      errors,
      warnings: violations.length - errors,
    },
  };
}

export function isJsonPayload(value: unknown): value is JsonPayload {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Partial<JsonPayload>;
  return Array.isArray(candidate.violations) && typeof candidate.summary === "object";
}

export function toJsonCliResult(violations: ReturnType<typeof runKlint>): {
  stdout: string;
  status: number;
} {
  const payload = toJsonPayload(violations);
  return {
    stdout: JSON.stringify(payload),
    status: payload.summary.errors > 0 ? 2 : 0,
  };
}

export function writeDebugEvent(event: KlintDebugEvent): void {
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

export function parseJsonPayload(value: string): unknown {
  try {
    return JSON.parse(value);
  } catch {
    return undefined;
  }
}

export function jsonPayloadEquals(a: unknown, b: unknown): boolean {
  return JSON.stringify(canonicalJson(a)) === JSON.stringify(canonicalJson(b));
}

function canonicalJson(value: unknown): unknown {
  if (Array.isArray(value)) {
    return value
      .map(canonicalJson)
      .sort((left, right) => JSON.stringify(left).localeCompare(JSON.stringify(right)));
  }
  if (!value || typeof value !== "object") return value;
  return Object.fromEntries(
    Object.entries(value)
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([key, entry]) => [key, canonicalJson(entry)])
  );
}
