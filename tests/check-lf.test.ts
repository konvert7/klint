import { describe, expect, test } from "bun:test";
import { hasCarriageReturn, isBinary, pathFrom } from "../tools/check-lf";

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
