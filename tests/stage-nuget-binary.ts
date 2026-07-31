import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import { isAbsolute, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const args = process.argv.slice(2);
const rid = valueAfter(args, "--rid");
const sourceArg = valueAfter(args, "--source");

const BINARY_BY_RID: Record<string, string> = {
  "linux-x64": "klint-rs",
  "osx-x64": "klint-rs",
  "osx-arm64": "klint-rs",
  "win-x64": "klint-rs.exe",
};

if (!rid || !(rid in BINARY_BY_RID)) {
  fail(
    `Usage: bun tests/stage-nuget-binary.ts --rid <${Object.keys(BINARY_BY_RID).join("|")}> --source <path>`
  );
}

const binaryName = BINARY_BY_RID[rid];
const source = resolveSource(sourceArg);
const destinationDir = join(root, "nuget", "binaries", rid);
const destination = join(destinationDir, binaryName);

if (!existsSync(source)) {
  fail(`Missing release binary: ${source}`);
}

mkdirSync(destinationDir, { recursive: true });
copyFileSync(source, destination);

process.stdout.write(`staged ${rid} binary at ${destination}\n`);

function resolveSource(sourceArg: string | undefined): string {
  if (sourceArg) return isAbsolute(sourceArg) ? sourceArg : resolve(root, sourceArg);
  return join(root, "target", "release", binaryName);
}

function valueAfter(args: string[], name: string): string | undefined {
  const index = args.indexOf(name);
  return index >= 0 ? args[index + 1] : undefined;
}

function fail(message: string): never {
  process.stderr.write(`${message}\n`);
  process.exit(1);
}
