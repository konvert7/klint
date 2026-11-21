import { join } from "node:path";

export interface BinaryLookup {
  workspaceRoot: string;
  configuredPath: string | undefined;
  platform: NodeJS.Platform;
  exists: (path: string) => boolean;
}

function localCandidates(root: string, platform: NodeJS.Platform): string[] {
  if (platform === "win32") {
    return [
      join(root, "node_modules", ".bin", "klint.cmd"),
      join(root, ".venv", "Scripts", "klint.exe"),
    ];
  }

  return [
    join(root, "node_modules", ".bin", "klint"),
    join(root, ".venv", "bin", "klint"),
  ];
}

export function resolveKlintBinary(lookup: BinaryLookup): string {
  const configured = lookup.configuredPath?.trim();
  if (configured) return configured;

  const local = localCandidates(lookup.workspaceRoot, lookup.platform).find(
    lookup.exists
  );

  return local ?? "klint";
}
