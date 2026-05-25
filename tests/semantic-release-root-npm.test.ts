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
      JSON.stringify({ name: "@konvert7/klint", version: "0.0.0" }, null, 2)
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
