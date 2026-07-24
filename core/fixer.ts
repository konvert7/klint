import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import type { Violation } from "./types";

/**
 * Applies all fixable violations to their source files.
 * Fixes are applied bottom-to-top within each file to preserve line offsets.
 * Returns the number of fixes applied.
 */
export function applyFixes(violations: Violation[], root: string): number {
  const byFile = new Map<string, Violation[]>();
  for (const v of violations) {
    if (!v.fix) continue;
    const abs = resolve(root, v.file);
    const existing = byFile.get(abs) ?? [];
    existing.push(v);
    byFile.set(abs, existing);
  }

  let applied = 0;
  for (const [absPath, fileViolations] of byFile) {
    const lines = readFileSync(absPath, "utf-8").split("\n");
    const sorted = [...fileViolations].sort(byLargestFixRangeFirst);

    const used: Array<{ start: number; end: number }> = [];
    for (const v of sorted) {
      if (!v.fix) continue;
      const { startLine, endLine, replacement } = v.fix;
      if (used.some((r) => startLine <= r.end && endLine >= r.start)) continue;
      lines.splice(startLine - 1, endLine - startLine + 1, ...replacement.split("\n"));
      used.push({ start: startLine, end: endLine });
      applied++;
    }

    writeFileSync(absPath, lines.join("\n"), "utf-8");
  }

  return applied;
}

/**
 * Largest-range fix wins: sorting by `endLine` descending applies fixes bottom-to-top
 * and lets an outer or overlapping fix beat inner ones that share a start position.
 */
function byLargestFixRangeFirst(a: Violation, b: Violation): number {
  return (b.fix?.endLine ?? 0) - (a.fix?.endLine ?? 0);
}
