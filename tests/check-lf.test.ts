import { afterAll, describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { checkLf, hasCarriageReturn, isBinary, pathFrom } from "../tools/lib/check-lf";

const scratch = mkdtempSync(join(tmpdir(), "klint-check-lf-"));

afterAll(() => {
  rmSync(scratch, { recursive: true, force: true });
});

function git(root: string, ...args: string[]): void {
  const r = spawnSync("git", ["-c", "core.autocrlf=false", ...args], { cwd: root });
  if (r.status !== 0) throw new Error(`git ${args.join(" ")}: ${r.stderr}`);
}

function repoWith(files: Record<string, string | Buffer>): string {
  const root = mkdtempSync(join(scratch, "repo-"));

  for (const [name, contents] of Object.entries(files)) {
    writeFileSync(join(root, name), contents);
  }

  git(root, "init", "-q");
  git(root, "add", "-A");

  return root;
}

describe("hasCarriageReturn", () => {
  test("flags CRLF or mixed endings in the index or the worktree", () => {
    expect(hasCarriageReturn("i/crlf  w/crlf  attr/text=auto eol=lf \ta.ts")).toBe(true);
    expect(hasCarriageReturn("i/lf    w/mixed attr/text=auto eol=lf \ta.ts")).toBe(true);
  });

  test("accepts LF and files git has no ending for", () => {
    expect(hasCarriageReturn("i/lf    w/lf    attr/text=auto eol=lf \ta.ts")).toBe(false);
    expect(hasCarriageReturn("i/-text w/-text attr/text=auto eol=lf \ta.png")).toBe(
      false
    );
  });
});

describe("pathFrom", () => {
  test("keeps a path that contains a tab", () => {
    expect(pathFrom("i/lf    w/lf    attr/ \tdir/a\tb.ts")).toBe("dir/a\tb.ts");
  });
});

describe("isBinary", () => {
  test("treats a NUL byte as binary", () => {
    expect(isBinary(Buffer.from([0x50, 0x00, 0x0d, 0x0a]))).toBe(true);
    expect(isBinary(Buffer.from("a\r\nb"))).toBe(false);
  });
});

describe("checkLf", () => {
  test("passes a repo whose tracked files all use LF", () => {
    expect(checkLf(repoWith({ "a.ts": "a\nb\n" }), false)).toEqual({
      exitCode: 0,
      stdout: "All tracked files use LF line endings.\n",
      stderr: "",
    });
  });

  test("names every CRLF file and how to fix it, and changes nothing", () => {
    const root = repoWith({ "a.ts": "a\r\nb\r\n", "b.ts": "ok\n", "c.ts": "x\r\ny\n" });

    expect(checkLf(root, false)).toEqual({
      exitCode: 1,
      stdout: "",
      stderr:
        "CRLF found in tracked files (LF required):\na.ts\nc.ts\n" +
        "Run 'bun run lf:fix' to convert them.\n",
    });
    expect(readFileSync(join(root, "a.ts"), "utf8")).toBe("a\r\nb\r\n");
  });

  test("fix converts every CRLF file to LF", () => {
    const root = repoWith({ "a.ts": "a\r\nb\r\n", "c.ts": "x\r\ny\n" });

    expect(checkLf(root, true)).toEqual({
      exitCode: 0,
      stdout: "Converted 2 file(s) to LF.\n",
      stderr: "",
    });
    expect(readFileSync(join(root, "a.ts"), "utf8")).toBe("a\nb\n");
    expect(readFileSync(join(root, "c.ts"), "utf8")).toBe("x\ny\n");
  });

  test("fix skips a file whose working copy turned binary since it was staged", () => {
    const root = repoWith({ "a.ts": "a\r\n", "b.ts": "b\r\n", "c.ts": "x\r\ny\n" });
    const binary = Buffer.from([0x61, 0x00, 0x0d, 0x0a]);
    writeFileSync(join(root, "a.ts"), binary);
    writeFileSync(join(root, "b.ts"), binary);

    expect(checkLf(root, true)).toEqual({
      exitCode: 1,
      stdout: "Converted 1 file(s) to LF.\n",
      stderr: "Skipped binary file(s):\na.ts\nb.ts\n",
    });
    expect(readFileSync(join(root, "a.ts"))).toEqual(binary);
    expect(readFileSync(join(root, "c.ts"), "utf8")).toBe("x\ny\n");
  });

  test("fails with git's own message outside a git checkout", () => {
    expect(() => checkLf(mkdtempSync(join(scratch, "plain-")), false)).toThrow(
      /not a git repository/
    );
  });
});
