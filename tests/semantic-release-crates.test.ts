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
type SpawnResult = { status: number | null; stderr?: string };

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

  test("the release commit keeps the crate version current", () => {
    const config = JSON.parse(readFileSync(join(ROOT, ".releaserc.json"), "utf-8"));
    const plugins: Array<string | [string, { assets?: string[] }]> = config.plugins;
    const names = plugins.map((plugin) => (Array.isArray(plugin) ? plugin[0] : plugin));
    const git = plugins.find(
      (plugin): plugin is [string, { assets: string[] }] =>
        Array.isArray(plugin) && plugin[0] === "@semantic-release/git"
    );

    expect(git?.[1].assets).toEqual(
      expect.arrayContaining(["crates/klint-rs/Cargo.toml", "Cargo.lock"])
    );
    expect(names.indexOf("./tools/semantic-release-crates.js")).toBeLessThan(
      names.indexOf("@semantic-release/git")
    );
  });
});

function fullRecorder(result: SpawnResult = { status: 0 }) {
  const calls: unknown[][] = [];

  return {
    calls,
    spawnSync(...args: unknown[]) {
      calls.push(args);

      return result;
    },
  };
}

function withToken(run: () => Promise<unknown>): Promise<unknown> {
  const saved = process.env.CARGO_REGISTRY_TOKEN;
  process.env.CARGO_REGISTRY_TOKEN = "token";

  return run().finally(() => {
    if (saved === undefined) delete process.env.CARGO_REGISTRY_TOKEN;
    else process.env.CARGO_REGISTRY_TOKEN = saved;
  });
}

describe("semantic-release crates plugin, edge cases", () => {
  test("verifyConditions passes with a token and no options", async () => {
    await withToken(() => verifyConditions({}, { cwd: workspace() }));
  });

  test("verifyConditions reads the package name, not a commented-out one", async () => {
    const cwd = workspace(`# was name = "old-klint"\n${MANIFEST}`);

    await verifyConditions({}, { cwd, options: { dryRun: true } });
  });

  test("verifyConditions names a manifest without a package name", async () => {
    const cwd = workspace(MANIFEST.replace('name = "klint-rs"\n', ""));

    await expect(
      verifyConditions({}, { cwd, options: { dryRun: true } })
    ).rejects.toThrow("crates/klint-rs/Cargo.toml declares undefined, expected klint-rs");
  });

  test("prepare stamps the package version line, not a key ending in version", async () => {
    const cwd = workspace(
      MANIFEST.replace(
        'version = "0.0.0"',
        'rust-version = "1.80"\nversion = "0.0.0" # stamped'
      )
    );

    await prepare(fullRecorder(), { cwd, nextRelease: { version: "1.2.3" } });

    const manifest = readFileSync(join(cwd, "crates", "klint-rs", "Cargo.toml"), "utf-8");
    expect(manifest).toContain('rust-version = "1.80"\nversion = "1.2.3" # stamped\n');
  });

  test("prepare runs cargo update in the workspace and logs the stamp", async () => {
    const cwd = workspace();
    const cargo = fullRecorder();
    const logs: string[] = [];

    await prepare(cargo, {
      cwd,
      nextRelease: { version: "1.2.3" },
      logger: { log: (message: string) => logs.push(message) },
    });

    expect(cargo.calls).toEqual([
      [
        "cargo",
        ["update", "--workspace"],
        { cwd, encoding: "utf-8", stdio: ["ignore", "ignore", "pipe"] },
      ],
    ]);
    expect(logs).toEqual(["Stamped klint-rs at 1.2.3 and bundled its skill"]);
  });

  test("prepare can run again over an already bundled skill", async () => {
    const cwd = workspace();

    await prepare(fullRecorder(), { cwd, nextRelease: { version: "1.2.3" } });
    await prepare(fullRecorder(), { cwd, nextRelease: { version: "1.2.4" } });

    expect(
      readFileSync(join(cwd, "crates", "klint-rs", "Cargo.toml"), "utf-8")
    ).toContain('version = "1.2.4"');
  });

  test("prepare fails without a release version", async () => {
    await expect(prepare(fullRecorder(), { cwd: workspace() })).rejects.toThrow(
      new Error("semantic-release did not provide nextRelease.version")
    );
  });

  test("a failed cargo update carries cargo's stderr, or nothing", async () => {
    await expect(
      prepare(fullRecorder({ status: 101, stderr: "boom" }), {
        cwd: workspace(),
        nextRelease: { version: "1.2.3" },
      })
    ).rejects.toThrow(new Error("cargo update failed with exit code 101\nboom"));
    await expect(
      prepare(fullRecorder({ status: 101 }), {
        cwd: workspace(),
        nextRelease: { version: "1.2.3" },
      })
    ).rejects.toThrow(new Error("cargo update failed with exit code 101\n"));
  });

  test("publish runs cargo publish in the workspace and logs it, with no options", async () => {
    const cwd = workspace();
    const cargo = fullRecorder();
    const logs: string[] = [];

    await publish(cargo, {
      cwd,
      nextRelease: { version: "1.2.3" },
      logger: { log: (message: string) => logs.push(message) },
    });

    expect(cargo.calls).toEqual([
      [
        "cargo",
        ["publish", "-p", "klint-rs", "--locked", "--allow-dirty"],
        { cwd, encoding: "utf-8", stdio: "inherit" },
      ],
    ]);
    expect(logs).toEqual(["Published klint-rs 1.2.3 to crates.io"]);
  });

  test("publish reports a killed cargo as exit code -1", async () => {
    await expect(
      publish(fullRecorder({ status: null }), {
        cwd: workspace(),
        nextRelease: { version: "1.2.3" },
      })
    ).rejects.toThrow(new Error("cargo publish failed with exit code -1"));
  });

  test("a dry run logs once, needs no logger and never calls cargo", async () => {
    const cargo = fullRecorder();
    const logs: string[] = [];

    await publish(cargo, {
      cwd: workspace(),
      options: { dryRun: true },
      logger: { log: (message: string) => logs.push(message) },
    });
    await publish(cargo, { cwd: workspace(), options: { dryRun: true } });

    expect(logs).toEqual(["Dry run: skipping the crates.io publish"]);
    expect(cargo.calls).toEqual([]);
  });
});
