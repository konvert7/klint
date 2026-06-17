import { describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { runKlint } from "../core/runner";
import type { ArchConfig } from "../core/types";

function lint(arch: ArchConfig, files: { path: string[]; content: string }[]) {
  const root = mkdtempSync(join(tmpdir(), "klint-arch-forbidden-"));
  for (const f of files) {
    mkdirSync(join(root, ...f.path.slice(0, -1)), { recursive: true });
    writeFileSync(join(root, ...f.path), f.content);
  }
  const violations = runKlint({ root, include: ["."], rules: {}, arch }, {});
  rmSync(root, { recursive: true });
  return violations.filter((v) => v.rule === "arch/forbidden");
}

describe("arch/forbidden", () => {
  test("flags pattern found in the scoped layer", () => {
    const v = lint(
      {
        layers: { lib: ["src/lib/**"] },
        forbidden: [
          {
            pattern: "console.log(",
            in: "lib",
            message: "Leaks into agent event stream",
          },
        ],
      },
      [
        {
          path: ["src", "lib", "utils.ts"],
          content: `export function debug() { console.log("hi"); }`,
        },
      ]
    );
    expect(v).toHaveLength(1);
    expect(v[0].message).toBe("Leaks into agent event stream");
    expect(v[0].rule).toBe("arch/forbidden");
    expect(v[0].severity).toBe("error");
  });

  test("does not flag pattern found outside the scoped layer", () => {
    const v = lint(
      {
        layers: { lib: ["src/lib/**"] },
        forbidden: [
          {
            pattern: "console.log(",
            in: "lib",
            message: "Leaks into agent event stream",
          },
        ],
      },
      [
        // this file is outside src/lib — not in scope
        {
          path: ["src", "scripts", "debug.ts"],
          content: `console.log("debugging");`,
        },
      ]
    );
    expect(v).toHaveLength(0);
  });

  test("does not flag when pattern is absent", () => {
    const v = lint(
      {
        layers: { lib: ["src/lib/**"] },
        forbidden: [
          {
            pattern: "console.log(",
            in: "lib",
            message: "Leaks into agent event stream",
          },
        ],
      },
      [{ path: ["src", "lib", "utils.ts"], content: `export const x = 1;` }]
    );
    expect(v).toHaveLength(0);
  });

  test("flags each line containing the pattern", () => {
    const v = lint(
      {
        layers: { lib: ["src/lib/**"] },
        forbidden: [
          { pattern: "process.exit(", in: "lib", message: "Use throw instead" },
        ],
      },
      [
        {
          path: ["src", "lib", "utils.ts"],
          content: `if (err) process.exit(1);\nif (fatal) process.exit(2);`,
        },
      ]
    );
    expect(v).toHaveLength(2);
    expect(v[0].line).toBe(1);
    expect(v[1].line).toBe(2);
  });

  test("respects severity override", () => {
    const v = lint(
      {
        layers: { lib: ["src/lib/**"] },
        forbidden: [
          {
            pattern: "console.log(",
            in: "lib",
            message: "Use logger",
            severity: "warn",
          },
        ],
      },
      [
        {
          path: ["src", "lib", "utils.ts"],
          content: `console.log("x");`,
        },
      ]
    );
    expect(v).toHaveLength(1);
    expect(v[0].severity).toBe("warn");
  });

  test("supports raw glob as in value (no named layer needed)", () => {
    const v = lint(
      {
        forbidden: [{ pattern: "console.log(", in: "src/lib/**", message: "Leaks" }],
      },
      [{ path: ["src", "lib", "utils.ts"], content: `console.log("hi");` }]
    );
    expect(v).toHaveLength(1);
  });
});

describe("arch/forbidden — jsx-element", () => {
  test("flags raw <button> in the scoped layer", () => {
    const v = lint(
      {
        forbidden: [
          {
            "jsx-element": "button",
            in: "src/**/*.tsx",
            message: "Use <Button /> from @/components/ui/button",
          },
        ],
      },
      [
        {
          path: ["src", "app", "page.tsx"],
          content: `export default function Page() { return <button>Click</button>; }`,
        },
      ]
    );
    expect(v).toHaveLength(1);
    expect(v[0].message).toBe("Use <Button /> from @/components/ui/button");
    expect(v[0].line).toBe(1);
  });

  test("does NOT flag <Button> (PascalCase component)", () => {
    const v = lint(
      {
        forbidden: [
          {
            "jsx-element": "button",
            in: "src/**/*.tsx",
            message: "Use <Button />",
          },
        ],
      },
      [
        {
          path: ["src", "app", "page.tsx"],
          content: `export default function Page() { return <Button>Click</Button>; }`,
        },
      ]
    );
    expect(v).toHaveLength(0);
  });

  test("does NOT flag <buttonGroup> (different tag name — AST advantage over substring)", () => {
    const v = lint(
      {
        forbidden: [
          {
            "jsx-element": "button",
            in: "src/**/*.tsx",
            message: "Use <Button />",
          },
        ],
      },
      [
        {
          path: ["src", "app", "page.tsx"],
          content: `export default function Page() { return <buttonGroup>x</buttonGroup>; }`,
        },
      ]
    );
    expect(v).toHaveLength(0);
  });

  test("skips non-JSX files (.ts)", () => {
    const v = lint(
      {
        forbidden: [{ "jsx-element": "button", in: "src/**", message: "no raw button" }],
      },
      [
        {
          path: ["src", "lib", "html.ts"],
          content: `export const html = "<button>x</button>";`,
        },
      ]
    );
    expect(v).toHaveLength(0);
  });

  test("flags self-closing elements (e.g. <input />)", () => {
    const v = lint(
      {
        forbidden: [
          {
            "jsx-element": ["input", "label"],
            in: "src/**/*.tsx",
            message: "Use @/components/ui/* primitives",
          },
        ],
      },
      [
        {
          path: ["src", "app", "form.tsx"],
          content: `export default function F() { return <><label>Name</label><input /></>; }`,
        },
      ]
    );
    expect(v).toHaveLength(2);
  });

  test("layer exclude lets design-system files use raw elements", () => {
    const v = lint(
      {
        layers: {
          "app-ui": [
            "src/app/**/*.tsx",
            "src/components/**/*.tsx",
            "!src/components/ui/**",
          ],
        },
        forbidden: [{ "jsx-element": "button", in: "app-ui", message: "Use <Button />" }],
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

  test("respects severity override", () => {
    const v = lint(
      {
        forbidden: [
          {
            "jsx-element": "button",
            in: "src/**/*.tsx",
            message: "warn me",
            severity: "warn",
          },
        ],
      },
      [{ path: ["src", "app", "page.tsx"], content: `const x = <button />;` }]
    );
    expect(v).toHaveLength(1);
    expect(v[0].severity).toBe("warn");
  });
});

describe("arch/forbidden — regex (re: prefix)", () => {
  test("flags lines matching the regex, skips non-matching lines", () => {
    const v = lint(
      {
        forbidden: [
          {
            pattern: "re:\\b(?:p|gap)-\\[",
            in: "src/**",
            message: "Use the spacing scale, not arbitrary values",
          },
        ],
      },
      [
        {
          path: ["src", "ui", "card.tsx"],
          content: `const a = <div className="p-[18px]" />;\nconst b = <div className="gap-4" />;\nconst c = <div className="gap-[3px]" />;`,
        },
      ]
    );
    expect(v).toHaveLength(2);
    expect(v.map((x) => x.line)).toEqual([1, 3]);
  });

  test("off-scale step regex flags gap-5 / p-7 but not gap-4", () => {
    const v = lint(
      {
        forbidden: [
          {
            pattern: "re:\\b(?:p|gap)-[3567]\\b",
            in: "src/**",
            message: "Off-scale spacing step",
          },
        ],
      },
      [
        {
          path: ["src", "ui", "row.tsx"],
          content: `const a = "gap-5";\nconst b = "gap-4";\nconst c = "p-7";`,
        },
      ]
    );
    expect(v.map((x) => x.line)).toEqual([1, 3]);
  });

  test("literal pattern is unaffected — 're' without colon is not a regex", () => {
    const v = lint(
      {
        forbidden: [{ pattern: "console.log(", in: "src/**", message: "no log" }],
      },
      [
        {
          path: ["src", "lib", "a.ts"],
          // a literal regex-special string must match literally, not as a pattern
          content: `console.log("x");`,
        },
      ]
    );
    expect(v).toHaveLength(1);
  });

  test("invalid regex fails loudly rather than silently passing", () => {
    expect(() =>
      lint(
        {
          forbidden: [{ pattern: "re:p-[", in: "src/**", message: "bad" }],
        },
        [{ path: ["src", "ui", "x.tsx"], content: `const a = "p-[1px]";` }]
      )
    ).toThrow(/invalid regex/);
  });
});
