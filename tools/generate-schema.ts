#!/usr/bin/env bun
import { writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { stringify as toYaml } from "yaml";
import { KlintConfigSchema } from "../core/config.schema";

const raw = KlintConfigSchema.toJSONSchema() as Record<string, unknown>;

// Set Draft-07 (broad editor compatibility) instead of 2020-12.
delete raw.$schema;

const schema = {
  $schema: "http://json-schema.org/draft-07/schema#",
  $id: "https://klint.dev/schema.json",
  ...raw,
};

const jsonOutPath = resolve(import.meta.dir, "../klint.schema.json");
writeFileSync(jsonOutPath, `${JSON.stringify(schema, null, 2)}\n`);
process.stdout.write(`Generated ${jsonOutPath}\n`);

const yamlOutPath = resolve(import.meta.dir, "../klint.schema.yaml");
writeFileSync(yamlOutPath, toYaml(schema, { lineWidth: 120 }));
process.stdout.write(`Generated ${yamlOutPath}\n`);
