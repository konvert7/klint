export type Severity = "error" | "warning";

export interface FileDiagnostic {
  line: number;
  message: string;
  rule: string;
  severity: Severity;
  docsUrl: string;
}

interface KlintViolation {
  file: string;
  line: number;
  rule: string;
  message: string;
  severity: string;
}

export function ruleDocsUrl(rule: string): string {
  return `https://klint.dev/rules/${rule}`;
}

function isViolation(value: unknown): value is KlintViolation {
  if (typeof value !== "object" || value === null) return false;

  const v = value as Record<string, unknown>;

  return (
    typeof v.file === "string" &&
    typeof v.line === "number" &&
    typeof v.rule === "string" &&
    typeof v.message === "string" &&
    typeof v.severity === "string"
  );
}

function violationsIn(stdout: string): KlintViolation[] {
  let parsed: unknown;

  try {
    parsed = JSON.parse(stdout);
  } catch {
    throw new Error(`klint printed output that is not JSON: ${stdout.slice(0, 200)}`);
  }

  const violations = (parsed as { violations?: unknown }).violations;
  if (!Array.isArray(violations)) {
    throw new Error("klint JSON output has no violations array");
  }

  return violations.filter(isViolation);
}

function toDiagnostic(violation: KlintViolation): FileDiagnostic {
  return {
    line: Math.max(violation.line - 1, 0),
    message: violation.message,
    rule: violation.rule,
    severity: violation.severity === "warn" ? "warning" : "error",
    docsUrl: ruleDocsUrl(violation.rule),
  };
}

export function diagnosticsByFile(stdout: string): Map<string, FileDiagnostic[]> {
  const byFile = new Map<string, FileDiagnostic[]>();

  for (const violation of violationsIn(stdout)) {
    const list = byFile.get(violation.file) ?? [];
    list.push(toDiagnostic(violation));
    byFile.set(violation.file, list);
  }

  return byFile;
}
