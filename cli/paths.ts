import { readFileSync } from "node:fs";
import { resolve } from "node:path";

export const PACKAGE_ROOT = resolve(import.meta.dir, "..");

export const SHIPPED_SKILL_PATH = resolve(
  PACKAGE_ROOT,
  "skill",
  "klint-rules",
  "SKILL.md"
);

export function installedVersion(): string {
  try {
    const manifest = readFileSync(resolve(PACKAGE_ROOT, "package.json"), "utf-8");
    return (JSON.parse(manifest) as { version?: string }).version ?? "0.0.0";
  } catch {
    return "0.0.0";
  }
}
