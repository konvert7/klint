import { describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { runKlint } from "../core/runner";
import type { ArchConfig } from "../core/types";

function lint(arch: ArchConfig, files: { path: string[]; content: string }[]) {
  const root = mkdtempSync(join(tmpdir(), "klint-arch-import-"));
  for (const f of files) {
    mkdirSync(join(root, ...f.path.slice(0, -1)), { recursive: true });
    writeFileSync(join(root, ...f.path), f.content);
  }
  const violations = runKlint({ root, include: ["."], rules: {}, arch }, {});
  rmSync(root, { recursive: true });
  return violations.filter((v) => v.rule === "arch/imports");
}

const LAYERS: ArchConfig["layers"] = {
  skills: ["assets/skills/**"],
  core: ["src/lib/**"],
};

describe("arch/imports — deny mode", () => {
  test("flags file in from-layer importing from deny-layer", () => {
    const v = lint(
      {
        layers: LAYERS,
        imports: [
          { from: "skills", deny: "core", message: "Skills must be self-contained" },
        ],
      },
      [
        {
          path: ["assets", "skills", "my-skill", "tools", "index.ts"],
          content: `import { foo } from "../../../../src/lib/utils";`,
        },
        { path: ["src", "lib", "utils.ts"], content: `export const foo = 1;` },
      ]
    );
    expect(v).toHaveLength(1);
    expect(v[0].message).toContain("self-contained");
    expect(v[0].rule).toBe("arch/imports");
    expect(v[0].severity).toBe("error");
  });

  test("does not flag file outside the from-layer", () => {
    const v = lint(
      {
        layers: LAYERS,
        imports: [{ from: "skills", deny: "core" }],
      },
      [
        // this file is in core, not skills — not in scope of the rule
        {
          path: ["src", "lib", "a.ts"],
          content: `import { foo } from "./b";`,
        },
        { path: ["src", "lib", "b.ts"], content: `export const foo = 1;` },
      ]
    );
    expect(v).toHaveLength(0);
  });

  test("does not flag npm package imports", () => {
    const v = lint(
      {
        layers: LAYERS,
        imports: [{ from: "skills", deny: "core" }],
      },
      [
        {
          path: ["assets", "skills", "my-skill", "tools", "index.ts"],
          content: `import { z } from "zod";`,
        },
      ]
    );
    expect(v).toHaveLength(0);
  });

  test("does not flag node: builtin imports", () => {
    const v = lint(
      {
        layers: LAYERS,
        imports: [{ from: "skills", deny: "core" }],
      },
      [
        {
          path: ["assets", "skills", "my-skill", "tools", "index.ts"],
          content: `import { readFileSync } from "node:fs";`,
        },
      ]
    );
    expect(v).toHaveLength(0);
  });

  test("does not flag import within the same layer", () => {
    const v = lint(
      {
        layers: LAYERS,
        imports: [{ from: "skills", deny: "core" }],
      },
      [
        {
          path: ["assets", "skills", "my-skill", "tools", "index.ts"],
          content: `import { helper } from "./helper";`,
        },
        {
          path: ["assets", "skills", "my-skill", "tools", "helper.ts"],
          content: `export const helper = 1;`,
        },
      ]
    );
    expect(v).toHaveLength(0);
  });

  test("respects severity override", () => {
    const v = lint(
      {
        layers: LAYERS,
        imports: [{ from: "skills", deny: "core", severity: "warn" }],
      },
      [
        {
          path: ["assets", "skills", "my-skill", "tools", "index.ts"],
          content: `import { foo } from "../../../../src/lib/utils";`,
        },
        { path: ["src", "lib", "utils.ts"], content: `export const foo = 1;` },
      ]
    );
    expect(v).toHaveLength(1);
    expect(v[0].severity).toBe("warn");
  });

  test("dynamic import is also flagged", () => {
    const v = lint(
      {
        layers: LAYERS,
        imports: [{ from: "skills", deny: "core" }],
      },
      [
        {
          path: ["assets", "skills", "my-skill", "tools", "index.ts"],
          content: `const m = await import("../../../../src/lib/utils");`,
        },
        { path: ["src", "lib", "utils.ts"], content: `export const foo = 1;` },
      ]
    );
    expect(v).toHaveLength(1);
  });
});

describe("arch/imports — type-only: allow", () => {
  test("does not flag import type when type-only: allow is set", () => {
    const v = lint(
      {
        layers: LAYERS,
        imports: [{ from: "skills", deny: "core", "type-only": "allow" }],
      },
      [
        {
          path: ["assets", "skills", "my-skill", "tools", "index.ts"],
          content: `import type { Foo } from "../../../../src/lib/types";`,
        },
        { path: ["src", "lib", "types.ts"], content: `export type Foo = string;` },
      ]
    );
    expect(v).toHaveLength(0);
  });

  test("still flags value import when type-only: allow is set", () => {
    const v = lint(
      {
        layers: LAYERS,
        imports: [{ from: "skills", deny: "core", "type-only": "allow" }],
      },
      [
        {
          path: ["assets", "skills", "my-skill", "tools", "index.ts"],
          content: `import { foo } from "../../../../src/lib/utils";`,
        },
        { path: ["src", "lib", "utils.ts"], content: `export const foo = 1;` },
      ]
    );
    expect(v).toHaveLength(1);
  });

  test("flags import type when type-only: allow is NOT set", () => {
    const v = lint(
      {
        layers: LAYERS,
        imports: [{ from: "skills", deny: "core" }],
      },
      [
        {
          path: ["assets", "skills", "my-skill", "tools", "index.ts"],
          content: `import type { Foo } from "../../../../src/lib/types";`,
        },
        { path: ["src", "lib", "types.ts"], content: `export type Foo = string;` },
      ]
    );
    expect(v).toHaveLength(1);
  });
});

const DAO_LAYERS: ArchConfig["layers"] = { dao: ["src/dao/**"] };

function lintDao(
  rule: Omit<NonNullable<ArchConfig["imports"]>[number], "from">,
  content: string
) {
  return lint({ layers: DAO_LAYERS, imports: [{ from: "dao", ...rule }] }, [
    { path: ["src", "dao", "users.ts"], content },
  ]);
}

describe("arch/imports — deny-packages", () => {
  test("flags a denied package import", () => {
    const v = lintDao(
      { "deny-packages": "next/headers", message: "DAO must not read request state" },
      `import { headers } from "next/headers";`
    );
    expect(v).toHaveLength(1);
    expect(v[0].message).toContain("request state");
    expect(v[0].rule).toBe("arch/imports");
    expect(v[0].line).toBe(1);
  });

  test("reports a default message when none is given", () => {
    const v = lintDao(
      { "deny-packages": ["next/headers"] },
      `import { headers } from "next/headers";`
    );
    expect(v).toHaveLength(1);
    expect(v[0].message).toBe("Import of a denied package");
  });

  test("matches subpaths of a denied package", () => {
    const v = lintDao({ "deny-packages": ["next"] }, `import "next/headers";`);
    expect(v).toHaveLength(1);
  });

  test("does not match a package that merely shares a prefix", () => {
    const v = lintDao({ "deny-packages": ["next"] }, `import "nextra";`);
    expect(v).toHaveLength(0);
  });

  test("does not match a sibling subpath", () => {
    const v = lintDao({ "deny-packages": ["next/headers"] }, `import "next/navigation";`);
    expect(v).toHaveLength(0);
  });

  test("flags node: builtins", () => {
    const v = lintDao({ "deny-packages": ["node:fs"] }, `import "node:fs/promises";`);
    expect(v).toHaveLength(1);
  });

  test("flags dynamic imports", () => {
    const v = lintDao(
      { "deny-packages": ["next/headers"] },
      `export const load = () => import("next/headers");`
    );
    expect(v).toHaveLength(1);
  });

  test("honors type-only: allow", () => {
    const v = lintDao(
      { "deny-packages": ["next/headers"], "type-only": "allow" },
      `import type { ReadonlyHeaders } from "next/headers";\nexport type H = ReadonlyHeaders;`
    );
    expect(v).toHaveLength(0);
  });

  test("flags import type when type-only: allow is NOT set", () => {
    const v = lintDao(
      { "deny-packages": ["next/headers"] },
      `import type { ReadonlyHeaders } from "next/headers";\nexport type H = ReadonlyHeaders;`
    );
    expect(v).toHaveLength(1);
  });

  test("leaves packages that are not denied alone", () => {
    const v = lintDao({ "deny-packages": ["next/headers"] }, `import "react";`);
    expect(v).toHaveLength(0);
  });

  test("composes with deny on project paths", () => {
    const v = lint(
      {
        layers: { dao: ["src/dao/**"], session: ["src/session.ts"] },
        imports: [{ from: "dao", deny: "session", "deny-packages": ["next/headers"] }],
      },
      [
        {
          path: ["src", "dao", "users.ts"],
          content: `import { headers } from "next/headers";\nimport { getUserId } from "../session";\nexport const x = [headers, getUserId];`,
        },
        { path: ["src", "session.ts"], content: `export const getUserId = () => "1";` },
      ]
    );
    expect(v).toHaveLength(2);
    expect(v.map((violation) => violation.line).sort()).toEqual([1, 2]);
  });
});

describe("arch/imports — allow (whitelist) mode", () => {
  test("does not flag import from explicitly allowed path", () => {
    const v = lint(
      {
        imports: [
          {
            from: ["src/dao/**"],
            allow: ["src/dao/**", "src/prisma/**"],
            message: "DAO may only import from dao or prisma",
          },
        ],
      },
      [
        {
          path: ["src", "dao", "user.ts"],
          content: `import { db } from "../prisma/client";`,
        },
        { path: ["src", "prisma", "client.ts"], content: `export const db = {};` },
      ]
    );
    expect(v).toHaveLength(0);
  });

  test("flags import from a path not in the allow list", () => {
    const v = lint(
      {
        imports: [
          {
            from: ["src/dao/**"],
            allow: ["src/dao/**", "src/prisma/**"],
            message: "DAO may only import from dao or prisma",
          },
        ],
      },
      [
        {
          path: ["src", "dao", "user.ts"],
          content: `import { service } from "../service/user";`,
        },
        { path: ["src", "service", "user.ts"], content: `export const service = {};` },
      ]
    );
    expect(v).toHaveLength(1);
    expect(v[0].message).toBe("DAO may only import from dao or prisma");
  });

  test("does not flag npm package in allow mode", () => {
    const v = lint(
      {
        imports: [
          {
            from: ["src/dao/**"],
            allow: ["src/dao/**"],
          },
        ],
      },
      [
        {
          path: ["src", "dao", "user.ts"],
          content: `import { z } from "zod";`,
        },
      ]
    );
    expect(v).toHaveLength(0);
  });
});
