import { z } from "zod";
import { BUILT_IN_PLUGINS } from "../plugins/index";
import { BUILT_IN_RULES } from "../rules/index";
import type { KlintRule, RuleMeta } from "./types";

const builtInRuleNames = Object.keys(BUILT_IN_RULES) as [string, ...string[]];
const builtInPluginNames = Object.keys(BUILT_IN_PLUGINS) as [string, ...string[]];
const pluginRuleNames = [
  ...new Set(Object.values(BUILT_IN_PLUGINS).flatMap((p) => Object.keys(p.rules))),
] as [string, ...string[]];
const allKnownRuleNames = [...new Set([...builtInRuleNames, ...pluginRuleNames])] as [
  string,
  ...string[],
];

/** Look up a rule's implementation across BUILT_IN_RULES and plugin implementations. */
function findRule(name: string): KlintRule | undefined {
  if (BUILT_IN_RULES[name]) return BUILT_IN_RULES[name];
  for (const plugin of Object.values(BUILT_IN_PLUGINS)) {
    if (plugin.implementations[name]) return plugin.implementations[name];
  }
  return undefined;
}

function metaFor(name: string): RuleMeta | undefined {
  return findRule(name)?.meta;
}

const SeveritySchema = z
  .enum(["error", "warn", "off"])
  .describe(
    'Rule severity. "error" exits with code 2; "warn" reports but exits 0; "off" silences.'
  );

const RuleOptionsSchema = z
  .object({
    severity: SeveritySchema.optional(),
    include: z
      .array(z.string())
      .optional()
      .describe(
        'Glob patterns scoping this rule to a subset of files. Prefix with ! to exclude. Example: ["src/hooks/**", "!src/hooks/scripts/**"]'
      ),
  })
  .strict()
  .describe(
    'Rule options object. Omit severity to default to "error". Add include to scope the rule to specific files.'
  );

const RuleValueSchema = z.union([SeveritySchema, RuleOptionsSchema]);

// ─── arch ─────────────────────────────────────────────────────────────

const ArchSeveritySchema = z
  .enum(["error", "warn"])
  .describe(
    'Severity for the arch rule. "error" exits with code 2; "warn" reports but exits 0. (arch rules cannot be "off" — remove the entry instead.)'
  );

const StringOrStringArray = z
  .union([z.string(), z.array(z.string())])
  .describe(
    "Either a single layer name / glob pattern or an array of them. Layer names defined in `arch.layers` are resolved to their glob lists."
  );

const ArchLayersSchema = z
  .record(z.string(), z.array(z.string()))
  .describe(
    "Named architectural zones. Keys are arbitrary layer names; values are glob patterns matching the files in that layer. Other arch rules reference these names instead of repeating globs."
  )
  .meta({
    examples: [
      'layers:\n  core:   ["src/hooks/lib/**", "src/tools/**"]\n  skills: ["assets/skills/**"]\n  dao:    ["src/dao/**"]',
    ],
  });

const ArchImportRuleSchema = z
  .object({
    from: StringOrStringArray.describe(
      "Source — a layer name or glob(s) for the files this rule applies to. Imports originating in `from` are checked against `deny`/`allow`."
    ),
    deny: StringOrStringArray.optional().describe(
      "Layer name(s) or glob(s) the source is not allowed to import from. Mutually exclusive in spirit with `allow` (use one or the other)."
    ),
    allow: StringOrStringArray.optional().describe(
      "Allowlist of layer name(s) / glob(s) the source may import from. Anything not matching `allow` is denied."
    ),
    "type-only": z
      .literal("allow")
      .optional()
      .describe(
        'When set to "allow", `import type` statements that would otherwise be denied are permitted. Useful for type-only references across layers.'
      ),
    message: z
      .string()
      .optional()
      .describe(
        "Custom error message reported when the rule is violated. Shown in the agent's event stream alongside file:line."
      ),
    severity: ArchSeveritySchema.optional(),
  })
  .strict()
  .describe(
    "An import-boundary rule. Restricts which layers a source can or cannot import from."
  )
  .meta({
    examples: [
      'arch:\n  imports:\n    - from: skills\n      deny: core\n      message: "Skills must be self-contained and portable"\n      severity: warn',
      'arch:\n  imports:\n    - from: ["src/dao/**"]\n      allow: ["src/dao/**", "src/prisma/**", "src/types/**"]',
    ],
  });

const ArchForbiddenPatternRuleSchema = z
  .object({
    pattern: z
      .string()
      .describe(
        "Pattern to search for, scanned per line. A literal substring by default (e.g. a console logging call). Prefix with `re:` to match a regular expression instead (e.g. `re:\\bp-\\[\\d+px\\]`). Regexes must use the common JS/RE2 subset — no lookaround or backreferences — so the TS and Rust engines agree. Note: a literal pattern that itself begins with `re:` cannot be expressed; such a value is always read as a regex."
      ),
    in: StringOrStringArray.describe(
      "Layer name(s) or glob(s) the pattern is forbidden in. Files outside the scope are not checked."
    ),
    message: z
      .string()
      .describe(
        "Required error message explaining why the pattern is forbidden. Shown to the agent so it can fix the call site."
      ),
    severity: ArchSeveritySchema.optional(),
  })
  .strict();

const ArchForbiddenJsxRuleSchema = z
  .object({
    "jsx-element": StringOrStringArray.describe(
      "JSX intrinsic element name(s) to forbid (e.g. `button`, `input`, `label`). AST-matched on opening and self-closing tag names — robust to whitespace, attributes, and naming collisions like `<buttonGroup>`."
    ),
    in: StringOrStringArray.describe(
      "Layer name(s) or glob(s) the element is forbidden in. Files outside the scope are not checked."
    ),
    message: z
      .string()
      .describe(
        "Required error message explaining why the raw element is forbidden. Typically points at the design-system replacement (e.g. `Use <Button /> from @/components/ui/button`)."
      ),
    severity: ArchSeveritySchema.optional(),
  })
  .strict();

const ArchForbiddenRuleSchema = z
  .union([ArchForbiddenPatternRuleSchema, ArchForbiddenJsxRuleSchema])
  .describe(
    "A forbidden-pattern rule. Blocks either a literal substring (`pattern:`) or a JSX element (`jsx-element:`) inside specific layers."
  )
  .meta({
    examples: [
      'arch:\n  forbidden:\n    - pattern: "debugLog("\n      in: core\n      message: "Leaks into the agent event stream"',
      'arch:\n  forbidden:\n    - jsx-element: ["button", "input", "label"]\n      in: ["src/app/**/*.tsx", "src/components/**/*.tsx", "!src/components/ui/**"]\n      message: "Use the design-system primitives in @/components/ui/* instead of raw HTML elements."',
    ],
  });

const ArchSingletonPatternRuleSchema = z
  .object({
    pattern: z
      .string()
      .describe(
        "Pattern whose appearance is allowed only at one location, scanned per line. A literal substring by default (e.g. `process.env.API_KEY`). Prefix with `re:` to match a regular expression instead. Regexes must use the common JS/RE2 subset — no lookaround or backreferences — so the TS and Rust engines agree. Note: a literal pattern that itself begins with `re:` cannot be expressed; such a value is always read as a regex."
      ),
    only: z
      .string()
      .describe(
        "The single file path where this pattern is allowed. Every other occurrence becomes a violation."
      ),
    in: StringOrStringArray.optional().describe(
      "Optional scope. Limit the scan to these layers/globs; defaults to all files in `include`."
    ),
    message: z.string(),
    severity: ArchSeveritySchema.optional(),
  })
  .strict();

const ArchSingletonJsxRuleSchema = z
  .object({
    "jsx-element": StringOrStringArray.describe(
      "JSX intrinsic element name(s) allowed only inside the `only` file. AST-matched on tag names."
    ),
    only: z
      .string()
      .describe(
        "The single file path where the element is allowed (typically the design-system primitive that wraps it)."
      ),
    in: StringOrStringArray.optional(),
    message: z.string(),
    severity: ArchSeveritySchema.optional(),
  })
  .strict();

const ArchSingletonRuleSchema = z
  .union([ArchSingletonPatternRuleSchema, ArchSingletonJsxRuleSchema])
  .describe(
    "A singleton-location rule. Pins a pattern or JSX element to exactly one file — the only honest way to enforce a module of record or a design-system primitive."
  )
  .meta({
    examples: [
      'arch:\n  singleton:\n    - pattern: "process.env.API_KEY"\n      only: "src/lib/auth.ts"\n      message: "API key access must funnel through the auth module."',
      'arch:\n  singleton:\n    - jsx-element: "button"\n      only: "src/components/ui/button.tsx"\n      in: ["src/**/*.tsx"]\n      message: "Raw <button> belongs only to the Button primitive."',
    ],
  });

const ArchMaxLinesRuleSchema = z
  .object({
    limit: z
      .number()
      .int()
      .positive()
      .describe(
        "Maximum number of physical lines a file may contain. Counts every line including blanks and comments. A file with more lines than this is a violation."
      ),
    in: StringOrStringArray.describe(
      "Layer name(s) or glob(s) the limit applies to. Files outside the scope are not checked."
    ),
    message: z
      .string()
      .optional()
      .describe(
        "Optional error message. Defaults to a message naming the limit (e.g. 'File exceeds the maximum of 300 lines')."
      ),
    severity: ArchSeveritySchema.optional(),
  })
  .strict()
  .describe(
    "A per-file line-count limit. Caps how long any file in scope may grow — the honest way to keep modules small without a formatter dependency."
  )
  .meta({
    examples: [
      'arch:\n  maxLines:\n    - limit: 300\n      in: src/**\n      message: "Split this module"',
      "arch:\n  maxLines:\n    - limit: 300\n      in: src/**\n    - limit: 600\n      in: tests/**",
    ],
  });

const ArchSchema = z
  .object({
    layers: ArchLayersSchema.optional(),
    imports: z
      .array(ArchImportRuleSchema)
      .optional()
      .describe(
        "Import-boundary rules. The dependency graph your README claims you have."
      ),
    forbidden: z
      .array(ArchForbiddenRuleSchema)
      .optional()
      .describe("Forbidden-pattern rules. Block literal strings scoped to a layer."),
    singleton: z
      .array(ArchSingletonRuleSchema)
      .optional()
      .describe(
        "Singleton-location rules. Pin a pattern to one file; every other touch is a violation."
      ),
    maxLines: z
      .array(ArchMaxLinesRuleSchema)
      .optional()
      .describe(
        "Per-file line-count limits. Cap how long files in a layer may grow, with different ceilings per scope."
      ),
  })
  .strict()
  .describe(
    "Architecture-as-Code constraints — layers, import boundaries, forbidden patterns, singleton locations. Enforced by the arch engine alongside the rule set."
  );

function buildRulesSchema() {
  const shape: Record<string, z.ZodType> = {};
  for (const name of allKnownRuleNames) {
    const meta = metaFor(name);
    let entry: z.ZodType = RuleValueSchema.optional();
    if (meta?.description) entry = entry.describe(meta.description);
    if (meta?.examples?.length) entry = entry.meta({ examples: meta.examples });
    shape[name] = entry;
  }
  return z.object(shape).catchall(RuleValueSchema);
}

export const KlintConfigSchema = z
  .object({
    $schema: z
      .string()
      .optional()
      .describe(
        "JSON Schema reference. Use ./klint.schema.json for local validation or https://klint.dev/schema.json for the published schema."
      ),
    root: z
      .string()
      .optional()
      .describe(
        "Root directory used to resolve include paths and report relative file names. Defaults to the directory containing klint.config.json."
      ),
    include: z
      .array(z.string())
      .optional()
      .describe(
        'Glob patterns selecting which TypeScript files to lint. Prefix with ! to exclude. Defaults to ["."] which lints all .ts files under root. Example: ["src", "klint", "!**/node_modules/**"]'
      ),
    plugins: z
      .array(z.enum(builtInPluginNames))
      .optional()
      .describe(
        'Named rule bundles to enable. Each plugin applies a default set of rules at "error" severity. Individual rules from the bundle can be overridden or silenced via the rules map. Available: "sonar".'
      ),
    rules: buildRulesSchema()
      .optional()
      .describe(
        'Map of rule name → severity or options. Example: { "no-floating-promise": "error", "no-sync-in-async": { "severity": "warn", "include": ["src/hooks/**"] } }.'
      ),
    arch: ArchSchema.optional(),
  })
  .strict()
  .describe(
    "klint configuration. Lives at klint.yaml (or klint.config.json) next to biome.json and knip.json."
  );

/** @lintignore */
export type KlintConfigFile = z.infer<typeof KlintConfigSchema>;
