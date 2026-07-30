import { describe, expect, test } from "bun:test";
import {
  declaredSchemaVersion,
  schemaUrlForVersion,
  schemaVersionAdvisory,
} from "../core/schema-version";

const taggedUrl = (version: string) =>
  `https://raw.githubusercontent.com/konvert7/klint/refs/tags/v${version}/klint.schema.json`;

describe("declaredSchemaVersion", () => {
  test("reads the version out of a tagged schema URL", () => {
    expect(declaredSchemaVersion(taggedUrl("0.29.0"))).toBe("0.29.0");
  });

  test("accepts prerelease tags", () => {
    expect(declaredSchemaVersion(taggedUrl("1.0.0-rc.1"))).toBe("1.0.0-rc.1");
  });

  test("ignores local paths, unversioned URLs, and non-semver tags", () => {
    expect(declaredSchemaVersion("./klint.schema.json")).toBeUndefined();
    expect(declaredSchemaVersion("https://klint.dev/schema.json")).toBeUndefined();
    expect(
      declaredSchemaVersion(
        "https://raw.githubusercontent.com/konvert7/klint/refs/tags/vmain/klint.schema.json"
      )
    ).toBeUndefined();
    expect(declaredSchemaVersion(undefined)).toBeUndefined();
  });

  test("round-trips the URL builder", () => {
    expect(declaredSchemaVersion(schemaUrlForVersion("2.5.1"))).toBe("2.5.1");
  });
});

describe("schemaVersionAdvisory", () => {
  const configText = "include: []\n$schema: x\nrules: {}\n";

  test("warns when the declared version differs from the installed one", () => {
    const [advisory, ...rest] = schemaVersionAdvisory({
      schema: taggedUrl("0.11.2"),
      installed: "0.29.0",
      configFile: "klint.yaml",
      configText,
    });

    expect(rest).toHaveLength(0);
    expect(advisory).toBeDefined();
    expect(advisory?.severity).toBe("warn");
    expect(advisory?.rule).toBe("klint/schema-version");
    expect(advisory?.file).toBe("klint.yaml");
    expect(advisory?.line).toBe(2);
    expect(advisory?.message).toContain("declares schema v0.11.2");
    expect(advisory?.message).toContain("klint 0.29.0 is installed");
    expect(advisory?.message).toContain(schemaUrlForVersion("0.29.0"));
  });

  test("stays silent when the versions match", () => {
    expect(
      schemaVersionAdvisory({
        schema: taggedUrl("0.29.0"),
        installed: "0.29.0",
        configFile: "klint.yaml",
        configText,
      })
    ).toEqual([]);
  });

  test("stays silent for a local schema path", () => {
    expect(
      schemaVersionAdvisory({
        schema: "./klint.schema.json",
        installed: "0.29.0",
        configFile: "klint.yaml",
        configText,
      })
    ).toEqual([]);
  });

  test("stays silent when no schema is declared", () => {
    expect(
      schemaVersionAdvisory({
        schema: undefined,
        installed: "0.29.0",
        configFile: "klint.yaml",
        configText,
      })
    ).toEqual([]);
  });

  test("stays silent for an unreleased build", () => {
    expect(
      schemaVersionAdvisory({
        schema: taggedUrl("0.11.2"),
        installed: "0.0.0",
        configFile: "klint.yaml",
        configText,
      })
    ).toEqual([]);
  });

  test("falls back to line 1 when the schema key is absent from the text", () => {
    const [advisory] = schemaVersionAdvisory({
      schema: taggedUrl("0.11.2"),
      installed: "0.29.0",
      configFile: "klint.config.json",
      configText: "{}",
    });

    expect(advisory?.line).toBe(1);
    expect(advisory?.file).toBe("klint.config.json");
  });
});
