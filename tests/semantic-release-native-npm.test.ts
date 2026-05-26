import { describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { prepare, publish, verifyConditions } from "../tools/semantic-release-native-npm";

const PACKAGES = [
  ["darwin-arm64", "@konvert7/klint-darwin-arm64", "bin/klint-rs"],
  ["darwin-x64", "@konvert7/klint-darwin-x64", "bin/klint-rs"],
  ["linux-x64", "@konvert7/klint-linux-x64", "bin/klint-rs"],
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

  test("publish skips missing native packages during dark rollout", async () => {
    const cwd = fixture();
    const logs: string[] = [];

    try {
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
