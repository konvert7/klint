import { spawnSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

export async function verifyConditions(_, context) {
  const cwd = context.cwd ?? process.cwd();
  const packageJson = readPackageJson(cwd);

  if (!packageJson.name) {
    throw new Error("Root package.json must have a name before npm publish");
  }
}

export async function prepare(_, context) {
  const cwd = context.cwd ?? process.cwd();
  const version = context.nextRelease?.version;

  if (!version) {
    throw new Error("semantic-release did not provide nextRelease.version");
  }

  const packageJson = readPackageJson(cwd);
  packageJson.version = version;
  writePackageJson(cwd, packageJson);
}

export async function publish(_, context) {
  const cwd = context.cwd ?? process.cwd();

  if (context.options?.dryRun) {
    context.logger?.log("Dry run: skipping npm publish");
    return;
  }

  const result = spawnSync("npm", ["publish", "--access", "public"], {
    cwd,
    encoding: "utf-8",
    stdio: "inherit",
  });

  if ((result.status ?? -1) !== 0) {
    throw new Error(`npm publish failed with exit code ${result.status ?? -1}`);
  }
}

function readPackageJson(cwd) {
  return JSON.parse(readFileSync(join(cwd, "package.json"), "utf-8"));
}

function writePackageJson(cwd, packageJson) {
  writeFileSync(join(cwd, "package.json"), `${JSON.stringify(packageJson, null, 2)}\n`);
}
