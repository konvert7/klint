import { describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { runKlint } from "../core/runner";
import type { ArchConfig } from "../core/types";

function lint(arch: ArchConfig, files: { path: string[]; content: string }[]) {
  const root = mkdtempSync(join(tmpdir(), "klint-arch-singleton-"));
  for (const f of files) {
    mkdirSync(join(root, ...f.path.slice(0, -1)), { recursive: true });
    writeFileSync(join(root, ...f.path), f.content);
  }
  const violations = runKlint({ root, include: ["."], rules: {}, arch }, {});
  rmSync(root, { recursive: true });
  return violations.filter((v) => v.rule === "arch/singleton");
}

describe("arch/singleton", () => {
  test("flags pattern found in a file that is not only", () => {
    const v = lint(
      {
        singleton: [
          {
            pattern: "process.env.PAL_HOME",
            only: "src/lib/paths.ts",
            message: "Use the paths module",
          },
        ],
      },
      [
        {
          path: ["src", "lib", "paths.ts"],
          content: `export const home = process.env.PAL_HOME;`,
        },
        {
          path: ["src", "tools", "deploy.ts"],
          content: `const h = process.env.PAL_HOME;`,
        },
      ]
    );
    expect(v).toHaveLength(1);
    expect(v[0].file).toBe("src/tools/deploy.ts");
    expect(v[0].message).toBe("Use the paths module");
    expect(v[0].rule).toBe("arch/singleton");
    expect(v[0].severity).toBe("error");
  });

  test("does not flag pattern in the only file", () => {
    const v = lint(
      {
        singleton: [
          {
            pattern: "process.env.PAL_HOME",
            only: "src/lib/paths.ts",
            message: "Use the paths module",
          },
        ],
      },
      [
        {
          path: ["src", "lib", "paths.ts"],
          content: `export const home = process.env.PAL_HOME;`,
        },
      ]
    );
    expect(v).toHaveLength(0);
  });

  test("does not flag when pattern is absent in other files", () => {
    const v = lint(
      {
        singleton: [
          {
            pattern: "process.env.PAL_HOME",
            only: "src/lib/paths.ts",
            message: "Use the paths module",
          },
        ],
      },
      [
        {
          path: ["src", "lib", "paths.ts"],
          content: `export const home = process.env.PAL_HOME;`,
        },
        { path: ["src", "tools", "deploy.ts"], content: `const h = "some-path";` },
      ]
    );
    expect(v).toHaveLength(0);
  });

  test("flags every line in every non-only file", () => {
    const v = lint(
      {
        singleton: [
          {
            pattern: "process.env.API_KEY",
            only: "src/lib/auth.ts",
            message: "Use the auth module",
          },
        ],
      },
      [
        { path: ["src", "lib", "auth.ts"], content: `const key = process.env.API_KEY;` },
        {
          path: ["src", "hooks", "a.ts"],
          content: `const k1 = process.env.API_KEY;\nconst k2 = process.env.API_KEY;`,
        },
        { path: ["src", "hooks", "b.ts"], content: `const k = process.env.API_KEY;` },
      ]
    );
    expect(v).toHaveLength(3);
  });

  test("respects severity override", () => {
    const v = lint(
      {
        singleton: [
          {
            pattern: "process.env.PAL_HOME",
            only: "src/lib/paths.ts",
            message: "Use the paths module",
            severity: "warn",
          },
        ],
      },
      [
        {
          path: ["src", "lib", "paths.ts"],
          content: `export const home = process.env.PAL_HOME;`,
        },
        { path: ["src", "other.ts"], content: `const h = process.env.PAL_HOME;` },
      ]
    );
    expect(v).toHaveLength(1);
    expect(v[0].severity).toBe("warn");
  });

  test("in field limits scan to scoped files only", () => {
    const v = lint(
      {
        singleton: [
          {
            pattern: "process.env.PAL_HOME",
            only: "src/lib/paths.ts",
            in: ["src/**"],
            message: "Use the paths module",
          },
        ],
      },
      [
        {
          path: ["src", "lib", "paths.ts"],
          content: `export const home = process.env.PAL_HOME;`,
        },
        // in src/ — should be flagged
        { path: ["src", "other.ts"], content: `const h = process.env.PAL_HOME;` },
        // outside src/ — should NOT be flagged even though it contains the pattern
        {
          path: ["klint", "tests", "fixture.ts"],
          content: `const h = process.env.PAL_HOME;`,
        },
      ]
    );
    expect(v).toHaveLength(1);
    expect(v[0].file).toBe("src/other.ts");
  });
});

describe("arch/singleton — jsx-element", () => {
  test("allows the element inside the only file", () => {
    const v = lint(
      {
        singleton: [
          {
            "jsx-element": "button",
            only: "src/components/ui/button.tsx",
            in: "src/**/*.tsx",
            message: "Raw <button> belongs only to the Button primitive",
          },
        ],
      },
      [
        {
          path: ["src", "components", "ui", "button.tsx"],
          content: `export function Button(p) { return <button {...p} />; }`,
        },
      ]
    );
    expect(v).toHaveLength(0);
  });

  test("flags the element outside the only file", () => {
    const v = lint(
      {
        singleton: [
          {
            "jsx-element": "button",
            only: "src/components/ui/button.tsx",
            in: "src/**/*.tsx",
            message: "Raw <button> belongs only to the Button primitive",
          },
        ],
      },
      [
        {
          path: ["src", "components", "ui", "button.tsx"],
          content: `export function Button(p) { return <button {...p} />; }`,
        },
        {
          path: ["src", "app", "page.tsx"],
          content: `export default function P() { return <button>x</button>; }`,
        },
      ]
    );
    expect(v).toHaveLength(1);
    expect(v[0].file).toBe("src/app/page.tsx");
  });

  test("array of tag names pins multiple primitives at once", () => {
    const v = lint(
      {
        singleton: [
          {
            "jsx-element": ["input", "label"],
            only: "src/components/ui/form.tsx",
            in: "src/**/*.tsx",
            message: "Use Input/Label primitives",
          },
        ],
      },
      [
        {
          path: ["src", "components", "ui", "form.tsx"],
          content: `export const I = () => <><label/><input/></>;`,
        },
        {
          path: ["src", "app", "page.tsx"],
          content: `export default function P() { return <input/>; }`,
        },
      ]
    );
    expect(v).toHaveLength(1);
    expect(v[0].file).toBe("src/app/page.tsx");
  });
});

describe("arch/singleton — regex (re: prefix)", () => {
  test("pins a regex match to the only file", () => {
    const v = lint(
      {
        singleton: [
          {
            pattern: "re:process\\.env\\.[A-Z_]+",
            only: "src/lib/env.ts",
            message: "Read env only through the env module",
          },
        ],
      },
      [
        {
          path: ["src", "lib", "env.ts"],
          content: `export const key = process.env.API_KEY;`,
        },
        {
          path: ["src", "tools", "deploy.ts"],
          content: `const k = process.env.API_KEY;`,
        },
      ]
    );
    expect(v).toHaveLength(1);
    expect(v[0].file).toBe("src/tools/deploy.ts");
  });
});
