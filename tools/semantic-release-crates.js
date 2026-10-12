import { spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";

const CRATE_NAME = "klint-rs";
const MANIFEST = "crates/klint-rs/Cargo.toml";
const WORKSPACE_SKILL = "skill/klint-rules/SKILL.md";
const PACKAGED_SKILL = "crates/klint-rs/skill/SKILL.md";

export async function verifyConditions(_, context) {
  const manifest = readManifest(cwdOf(context));
  if (crateNameOf(manifest) !== CRATE_NAME) {
    throw new Error(`${MANIFEST} declares ${crateNameOf(manifest)}, expected ${CRATE_NAME}`);
  }
  if (context.options?.dryRun) return;
  if (!process.env.CARGO_REGISTRY_TOKEN) {
    throw new Error(
      "CARGO_REGISTRY_TOKEN is unset; crates.io trusted publishing issued no token"
    );
  }
}

export async function prepare(pluginConfig = {}, context) {
  const cwd = cwdOf(context);
  const version = versionOf(context);

  writeManifest(cwd, stampVersion(readManifest(cwd), version));
  refreshLockfile(cwd, pluginConfig.spawnSync ?? spawnSync);
  bundleSkill(cwd);

  context.logger?.log(`Stamped ${CRATE_NAME} at ${version} and bundled its skill`);
}

export async function publish(pluginConfig = {}, context) {
  if (context.options?.dryRun) {
    context.logger?.log("Dry run: skipping the crates.io publish");
    return;
  }

  const spawn = pluginConfig.spawnSync ?? spawnSync;
  const result = spawn(
    "cargo",
    ["publish", "-p", CRATE_NAME, "--locked", "--allow-dirty"],
    { cwd: cwdOf(context), encoding: "utf-8", stdio: "inherit" }
  );
  if ((result.status ?? -1) !== 0) {
    throw new Error(`cargo publish failed with exit code ${result.status ?? -1}`);
  }

  context.logger?.log(`Published ${CRATE_NAME} ${versionOf(context)} to crates.io`);
}

function stampVersion(manifest, version) {
  return manifest.replace(/^version = ".*"$/m, `version = "${version}"`);
}

function refreshLockfile(cwd, spawn) {
  const result = spawn("cargo", ["update", "--workspace"], {
    cwd,
    encoding: "utf-8",
    stdio: ["ignore", "ignore", "pipe"],
  });
  if ((result.status ?? -1) !== 0) {
    throw new Error(
      `cargo update failed with exit code ${result.status ?? -1}\n${result.stderr ?? ""}`
    );
  }
}

function bundleSkill(cwd) {
  const target = join(cwd, PACKAGED_SKILL);
  mkdirSync(dirname(target), { recursive: true });
  copyFileSync(join(cwd, WORKSPACE_SKILL), target);
}

function crateNameOf(manifest) {
  return manifest.match(/^name = "(.*)"$/m)?.[1];
}

function readManifest(cwd) {
  return readFileSync(join(cwd, MANIFEST), "utf-8");
}

function writeManifest(cwd, manifest) {
  writeFileSync(join(cwd, MANIFEST), manifest);
}

function cwdOf(context) {
  return context.cwd ?? process.cwd();
}

function versionOf(context) {
  const version = context.nextRelease?.version;
  if (!version) throw new Error("semantic-release did not provide nextRelease.version");
  return version;
}
