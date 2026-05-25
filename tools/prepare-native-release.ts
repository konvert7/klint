import {
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { nativePackages } from "../core/native-binary";

interface PackageJson {
  name: string;
  version: string;
  private?: boolean;
  optionalDependencies?: Record<string, string>;
}

interface Options {
  version: string;
  outDir: string;
  requireBinaries: boolean;
  keep: boolean;
}

const root = fileURLToPath(new URL("..", import.meta.url));
const options = parseArgs();

if (existsSync(options.outDir)) {
  rmSync(options.outDir, { recursive: true, force: true });
}
mkdirSync(options.outDir, { recursive: true });

try {
  const releaseRootPackageJson = prepareRootPackageJson();
  const nativeResults = nativePackages().map(prepareNativePackage);

  validateRootPackageJson(releaseRootPackageJson);
  for (const result of nativeResults) validateNativePackage(result.packageJson);

  const missing = nativeResults
    .filter((result) => !result.binaryCopied)
    .map((result) => result.packageJson.name);

  if (options.requireBinaries && missing.length > 0) {
    fail(`Missing native release binaries:\n${missing.join("\n")}`);
  }

  const missingMessage =
    missing.length > 0
      ? `missing binaries: ${missing.join(", ")}`
      : "missing binaries: none";
  const outputLines = [
    `native release dry-run ok (${options.version})`,
    `output: ${options.outDir}`,
    `native packages: ${nativeResults.length}`,
    missingMessage,
  ];
  process.stdout.write(`${outputLines.join("\n")}\n`);
} finally {
  if (!options.keep) {
    rmSync(options.outDir, { recursive: true, force: true });
  }
}

function prepareRootPackageJson(): PackageJson {
  const packageJson = readPackageJson(join(root, "package.json"));
  packageJson.version = options.version;
  packageJson.optionalDependencies = Object.fromEntries(
    nativePackages().map((nativePackage) => [nativePackage.packageName, options.version])
  );

  const outPath = join(options.outDir, "package.json");
  writeJson(outPath, packageJson);
  return packageJson;
}

function prepareNativePackage(nativePackage: ReturnType<typeof nativePackages>[number]): {
  binaryCopied: boolean;
  packageJson: PackageJson;
} {
  const packageDirName = `${nativePackage.platform}-${nativePackage.arch}`;
  const sourceDir = join(root, "npm", "native", packageDirName);
  const outDir = join(options.outDir, "npm", "native", packageDirName);
  mkdirSync(outDir, { recursive: true });

  const packageJson = readPackageJson(join(sourceDir, "package.json"));
  packageJson.version = options.version;
  packageJson.private = false;
  writeJson(join(outDir, "package.json"), packageJson);

  const sourceBinary = join(sourceDir, nativePackage.binaryPath);
  const outBinary = join(outDir, nativePackage.binaryPath);
  const binaryCopied = existsSync(sourceBinary);
  if (binaryCopied) {
    mkdirSync(dirname(outBinary), { recursive: true });
    copyFileSync(sourceBinary, outBinary);
  }

  return { binaryCopied, packageJson };
}

function validateRootPackageJson(packageJson: PackageJson): void {
  const optionalDependencies = packageJson.optionalDependencies ?? {};
  for (const nativePackage of nativePackages()) {
    if (optionalDependencies[nativePackage.packageName] !== options.version) {
      fail(
        `Root optional dependency ${nativePackage.packageName} is not pinned to ${options.version}`
      );
    }
  }
}

function validateNativePackage(packageJson: PackageJson): void {
  if (packageJson.version !== options.version) {
    fail(
      `${packageJson.name} version is ${packageJson.version}, expected ${options.version}`
    );
  }
  if (packageJson.private !== false) {
    fail(`${packageJson.name} private must be false in release prep output`);
  }
}

function readPackageJson(path: string): PackageJson {
  try {
    return JSON.parse(readFileSync(path, "utf-8")) as PackageJson;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    fail(`Failed to read package JSON at ${path}: ${message}`);
  }
}

function writeJson(path: string, value: unknown): void {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function parseArgs(): Options {
  const args = process.argv.slice(2);
  const version = valueAfter(args, "--version");
  if (!version) {
    fail(
      "Usage: bun tools/prepare-native-release.ts --version <version> [--out <dir>] [--require-binaries] [--keep]"
    );
  }

  return {
    version,
    outDir: resolve(
      valueAfter(args, "--out") ?? mkdtempSync(join(tmpdir(), "klint-native-release-"))
    ),
    requireBinaries: args.includes("--require-binaries"),
    keep: args.includes("--keep"),
  };
}

function valueAfter(args: string[], name: string): string | undefined {
  const index = args.indexOf(name);
  return index >= 0 ? args[index + 1] : undefined;
}

function fail(message: string): never {
  process.stderr.write(`${message}\n`);
  process.exit(1);
}
