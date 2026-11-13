import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import {
  nonExactVersions,
  trackedManifests,
  versionsStampedAtRelease,
} from "../tools/check-exact-versions";

describe("nonExactVersions", () => {
  test("flags caret, tilde, latest, ranges and wildcards in every dependency section", () => {
    const manifest = {
      dependencies: { a: "^1.2.3", b: "1.2.3" },
      devDependencies: { c: "~2.0.0", d: "latest" },
      optionalDependencies: { e: ">=1.0.0", f: "*" },
      peerDependencies: { g: "1.x" },
      overrides: { h: "*" },
    };
    expect(nonExactVersions(manifest).map(({ name }) => name)).toEqual([
      "a",
      "c",
      "d",
      "e",
      "f",
      "g",
      "h",
    ]);
  });

  test("accepts exact versions, prereleases included", () => {
    const manifest = {
      dependencies: { a: "1.2.3", b: "0.0.1-canary.10", c: "2.0.0+b.5" },
    };
    expect(nonExactVersions(manifest)).toEqual([]);
  });

  test("reads nested overrides and npm's '.' self-version", () => {
    const manifest = {
      overrides: {
        "eslint-plugin-sonarjs": { minimatch: "9.0.7" },
        foo: { ".": "^1.0.0", bar: "~2.0.0" },
      },
    };
    expect(nonExactVersions(manifest)).toEqual([
      { section: "overrides", name: "foo", version: "^1.0.0" },
      { section: "overrides", name: "foo > bar", version: "~2.0.0" },
    ]);
  });

  test("skips a package whose version the release writes", () => {
    const manifest = { optionalDependencies: { native: "*", other: "*" } };
    expect(nonExactVersions(manifest, new Set(["native"]))).toEqual([
      { section: "optionalDependencies", name: "other", version: "*" },
    ]);
  });
});

describe("versionsStampedAtRelease", () => {
  test("names every native binary package and nothing else", () => {
    const stamped = versionsStampedAtRelease();
    expect(stamped.has("@konvert7/klint-linux-x64")).toBe(true);
    expect(stamped.has("typescript")).toBe(false);
  });
});

test("every tracked package.json in this repo pins every dependency exactly", () => {
  const root = join(import.meta.dir, "..");
  const stamped = versionsStampedAtRelease();
  const offenders = trackedManifests(root).flatMap((path) =>
    nonExactVersions(JSON.parse(readFileSync(join(root, path), "utf8")), stamped)
  );
  expect(offenders).toEqual([]);
});
