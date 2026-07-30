import type { Violation } from "./types";

const SCHEMA_VERSION_RULE = "klint/schema-version";

const TAGGED_SCHEMA_URL = /\/refs\/tags\/v(\d+\.\d+\.\d+(?:-[0-9A-Za-z.]+)?)\//;
const UNRELEASED = "0.0.0";

export function schemaUrlForVersion(version: string): string {
  return `https://raw.githubusercontent.com/konvert7/klint/refs/tags/v${version}/klint.schema.json`;
}

export function declaredSchemaVersion(schema: string | undefined): string | undefined {
  return schema?.match(TAGGED_SCHEMA_URL)?.[1];
}

export function schemaVersionAdvisory({
  schema,
  installed,
  configFile,
  configText,
}: {
  schema: string | undefined;
  installed: string;
  configFile: string;
  configText: string;
}): Array<Omit<Violation, "fix">> {
  const declared = declaredSchemaVersion(schema);
  if (declared === undefined || declared === installed || installed === UNRELEASED) {
    return [];
  }

  return [
    {
      file: configFile,
      line: schemaDeclarationLine(configText),
      rule: SCHEMA_VERSION_RULE,
      severity: "warn",
      message: `config declares schema v${declared} but klint ${installed} is installed — update $schema to ${schemaUrlForVersion(installed)}`,
    },
  ];
}

function schemaDeclarationLine(configText: string): number {
  const index = configText.split("\n").findIndex((line) => line.includes("$schema"));
  return index === -1 ? 1 : index + 1;
}
