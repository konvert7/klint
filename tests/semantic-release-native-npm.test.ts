import { afterEach, describe, expect, spyOn, test } from "bun:test";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { prepare, publish, verifyConditions } from "../tools/semantic-release-native-npm";

const PACKAGES = [
  ["darwin-arm64", "@konvert7/klint-darwin-arm64", "bin/klint-rs"],
  ["darwin-x64", "@konvert7/klint-darwin-x64", "bin/klint-rs"],
  ["linux-x64", "@konvert7/klint-linux-x64", "bin/klint-rs"],
  ["linux-x64-musl", "@konvert7/klint-linux-x64-musl", "bin/klint-rs"],
  ["linux-arm64", "@konvert7/klint-linux-arm64", "bin/klint-rs"],
  ["win32-x64", "@konvert7/klint-win32-x64", "bin/klint-rs.exe"],
] as const;

describe("semantic-release native npm plugin", () => {
  test("prepare writes release versions and makes native packages publishable", async () => {
    const cwd = fixture();

    try {
      await prepare(
        {},
        {
          cwd,
          nextRelease: { version: "1.2.3" },
        }
      );

      for (const [dir, name] of PACKAGES) {
        const packageJson = readPackageJson(cwd, dir);
        expect(packageJson.name).toBe(name);
        expect(packageJson.version).toBe("1.2.3");
        expect(packageJson.private).toBe(false);
        expect(
          readFileSync(join(cwd, "npm", "native", dir, "bin", "VERSION"), "utf-8")
        ).toBe("1.2.3\n");
      }
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  test("prepare refuses to publish packages with missing binaries", async () => {
    const cwd = fixture({ skipBinaryFor: "linux-x64" });

    try {
      await expect(
        prepare(
          {},
          {
            cwd,
            nextRelease: { version: "1.2.3" },
          }
        )
      ).rejects.toThrow("Missing native binary for @konvert7/klint-linux-x64");
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  test("publish skips npm publish during semantic-release dry runs", async () => {
    const cwd = fixture();

    try {
      await publish(
        {},
        {
          cwd,
          options: { dryRun: true },
          logger: { log() {} },
        }
      );
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  test("publish skips missing native packages only with explicit env escape hatch", async () => {
    const cwd = fixture();
    const logs: string[] = [];
    const previous = process.env.KLINT_SKIP_MISSING_NATIVE_PUBLISH;

    try {
      process.env.KLINT_SKIP_MISSING_NATIVE_PUBLISH = "1";
      await publish(
        {
          spawnSync() {
            return {
              status: 1,
              stdout: "",
              stderr:
                "npm error code E404\nnpm error 404 Not Found - PUT https://registry.npmjs.org/@konvert7%2fklint-darwin-arm64 - Not found\n",
            };
          },
        },
        {
          cwd,
          options: {},
          logger: { log: (message: string) => logs.push(message) },
        }
      );

      expect(logs).toHaveLength(PACKAGES.length);
      expect(logs[0]).toContain("Skipping @konvert7/klint-darwin-arm64");
    } finally {
      restoreEnv("KLINT_SKIP_MISSING_NATIVE_PUBLISH", previous);
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  test("publish fails on missing native packages by default", async () => {
    const cwd = fixture();
    const previous = process.env.KLINT_SKIP_MISSING_NATIVE_PUBLISH;

    try {
      delete process.env.KLINT_SKIP_MISSING_NATIVE_PUBLISH;
      await expect(
        publish(
          {
            spawnSync() {
              return {
                status: 1,
                stdout: "",
                stderr:
                  "npm error code E404\nnpm error 404 Not Found - PUT https://registry.npmjs.org/@konvert7%2fklint-darwin-arm64 - Not found\n",
              };
            },
          },
          {
            cwd,
            options: {},
            logger: { log() {} },
          }
        )
      ).rejects.toThrow("@konvert7/klint-darwin-arm64 npm publish failed");
    } finally {
      restoreEnv("KLINT_SKIP_MISSING_NATIVE_PUBLISH", previous);
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  test("publish still fails on non-404 npm errors", async () => {
    const cwd = fixture();

    try {
      await expect(
        publish(
          {
            spawnSync() {
              return {
                status: 1,
                stdout: "",
                stderr: "npm error code E500\n",
              };
            },
          },
          {
            cwd,
            options: {},
            logger: { log() {} },
          }
        )
      ).rejects.toThrow("@konvert7/klint-darwin-arm64 npm publish failed");
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  test("verifyConditions rejects mismatched package names", async () => {
    const cwd = fixture({ badNameFor: "darwin-arm64" });

    try {
      await expect(verifyConditions({}, { cwd })).rejects.toThrow(
        "darwin-arm64 package name is bad"
      );
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });
});

type SpawnResult = { status: number | null; stdout?: string; stderr?: string };

function recordingSpawn(result: SpawnResult) {
  const calls: unknown[][] = [];

  return {
    calls,
    spawnSync(...args: unknown[]) {
      calls.push(args);

      return result;
    },
  };
}

async function withSkipEnv(run: () => unknown): Promise<void> {
  const previous = process.env.KLINT_SKIP_MISSING_NATIVE_PUBLISH;
  process.env.KLINT_SKIP_MISSING_NATIVE_PUBLISH = "1";

  try {
    await run();
  } finally {
    restoreEnv("KLINT_SKIP_MISSING_NATIVE_PUBLISH", previous);
  }
}

function fixture(options: { skipBinaryFor?: string; badNameFor?: string } = {}): string {
  const cwd = mkdtempSync(join(tmpdir(), "klint-native-npm-plugin-"));

  for (const [dir, name, binaryPath] of PACKAGES) {
    const packageDir = join(cwd, "npm", "native", dir);
    mkdirSync(join(packageDir, "bin"), { recursive: true });
    writeFileSync(
      join(packageDir, "package.json"),
      JSON.stringify(
        {
          name: options.badNameFor === dir ? "bad" : name,
          version: "0.0.0",
          private: true,
        },
        null,
        2
      )
    );

    if (options.skipBinaryFor !== dir) {
      writeFileSync(join(packageDir, binaryPath), "");
    }
  }

  return cwd;
}

function readPackageJson(cwd: string, dir: string) {
  return JSON.parse(
    readFileSync(join(cwd, "npm", "native", dir, "package.json"), "utf-8")
  );
}

function restoreEnv(name: string, value: string | undefined): void {
  if (value === undefined) delete process.env[name];
  else process.env[name] = value;
}

describe("semantic-release native npm plugin, edge cases", () => {
  const cwds: string[] = [];
  const newFixture = () => {
    const cwd = fixture();
    cwds.push(cwd);

    return cwd;
  };

  afterEach(() => {
    for (const cwd of cwds.splice(0)) rmSync(cwd, { recursive: true, force: true });
  });

  test("verifyConditions accepts the expected package names", async () => {
    await expect(verifyConditions({}, { cwd: newFixture() })).resolves.toBeUndefined();
  });

  test("prepare fails without a release version", async () => {
    await expect(prepare({}, { cwd: newFixture() })).rejects.toThrow(
      new Error("semantic-release did not provide nextRelease.version")
    );
  });

  test("publish runs npm publish --access public in every package folder, in order", async () => {
    const cwd = newFixture();
    const npm = recordingSpawn({ status: 0 });

    await publish(npm, { cwd });

    expect(npm.calls).toEqual(
      PACKAGES.map(([dir]) => [
        "npm",
        ["publish", "--access", "public"],
        {
          cwd: join(cwd, "npm", "native", dir),
          encoding: "utf-8",
          stdio: ["ignore", "pipe", "pipe"],
        },
      ])
    );
  });

  test("publish forwards npm's output", async () => {
    const out = spyOn(process.stdout, "write").mockImplementation(() => true);
    const err = spyOn(process.stderr, "write").mockImplementation(() => true);

    try {
      await publish(recordingSpawn({ status: 0, stdout: "out", stderr: "err" }), {
        cwd: newFixture(),
      });
      expect(out.mock.calls).toEqual(PACKAGES.map(() => ["out"]));
      expect(err.mock.calls).toEqual(PACKAGES.map(() => ["err"]));
    } finally {
      out.mockRestore();
      err.mockRestore();
    }
  });

  test("publish reports a killed npm as exit code -1", async () => {
    await expect(
      publish(recordingSpawn({ status: null }), { cwd: newFixture() })
    ).rejects.toThrow(
      new Error("@konvert7/klint-darwin-arm64 npm publish failed with exit code -1")
    );
  });

  test("a dry run logs once and never calls npm, and needs no logger", async () => {
    const logs: string[] = [];
    const npm = recordingSpawn({ status: 0 });

    await publish(npm, {
      cwd: newFixture(),
      options: { dryRun: true },
      logger: { log: (message: string) => logs.push(message) },
    });
    await publish(npm, { cwd: newFixture(), options: { dryRun: true } });

    expect(logs).toEqual(["Dry run: skipping native npm publishes"]);
    expect(npm.calls).toEqual([]);
  });

  test.each([
    { stdout: "npm error code E404" },
    { stderr: "npm error 404 Not Found - PUT" },
    { stderr: "package not found" },
    { stdout: "could not be found or you do not have permission to use it" },
  ])("with the escape hatch, skips a package npm reports missing: %p", async (output) => {
    await withSkipEnv(() =>
      expect(
        publish(recordingSpawn({ status: 1, ...output }), { cwd: newFixture() })
      ).resolves.toBeUndefined()
    );
  });

  test("with the escape hatch, still fails on other npm errors", async () => {
    await withSkipEnv(() =>
      expect(
        publish(recordingSpawn({ status: 1, stderr: "npm error code E500" }), {
          cwd: newFixture(),
        })
      ).rejects.toThrow(
        "@konvert7/klint-darwin-arm64 npm publish failed with exit code 1"
      )
    );
  });
});
