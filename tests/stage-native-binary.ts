import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import { isAbsolute, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  nativePackageForPlatform,
  nativePackages,
  resolveNativePackageBinary,
} from "../core/native-binary";

const root = fileURLToPath(new URL("..", import.meta.url));
const args = process.argv.slice(2);
const packageKey = valueAfter(args, "--package");
const sourceArg = valueAfter(args, "--source");
const nativePackage = packageKey
  ? nativePackages().find((pkg) => `${pkg.platform}-${pkg.arch}` === packageKey)
  : nativePackageForPlatform();

if (!nativePackage) {
  fail(
    packageKey
      ? `No native package mapping for ${packageKey}`
      : `No native package mapping for ${process.platform}-${process.arch}`
  );
}

const source = resolveSource(sourceArg);
const packageRoot = join(
  root,
  "npm",
  "native",
  `${nativePackage.platform}-${nativePackage.arch}`
);
const destination = join(packageRoot, nativePackage.binaryPath);

if (!existsSync(source)) {
  fail(`Missing release binary: ${source}`);
}

mkdirSync(join(packageRoot, "bin"), { recursive: true });
copyFileSync(source, destination);

const resolved = resolveNativePackageBinary({
  packageRoot: root,
  platform: nativePackage.platform,
  arch: nativePackage.arch,
  resolvePackageJson: (packageName) => {
    if (packageName !== nativePackage.packageName) {
      throw new Error(`Unexpected package resolution: ${packageName}`);
    }
    return join(packageRoot, "package.json");
  },
});

if (resolved !== destination) {
  fail(
    `Native binary resolver returned ${resolved ?? "(none)"}, expected ${destination}`
  );
}

process.stdout.write(`staged ${nativePackage.packageName} binary at ${destination}\n`);

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
