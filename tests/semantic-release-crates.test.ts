import { afterEach, describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { prepare, publish, verifyConditions } from "../tools/semantic-release-crates";

const ROOT = resolve(import.meta.dir, "..");
const MANIFEST = `[package]
name = "klint-rs"
version = "0.0.0"

[dependencies]
serde = { version = "1.0" }
`;

type Command = { command: string; args: string[] };

const workspaces: string[] = [];

function workspace(manifest = MANIFEST): string {
  const cwd = mkdtempSync(join(tmpdir(), "klint-crates-plugin-"));
  workspaces.push(cwd);
  mkdirSync(join(cwd, "crates", "klint-rs"), { recursive: true });
  mkdirSync(join(cwd, "skill", "klint-rules"), { recursive: true });
  writeFileSync(join(cwd, "crates", "klint-rs", "Cargo.toml"), manifest);
  writeFileSync(join(cwd, "skill", "klint-rules", "SKILL.md"), "# klint rules\n");
  return cwd;
}

function recorder(status = 0) {
  const commands: Command[] = [];
  const spawnSync = (command: string, args: string[]) => {
    commands.push({ command, args });
    return { status };
  };
  return { commands, spawnSync };
}

afterEach(() => {
  for (const cwd of workspaces.splice(0)) rmSync(cwd, { recursive: true, force: true });
});

describe("semantic-release crates plugin", () => {
  test("prepare stamps only the package version and refreshes the lockfile", async () => {
    const cwd = workspace();
    const { commands, spawnSync } = recorder();

    await prepare({ spawnSync }, { cwd, nextRelease: { version: "1.2.3" } });

    const manifest = readFileSync(join(cwd, "crates", "klint-rs", "Cargo.toml"), "utf-8");
    expect(manifest).toContain('\nversion = "1.2.3"\n');
    expect(manifest).toContain('serde = { version = "1.0" }');
    expect(commands).toEqual([{ command: "cargo", args: ["update", "--workspace"] }]);
  });

  test("prepare bundles the skill inside the crate so the package can build alone", async () => {
    const cwd = workspace();

    await prepare(recorder(), { cwd, nextRelease: { version: "1.2.3" } });

    expect(
      readFileSync(join(cwd, "crates", "klint-rs", "skill", "SKILL.md"), "utf-8")
    ).toBe("# klint rules\n");
  });

  test("prepare fails loudly when the lockfile refresh fails", async () => {
    const cwd = workspace();

    await expect(
      prepare(recorder(101), { cwd, nextRelease: { version: "1.2.3" } })
    ).rejects.toThrow("cargo update failed with exit code 101");
  });

  test("publish uploads the stamped crate with the locked dependency set", async () => {
    const { commands, spawnSync } = recorder();

    await publish(
      { spawnSync },
      { cwd: workspace(), nextRelease: { version: "1.2.3" }, options: {} }
    );

    expect(commands).toEqual([
      {
        command: "cargo",
        args: ["publish", "-p", "klint-rs", "--locked", "--allow-dirty"],
      },
    ]);
  });

  test("publish fails loudly when cargo publish fails", async () => {
    await expect(
      publish(recorder(1), {
        cwd: workspace(),
        nextRelease: { version: "1.2.3" },
        options: {},
      })
    ).rejects.toThrow("cargo publish failed with exit code 1");
  });

  test("publish skips crates.io during semantic-release dry runs", async () => {
    const { commands, spawnSync } = recorder();

    await publish({ spawnSync }, { cwd: workspace(), options: { dryRun: true } });

    expect(commands).toEqual([]);
  });

  test("verifyConditions rejects a manifest for another crate", async () => {
    const cwd = workspace(MANIFEST.replace("klint-rs", "other"));

    await expect(
      verifyConditions({}, { cwd, options: { dryRun: true } })
    ).rejects.toThrow("declares other, expected klint-rs");
  });

  test("verifyConditions requires a crates.io token outside dry runs", async () => {
    const { CARGO_REGISTRY_TOKEN: _inherited, ...env } = process.env;
    const saved = process.env;
    process.env = env;
    try {
      await expect(
        verifyConditions({}, { cwd: workspace(), options: {} })
      ).rejects.toThrow("CARGO_REGISTRY_TOKEN is unset");
    } finally {
      process.env = saved;
    }
  });

  test("the repository crate passes verification", async () => {
    await verifyConditions({}, { cwd: ROOT, options: { dryRun: true } });
  });
});
