import { describe, expect, test } from "bun:test";
import { replaceRequired, valueAfter } from "../tools/lib/release-cli";

describe("replaceRequired", () => {
  test("replaces the first match", () => {
    expect(
      replaceRequired('a = "1"\na = "2"\n', /^a = ".+"$/m, 'a = "9"', "x.toml")
    ).toBe('a = "9"\na = "2"\n');
  });

  test("names the pattern and the file when nothing matches", () => {
    expect(() => replaceRequired("b = 1\n", /^a = .+$/m, "a = 2", "x.toml")).toThrow(
      new Error("Expected release metadata pattern /^a = .+$/m in x.toml")
    );
  });
});

describe("valueAfter", () => {
  test("returns the argument after the flag, including at the start", () => {
    expect(valueAfter(["--version", "1.0.0", "--out", "dist"], "--version")).toBe(
      "1.0.0"
    );
    expect(valueAfter(["--version", "1.0.0", "--out", "dist"], "--out")).toBe("dist");
  });

  test("is undefined when the flag is missing or last", () => {
    expect(valueAfter(["--out", "dist"], "--version")).toBeUndefined();
    expect(valueAfter(["--out"], "--out")).toBeUndefined();
  });
});
