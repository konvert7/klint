import { readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { fail, replaceRequired, valueAfter } from "./release-cli";

interface Options {
  version: string;
  root: string;
}

const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const options = parseArgs();
const csprojPath = join(options.root, "nuget", "Klint.Tool.csproj");

const csproj = readFileSync(csprojPath, "utf-8");
const nextCsproj = replaceRequired(
  csproj,
  /<Version>.+<\/Version>/,
  `<Version>${options.version}</Version>`,
  csprojPath
);
writeFileSync(csprojPath, nextCsproj);

process.stdout.write(
  `prepared NuGet release metadata: Konvert7.Klint@${options.version}\n`
);

function parseArgs(): Options {
  const args = process.argv.slice(2);
  const version = valueAfter(args, "--version");
  if (!version) {
    fail(
      "Usage: bun tools/prepare-nuget-release.ts --version <version> [--root <repo-root>]"
    );
  }

  return {
    version,
    root: resolve(valueAfter(args, "--root") ?? repoRoot),
  };
}
