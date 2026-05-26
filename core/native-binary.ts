import { existsSync, readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";

type NativePlatform = "darwin" | "linux" | "win32";
type NativeArch = "arm64" | "x64";

export interface NativePackage {
  platform: NativePlatform;
  arch: NativeArch;
  packageName: string;
  binaryPath: string;
}

const NATIVE_PACKAGES: Record<string, NativePackage> = {
  "darwin-arm64": {
    platform: "darwin",
    arch: "arm64",
    packageName: "@konvert7/klint-darwin-arm64",
    binaryPath: "bin/klint-rs",
  },
  "darwin-x64": {
    platform: "darwin",
    arch: "x64",
    packageName: "@konvert7/klint-darwin-x64",
    binaryPath: "bin/klint-rs",
  },
  "linux-x64": {
    platform: "linux",
    arch: "x64",
    packageName: "@konvert7/klint-linux-x64",
    binaryPath: "bin/klint-rs",
  },
  "win32-x64": {
    platform: "win32",
    arch: "x64",
    packageName: "@konvert7/klint-win32-x64",
    binaryPath: "bin/klint-rs.exe",
  },
};

interface NativeBinaryOptions {
  packageRoot: string;
  platform?: string;
  arch?: string;
  exists?: (path: string) => boolean;
  readPackageJson?: (path: string) => { version?: string };
  resolvePackageJson?: (packageName: string) => string;
}

export function nativePackageForPlatform(
  platform: string = process.platform,
  arch: string = process.arch
): NativePackage | undefined {
  return NATIVE_PACKAGES[`${platform}-${arch}`];
}

export function nativePackages(): NativePackage[] {
  return Object.values(NATIVE_PACKAGES);
}

export function resolveNativePackageBinary({
  packageRoot,
  platform = process.platform,
  arch = process.arch,
  exists = existsSync,
  readPackageJson = readJsonFile,
  resolvePackageJson,
}: NativeBinaryOptions): string | undefined {
  const nativePackage = nativePackageForPlatform(platform, arch);
  if (!nativePackage) return undefined;

  const packageJsonPath = resolveOptionalPackageJson(
    packageRoot,
    nativePackage.packageName,
    resolvePackageJson
  );
  if (!packageJsonPath) return undefined;
  if (!nativePackageVersionMatches(packageRoot, packageJsonPath, readPackageJson)) {
    return undefined;
  }

  const binaryPath = join(dirname(packageJsonPath), nativePackage.binaryPath);
  return exists(binaryPath) ? binaryPath : undefined;
}

function nativePackageVersionMatches(
  packageRoot: string,
  nativePackageJsonPath: string,
  readPackageJson: (path: string) => { version?: string }
): boolean {
  try {
    const rootVersion = readPackageJson(join(packageRoot, "package.json")).version;
    const nativeVersion = readPackageJson(nativePackageJsonPath).version;
    if (!rootVersion || !nativeVersion) return true;
    return rootVersion === nativeVersion;
  } catch {
    return true;
  }
}

function readJsonFile(path: string): { version?: string } {
  return JSON.parse(readFileSync(path, "utf-8")) as { version?: string };
}

function resolveOptionalPackageJson(
  packageRoot: string,
  packageName: string,
  resolvePackageJson?: (packageName: string) => string
): string | undefined {
  try {
    if (resolvePackageJson) return resolvePackageJson(packageName);
    return createRequire(join(packageRoot, "package.json")).resolve(
      `${packageName}/package.json`
    );
  } catch {
    return undefined;
  }
}
