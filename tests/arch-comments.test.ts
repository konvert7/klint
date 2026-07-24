import { describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { KlintConfigSchema } from "../core/config.schema";
import { runKlint } from "../core/runner";
import type { ArchConfig } from "../core/types";

function lint(
  arch: ArchConfig,
  files: { path: string[]; content: string }[],
  rule: string
) {
  const root = mkdtempSync(join(tmpdir(), "klint-arch-comments-"));
  for (const f of files) {
    mkdirSync(join(root, ...f.path.slice(0, -1)), { recursive: true });
    writeFileSync(join(root, ...f.path), f.content);
  }
  const violations = runKlint({ root, include: ["."], rules: {}, arch }, {});
  rmSync(root, { recursive: true });
  return violations.filter((v) => v.rule === rule);
}

const DOC_BLOCK = "/**\n * doc line\n * doc line\n */\nconst x = 1;\n";

describe("arch/max-comment-density", () => {
  test("flags a file whose non-doc comment ratio exceeds the limit, at line 1", () => {
    const v = lint(
      { maxCommentDensity: [{ limit: 20, in: "src/**" }] },
      [
        {
          path: ["src", "noisy.ts"],
          content: "// a\n// b\nconst x = 1;\nconst y = 2;\nconst z = 3;\n",
        },
      ],
      "arch/max-comment-density"
    );
    expect(v).toHaveLength(1);
    expect(v[0].line).toBe(1);
    expect(v[0].severity).toBe("error");
  });

  test("does not flag a file under the limit", () => {
    const v = lint(
      { maxCommentDensity: [{ limit: 40, in: "src/**" }] },
      [
        {
          path: ["src", "ok.ts"],
          content: "// a\nconst x = 1;\nconst y = 2;\nconst z = 3;\nconst w = 4;\n",
        },
      ],
      "arch/max-comment-density"
    );
    expect(v).toHaveLength(0);
  });

  test("doc-comments are exempt by default", () => {
    const v = lint(
      { maxCommentDensity: [{ limit: 1, in: "src/**" }] },
      [{ path: ["src", "documented.ts"], content: DOC_BLOCK }],
      "arch/max-comment-density"
    );
    expect(v).toHaveLength(0);
  });

  test("countDocComments: true counts doc-comments toward the ratio", () => {
    const v = lint(
      { maxCommentDensity: [{ limit: 20, countDocComments: true, in: "src/**" }] },
      [{ path: ["src", "documented.ts"], content: DOC_BLOCK }],
      "arch/max-comment-density"
    );
    expect(v).toHaveLength(1);
  });

  test("counts inline (trailing) comment lines toward the ratio", () => {
    const v = lint(
      { maxCommentDensity: [{ limit: 40, in: "src/**" }] },
      [
        {
          path: ["src", "inline.ts"],
          content: "const a = 1; // x\nconst b = 2; // y\nconst c = 3;\n",
        },
      ],
      "arch/max-comment-density"
    );
    expect(v).toHaveLength(1);
  });
});

describe("arch/max-comment-block", () => {
  test("flags a comment block taller than the limit, at the first offending line", () => {
    const v = lint(
      { maxCommentBlock: [{ limit: 2, in: "src/**" }] },
      [{ path: ["src", "tall.ts"], content: "// a\n// b\n// c\nconst x = 1;\n" }],
      "arch/max-comment-block"
    );
    expect(v).toHaveLength(1);
    expect(v[0].line).toBe(3);
  });

  test("allows a block exactly at the limit", () => {
    const v = lint(
      { maxCommentBlock: [{ limit: 2, in: "src/**" }] },
      [{ path: ["src", "ok.ts"], content: "// a\n// b\nconst x = 1;\n" }],
      "arch/max-comment-block"
    );
    expect(v).toHaveLength(0);
  });

  test("a blank line breaks the run", () => {
    const v = lint(
      { maxCommentBlock: [{ limit: 2, in: "src/**" }] },
      [
        {
          path: ["src", "split.ts"],
          content: "// a\n// b\n\n// c\n// d\nconst x = 1;\n",
        },
      ],
      "arch/max-comment-block"
    );
    expect(v).toHaveLength(0);
  });

  test("a multi-line block comment counts its physical lines", () => {
    const v = lint(
      { maxCommentBlock: [{ limit: 2, in: "src/**" }] },
      [{ path: ["src", "block.ts"], content: "/* a\n b\n c */\nconst x = 1;\n" }],
      "arch/max-comment-block"
    );
    expect(v).toHaveLength(1);
    expect(v[0].line).toBe(3);
  });

  test("doc-comments are exempt by default", () => {
    const v = lint(
      { maxCommentBlock: [{ limit: 2, in: "src/**" }] },
      [{ path: ["src", "documented.ts"], content: DOC_BLOCK }],
      "arch/max-comment-block"
    );
    expect(v).toHaveLength(0);
  });

  test("countDocComments: true counts a tall doc block", () => {
    const v = lint(
      { maxCommentBlock: [{ limit: 2, countDocComments: true, in: "src/**" }] },
      [{ path: ["src", "documented.ts"], content: DOC_BLOCK }],
      "arch/max-comment-block"
    );
    expect(v).toHaveLength(1);
    expect(v[0].line).toBe(3);
  });
});

describe("arch comment rules — shared behavior", () => {
  test("respects in: scoping — files outside scope are ignored", () => {
    const v = lint(
      { maxCommentBlock: [{ limit: 1, in: "src/**" }] },
      [
        { path: ["src", "a.ts"], content: "// a\n// b\nconst x = 1;\n" },
        { path: ["lib", "b.ts"], content: "// a\n// b\nconst x = 1;\n" },
      ],
      "arch/max-comment-block"
    );
    expect(v).toHaveLength(1);
    expect(v[0].file).toBe("src/a.ts");
  });

  test("supports a custom message and severity override", () => {
    const v = lint(
      {
        maxCommentDensity: [
          { limit: 1, in: "src/**", message: "Too chatty", severity: "warn" },
        ],
      },
      [{ path: ["src", "a.ts"], content: "// a\n// b\nconst x = 1;\n" }],
      "arch/max-comment-density"
    );
    expect(v).toHaveLength(1);
    expect(v[0].message).toBe("Too chatty");
    expect(v[0].severity).toBe("warn");
  });
});

describe("arch comment rules — schema validation", () => {
  function parse(arch: unknown) {
    return KlintConfigSchema.safeParse({ include: ["."], rules: {}, arch });
  }

  test("density limit must be a positive percentage no greater than 100", () => {
    expect(parse({ maxCommentDensity: [{ limit: 0, in: "src/**" }] }).success).toBe(
      false
    );
    expect(parse({ maxCommentDensity: [{ limit: 101, in: "src/**" }] }).success).toBe(
      false
    );
    expect(parse({ maxCommentDensity: [{ limit: 2.5, in: "src/**" }] }).success).toBe(
      true
    );
  });

  test("block limit must be a positive integer", () => {
    expect(parse({ maxCommentBlock: [{ limit: 0, in: "src/**" }] }).success).toBe(false);
    expect(parse({ maxCommentBlock: [{ limit: 2.5, in: "src/**" }] }).success).toBe(
      false
    );
    expect(parse({ maxCommentBlock: [{ limit: 2, in: "src/**" }] }).success).toBe(true);
  });

  test("countDocComments is accepted", () => {
    expect(
      parse({ maxCommentBlock: [{ limit: 2, countDocComments: true, in: "src/**" }] })
        .success
    ).toBe(true);
  });
});
