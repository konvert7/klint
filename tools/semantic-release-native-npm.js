import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { spawnSync } from "node:child_process";

const NATIVE_PACKAGES = [
  {
    dir: "darwin-arm64",
    name: "@konvert7/klint-darwin-arm64",
    binaryPath: "bin/klint-rs",
  },
  {
    dir: "darwin-x64",
    name: "@konvert7/klint-darwin-x64",
    binaryPath: "bin/klint-rs",
  },
  {
    dir: "linux-x64",
    name: "@konvert7/klint-linux-x64",
    binaryPath: "bin/klint-rs",
  },
  {
    dir: "win32-x64",
    name: "@konvert7/klint-win32-x64",
    binaryPath: "bin/klint-rs.exe",
  },
];

export async function verifyConditions(_, context) {
  const cwd = context.cwd ?? process.cwd();

  for (const nativePackage of NATIVE_PACKAGES) {
    const packageJson = readPackageJson(packageDir(cwd, nativePackage));
    if (packageJson.name !== nativePackage.name) {
      throw new Error(
        `${nativePackage.dir} package name is ${packageJson.name}, expected ${nativePackage.name}`
      );
    }
  }
}

export async function prepare(_, context) {
  const cwd = context.cwd ?? process.cwd();
  const version = context.nextRelease?.version;

  if (!version) {
    throw new Error("semantic-release did not provide nextRelease.version");
  }

  for (const nativePackage of NATIVE_PACKAGES) {
    const dir = packageDir(cwd, nativePackage);
    const packageJson = readPackageJson(dir);
    packageJson.version = version;
    packageJson.private = false;
    writePackageJson(dir, packageJson);

    const binary = join(dir, nativePackage.binaryPath);
    if (!existsSync(binary)) {
      throw new Error(`Missing native binary for ${nativePackage.name}: ${binary}`);
    }
  }
}

export async function publish(_, context) {
  const cwd = context.cwd ?? process.cwd();

  if (context.options?.dryRun) {
    context.logger?.log("Dry run: skipping native npm publishes");
    return;
  }

  for (const nativePackage of NATIVE_PACKAGES) {
    const result = spawnSync("npm", ["publish", "--access", "public"], {
      cwd: packageDir(cwd, nativePackage),
      encoding: "utf-8",
      stdio: "inherit",
    });

    if ((result.status ?? -1) !== 0) {
      throw new Error(
        `${nativePackage.name} npm publish failed with exit code ${result.status ?? -1}`
      );
    }
  }
}

function packageDir(cwd, nativePackage) {
  return join(cwd, "npm", "native", nativePackage.dir);
}

function readPackageJson(dir) {
  return JSON.parse(readFileSync(join(dir, "package.json"), "utf-8"));
}

function writePackageJson(dir, packageJson) {
  writeFileSync(join(dir, "package.json"), `${JSON.stringify(packageJson, null, 2)}\n`);
}

