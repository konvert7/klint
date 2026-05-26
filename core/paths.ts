import { relative } from "node:path";

export function toSlashPath(path: string): string {
  return path.replaceAll("\\", "/");
}

export function relativeSlashPath(from: string, to: string): string {
  return toSlashPath(relative(from, to));
}
