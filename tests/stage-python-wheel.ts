import { copyFileSync, existsSync, mkdirSync, rmSync } from "node:fs";
import { isAbsolute, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const args = process.argv.slice(2);
const sourceArg = valueAfter(args, "--source");
const source = resolveSource(sourceArg);
const binDir = join(root, "python", "klint", "_bin");
const destination = join(
  binDir,
  process.platform === "win32" ? "klint-rs.exe" : "klint-rs"
);

if (!existsSync(source)) {
  fail(`Missing release binary: ${source}`);
}

rmSync(binDir, { recursive: true, force: true });
mkdirSync(binDir, { recursive: true });
copyFileSync(source, destination);

process.stdout.write(`staged Python wheel binary at ${destination}\n`);

function fail(message: string): never {
  process.stderr.write(`${message}\n`);
  process.exit(1);
}

function valueAfter(args: string[], name: string): string | undefined {
  const index = args.indexOf(name);
  return index >= 0 ? args[index + 1] : undefined;
}

function resolveSource(sourceArg: string | undefined): string {
  if (sourceArg) return isAbsolute(sourceArg) ? sourceArg : resolve(root, sourceArg);

  return join(
    root,
    "target",
    "release",
    process.platform === "win32" ? "klint-rs.exe" : "klint-rs"
  );
}
