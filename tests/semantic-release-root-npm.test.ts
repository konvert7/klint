import { describe, expect, test } from "bun:test";
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

    try {
      await prepare(
        {},
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
        "@konvert7/klint-linux-x64": "1.2.3",
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
