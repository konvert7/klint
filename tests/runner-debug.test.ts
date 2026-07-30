import { describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { runKlint } from "../core/runner";
import type { KlintRule } from "../core/types";

function fixture(): string {
  const root = mkdtempSync(join(tmpdir(), "klint-runner-debug-"));
  mkdirSync(join(root, "src"), { recursive: true });
  mkdirSync(join(root, "tests"), { recursive: true });
  mkdirSync(join(root, "assets", "demo", "node_modules", "pkg"), {
    recursive: true,
  });
  mkdirSync(join(root, "node_modules", "pkg"), { recursive: true });
  writeFileSync(join(root, "src", "index.ts"), "export const value = 1;\n");
  writeFileSync(join(root, "tests", "index.ts"), "export const test = 1;\n");
  writeFileSync(
    join(root, "assets", "demo", "node_modules", "pkg", "index.ts"),
    "export const nestedIgnored = 1;\n"
  );
  writeFileSync(
    join(root, "node_modules", "pkg", "index.ts"),
    "export const ignored = 1;\n"
  );
  return root;
}

describe("runner file resolution debug", () => {
  test("supports glob includes and skips excluded directories while walking", () => {
    const root = fixture();
    const seen: string[][] = [];
    const rule: KlintRule = {
      meta: {
        description: "capture files",
        examples: [],
      },
      check(ctx) {
        seen.push(ctx.files.map((file) => file.replaceAll("\\", "/")));
      },
    };

    try {
      const debug: string[] = [];
      runKlint(
        {
          root,
          include: ["**/*.ts", "!tests/**", "!node_modules/**", "!**/node_modules/**"],
          rules: { capture: "error" },
        },
        { capture: rule },
        {
          onDebug(event) {
            if (event.type === "files:resolved") debug.push(String(event.files));
          },
        }
      );

      expect(seen).toHaveLength(1);
      expect(seen[0]).toHaveLength(1);
      expect(seen[0][0]).toEndWith("/src/index.ts");
      expect(debug).toEqual(["1"]);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  test("skips nested node_modules when a directory include is used", () => {
    const root = fixture();
    const seen: string[][] = [];
    const rule: KlintRule = {
      meta: {
        description: "capture files",
        examples: [],
      },
      check(ctx) {
        seen.push(ctx.files.map((file) => file.replaceAll("\\", "/")));
      },
    };

    try {
      const debug: string[] = [];
      runKlint(
        {
          root,
          include: ["src", "assets", "!**/node_modules/**"],
          rules: { capture: "error" },
        },
        { capture: rule },
        {
          onDebug(event) {
            if (event.type === "files:resolved") debug.push(String(event.files));
          },
        }
      );

      expect(seen).toHaveLength(1);
      expect(seen[0]).toHaveLength(1);
      expect(seen[0][0]).toEndWith("/src/index.ts");
      expect(debug).toEqual(["1"]);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  test("excludes individual files, not only directories", () => {
    const root = mkdtempSync(join(tmpdir(), "klint-runner-exclude-file-"));
    mkdirSync(join(root, "src", "jobs"), { recursive: true });
    writeFileSync(join(root, "src", "jobs", "keep.ts"), "export const a = 1;\n");
    writeFileSync(join(root, "src", "jobs", "worker.ts"), "export const b = 2;\n");
    writeFileSync(join(root, "src", "jobs", "worker.test.ts"), "export const c = 3;\n");

    const seen: string[][] = [];
    const rule: KlintRule = {
      meta: { description: "capture files", examples: [] },
      check(ctx) {
        seen.push(ctx.files.map((file) => file.replaceAll("\\", "/")));
      },
    };

    try {
      runKlint(
        {
          root,
          include: ["src", "!**/*.test.ts", "!src/jobs/worker.ts"],
          rules: { capture: "error" },
        },
        { capture: rule }
      );

      expect(seen).toHaveLength(1);
      expect(seen[0]).toHaveLength(1);
      expect(seen[0][0]).toEndWith("/src/jobs/keep.ts");
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });
});
