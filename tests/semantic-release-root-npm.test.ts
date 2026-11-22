import { afterEach, describe, expect, test } from "bun:test";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { prepare, publish, verifyConditions } from "../tools/semantic-release-root-npm";

describe("semantic-release root npm plugin", () => {
  test("prepare writes the semantic-release version to root package.json", async () => {
    const cwd = mkdtempSync(join(tmpdir(), "klint-root-npm-plugin-"));
    writeFileSync(
      join(cwd, "package.json"),
      JSON.stringify(
        {
          name: "@konvert7/klint",
          version: "0.0.0",
          optionalDependencies: {
            "@konvert7/klint-darwin-arm64": "*",
          },
        },
        null,
        2
      )
    );

    const commands: Array<{ command: string; args: string[] }> = [];

    try {
      await prepare(
        {
          spawnSync(command: string, args: string[]) {
            commands.push({ command, args });

            return { status: 0 };
          },
        },
        {
          cwd,
          nextRelease: { version: "1.2.3" },
        }
      );

      const packageJson = JSON.parse(readFileSync(join(cwd, "package.json"), "utf-8"));
      expect(packageJson.version).toBe("1.2.3");
      expect(packageJson.optionalDependencies).toEqual({
        "@konvert7/klint-darwin-arm64": "*",
      });
      expect(commands).toEqual([
        {
          command: "cargo",
          args: [
            "run",
            "--quiet",
            "-p",
            "klint-rs",
            "--example",
            "generate_schema",
            "--",
            "1.2.3",
          ],
        },
      ]);
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  test("prepare fails loudly when schema regeneration fails", async () => {
    const cwd = mkdtempSync(join(tmpdir(), "klint-root-npm-plugin-"));
    writeFileSync(
      join(cwd, "package.json"),
      JSON.stringify({ name: "@konvert7/klint", version: "0.0.0" })
    );

    try {
      await expect(
        prepare(
          { spawnSync: () => ({ status: 1 }) },
          { cwd, nextRelease: { version: "1.2.3" } }
        )
      ).rejects.toThrow("Schema generation failed");
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  test("publish pins native optional dependencies to the release version", async () => {
    const cwd = mkdtempSync(join(tmpdir(), "klint-root-npm-plugin-"));
    writeFileSync(
      join(cwd, "package.json"),
      JSON.stringify({ name: "@konvert7/klint", version: "1.2.3" }, null, 2)
    );
    const publishes: Array<{ command: string; args: string[] }> = [];

    try {
      await publish(
        {
          spawnSync(command: string, args: string[]) {
            publishes.push({ command, args });

            return { status: 0 };
          },
        },
        {
          cwd,
          nextRelease: { version: "1.2.3" },
          options: {},
        }
      );

      const packageJson = JSON.parse(readFileSync(join(cwd, "package.json"), "utf-8"));
      expect(packageJson.optionalDependencies).toEqual({
        "@konvert7/klint-darwin-arm64": "1.2.3",
        "@konvert7/klint-darwin-x64": "1.2.3",
        "@konvert7/klint-linux-arm64": "1.2.3",
        "@konvert7/klint-linux-x64": "1.2.3",
        "@konvert7/klint-linux-x64-musl": "1.2.3",
        "@konvert7/klint-win32-x64": "1.2.3",
      });
      expect(publishes).toEqual([
        { command: "npm", args: ["publish", "--access", "public"] },
      ]);
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  test("publish skips npm publish during semantic-release dry runs", async () => {
    await publish(
      {},
      {
        cwd: process.cwd(),
        options: { dryRun: true },
        logger: { log() {} },
      }
    );
  });

  test("verifyConditions requires a package name", async () => {
    const cwd = mkdtempSync(join(tmpdir(), "klint-root-npm-plugin-"));
    writeFileSync(join(cwd, "package.json"), JSON.stringify({ version: "0.0.0" }));

    try {
      await expect(verifyConditions({}, { cwd })).rejects.toThrow(
        "Root package.json must have a name"
      );
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });
});

type SpawnResult = { status: number | null };

function recordingSpawn(result: SpawnResult = { status: 0 }) {
  const calls: unknown[][] = [];

  return {
    calls,
    spawnSync(...args: unknown[]) {
      calls.push(args);

      return result;
    },
  };
}

describe("semantic-release root npm plugin, edge cases", () => {
  const cwds: string[] = [];

  function project(packageJson: object = { name: "@konvert7/klint", version: "0.0.0" }) {
    const cwd = mkdtempSync(join(tmpdir(), "klint-root-npm-plugin-"));
    cwds.push(cwd);
    writeFileSync(join(cwd, "package.json"), JSON.stringify(packageJson));

    return cwd;
  }

  const packageJsonOf = (cwd: string) =>
    JSON.parse(readFileSync(join(cwd, "package.json"), "utf-8"));

  afterEach(() => {
    for (const cwd of cwds.splice(0)) rmSync(cwd, { recursive: true, force: true });
  });

  test("verifyConditions accepts a named package", async () => {
    await expect(verifyConditions({}, { cwd: project() })).resolves.toBeUndefined();
  });

  test("prepare regenerates the schema for the release version in the project", async () => {
    const cwd = project();
    const cargo = recordingSpawn();

    await prepare(cargo, { cwd, nextRelease: { version: "1.2.3" } });

    expect(cargo.calls).toEqual([
      [
        "cargo",
        [
          "run",
          "--quiet",
          "-p",
          "klint-rs",
          "--example",
          "generate_schema",
          "--",
          "1.2.3",
        ],
        { cwd, encoding: "utf-8", stdio: "inherit" },
      ],
    ]);
    expect(readFileSync(join(cwd, "package.json"), "utf-8")).toBe(
      `${JSON.stringify({ name: "@konvert7/klint", version: "1.2.3" }, null, 2)}\n`
    );
  });

  test("prepare and publish fail without a release version", async () => {
    const message = new Error("semantic-release did not provide nextRelease.version");

    await expect(prepare(recordingSpawn(), { cwd: project() })).rejects.toThrow(message);
    await expect(publish(recordingSpawn(), { cwd: project() })).rejects.toThrow(message);
  });

  test("a killed cargo or npm reports exit code -1", async () => {
    const killed = () => recordingSpawn({ status: null });
    const context = () => ({ cwd: project(), nextRelease: { version: "1.2.3" } });

    await expect(prepare(killed(), context())).rejects.toThrow(
      new Error("Schema generation failed with exit code -1")
    );
    await expect(publish(killed(), context())).rejects.toThrow(
      new Error("npm publish failed with exit code -1")
    );
  });

  test("publish keeps other optional dependencies and runs npm in the project", async () => {
    const cwd = project({
      name: "@konvert7/klint",
      version: "1.2.3",
      optionalDependencies: { other: "1.0.0" },
    });
    const npm = recordingSpawn();

    await publish(npm, { cwd, nextRelease: { version: "1.2.3" } });

    expect(packageJsonOf(cwd).optionalDependencies.other).toBe("1.0.0");
    expect(npm.calls).toEqual([
      [
        "npm",
        ["publish", "--access", "public"],
        { cwd, encoding: "utf-8", stdio: "inherit" },
      ],
    ]);
  });

  test("a dry run logs once, needs no logger and leaves package.json alone", async () => {
    const cwd = project();
    const before = readFileSync(join(cwd, "package.json"), "utf-8");
    const npm = recordingSpawn();
    const logs: string[] = [];

    await publish(npm, {
      cwd,
      nextRelease: { version: "1.2.3" },
      options: { dryRun: true },
      logger: { log: (message: string) => logs.push(message) },
    });
    await publish(npm, { cwd, options: { dryRun: true } });

    expect(logs).toEqual(["Dry run: skipping npm publish"]);
    expect(npm.calls).toEqual([]);
    expect(readFileSync(join(cwd, "package.json"), "utf-8")).toBe(before);
  });
});
