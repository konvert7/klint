#!/usr/bin/env bun
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { stringify as toYaml } from "yaml";
import { KlintConfigSchema } from "../core/config.schema";
import { schemaUrlForVersion } from "../core/schema-version";

const raw = KlintConfigSchema.toJSONSchema() as Record<string, unknown>;

// Set Draft-07 (broad editor compatibility) instead of 2020-12.
delete raw.$schema;

function packageVersion(): string {
  const manifest = readFileSync(resolve(import.meta.dir, "../package.json"), "utf-8");
  try {
    return (JSON.parse(manifest) as { version?: string }).version ?? "0.0.0";
  } catch {
    throw new Error("package.json is not valid JSON — cannot stamp the schema version");
  }
}

const schema = {
  $schema: "http://json-schema.org/draft-07/schema#",
  $id: schemaUrlForVersion(packageVersion()),
  ...raw,
};

const jsonOutPath = resolve(import.meta.dir, "../klint.schema.json");
writeFileSync(jsonOutPath, `${JSON.stringify(schema, null, 2)}\n`);
process.stdout.write(`Generated ${jsonOutPath}\n`);

const yamlOutPath = resolve(import.meta.dir, "../klint.schema.yaml");
writeFileSync(yamlOutPath, toYaml(schema, { lineWidth: 120 }));
process.stdout.write(`Generated ${yamlOutPath}\n`);
