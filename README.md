# klint

Architecture-as-Code linting for TypeScript, Python, and Swift projects.

Biome, oxlint, Ruff, and SwiftLint are excellent at syntax, style, and fast local correctness. klint handles the rules that need project context: layer boundaries, singleton ownership, forbidden patterns, custom repository policies, and type-aware TypeScript checks that should block an AI agent or CI run before code lands.

Use klint when "please follow AGENTS.md" is not strong enough. Put the rule in `klint.yaml`, run `klint`, and make the constraint executable.

## Install

For macOS native CLI usage:

```sh
brew tap konvert7/tap
brew install klint
```

For TypeScript projects using Bun:

```sh
bun add -d @konvert7/klint
```

Add a script:

```json
{
  "scripts": {
    "klint": "klint"
  }
}
```

Then create `klint.yaml` in your project root.

## Quickstart

```yaml
# klint.yaml
include: ["src", "!**/node_modules/**"]

rules:
  no-floating-promise: error
  no-unguarded-json-parse: error
  no-sync-in-async:
    severity: error
    include: ["src/hooks/**"]

arch:
  layers:
    app: ["src/app/**"]
    data: ["src/data/**"]
    ui: ["src/components/**"]

  imports:
    - from: ui
      deny: data
      message: "UI components must go through app services, not data modules"
```

Run it:

```sh
bun run klint
```

For machines and hooks:

```sh
bun run klint -- --json
```

`--json` emits structured violations and exits `2` when errors are present.

```json
{
  "violations": [
    {
      "rule": "arch/imports",
      "file": "src/components/Profile.tsx",
      "line": 3,
      "severity": "error",
      "message": "UI components must go through app services, not data modules",
      "fix": null
    }
  ],
  "summary": { "errors": 1, "warnings": 0 }
}
```

## What Klint Catches

klint is for rules that are too project-specific or context-heavy for a formatter.

| Problem | Example |
|---------|---------|
| Layer boundaries | `src/components/**` must not import from `src/data/**` |
| Singleton ownership | `process.env.API_KEY` may only appear in `src/lib/auth.ts` |
| Forbidden patterns | `console.log(` is blocked inside hook libraries |
| Raw JSX elements | `<button>` is blocked outside `src/components/ui/**` |
| Type-aware mistakes | Promise-returning calls cannot be silently discarded |
| Repository policy | Custom `klint.rules.ts` rules run with the same severity/config system |

Rules give agents room to move quickly because the unsafe moves are blocked structurally.

## CLI

```sh
klint [--config <dir>] [--rules <file>] [--engine ts|rust|compare|auto] [--fix] [--json]
klint install-skill [--agents <list>] [--symlink | --copy]
```

| Flag | Description |
|------|-------------|
| `--config <dir>` | Directory containing `klint.yaml` or `klint.config.json` (default: cwd) |
| `--rules <file>` | Path to custom rules file (default: `<configDir>/klint.rules.ts` when present) |
| `--engine <name>` | Engine mode: `ts` (default), `rust`, `compare`, or `auto` |
| `--fix` | Apply auto-fixes for fixable violations in-place |
| `--json` | Emit structured JSON to stdout for agents and CI |

`klint install-skill` installs the bundled rule-authoring skill for agent environments:

```sh
klint install-skill --agents claude,opencode,cursor,codex --copy
```

## Engines

klint currently ships a TypeScript engine and an experimental Rust engine. The Rust path is built for portable architecture checks and syntax-local rules; type-aware checks stay in TypeScript for now.

Maintainer migration notes live in [`docs/rust-engine.md`](docs/rust-engine.md).

| Engine | What it does | When to use it |
|--------|--------------|----------------|
| `ts` | Runs the full TypeScript implementation. This is the default. | Maximum compatibility |
| `rust` | Runs only rules supported by the Rust engine. Unsupported TS-only rules are rejected. | Strict Rust smoke tests and native-engine debugging |
| `compare` | Runs TS and Rust on the same supported config and fails if JSON output differs. Requires `--json`. | Parity verification while porting rules |
| `auto` | Runs Rust-supported rules in Rust, TS-only rules and unsupported plugin defaults in TypeScript, then merges output. | Recommended experimental migration path |

Examples:

```sh
klint --engine auto
klint --engine rust --json     # strict Rust-only run; rejects unsupported rules
klint --engine compare --json  # parity check; rejects unsupported rules
```

Rust currently supports:

- `arch` rules: imports, forbidden patterns, singleton locations
- `no-unguarded-json-parse`
- `no-sync-in-async`
- `no-nested-template-literals`
- `no-consecutive-array-push`
- `no-string-match`
- `sonar/no-single-char-class`
- `sonar/prefer-at`
- `sonar/prefer-nullish-coalescing-assign`
- `sonar/prefer-string-replaceall`
- `sonar/prefer-string-raw`
- `sonar/prefer-string-raw-regexp`

These rules remain TypeScript-owned because they need TypeScript semantic information:

- `no-floating-promise`
- `no-misused-promises`
- `no-date-equality`
- `no-optional-chain-on-non-nullable`
- `no-object-in-template`

## Configuration

`klint.yaml` lives at your project root alongside tools like `biome.json`, `tsconfig.json`, and `knip.json`.

```yaml
# yaml-language-server: $schema=./klint.schema.yaml

include: ["src", "!**/node_modules/**"]
plugins: [sonar]

rules:
  no-unguarded-json-parse: error
  no-floating-promise: error
  no-misused-promises: error
  sonar/prefer-string-replaceall: warn
  my-custom-rule:
    severity: error
    include: ["src/server/**"]

arch:
  layers:
    server: ["src/server/**"]
    client: ["src/client/**"]

  imports:
    - from: client
      deny: server
      message: "Client code must not import server-only modules"
```

`include` selects source files to lint. Patterns support `**` globs and `!` negation.

`plugins` enables bundled rule groups. Today, `sonar` provides focused code-quality rules.

`rules` maps rule names to `"error"`, `"warn"`, `"off"`, or an options object with `severity` and/or `include`.

`arch` declares architecture constraints without writing a custom rule.

`klint.config.json` is still supported for backwards compatibility.

## Architecture as Code

The `arch:` section turns project boundaries into executable policy.

### Layers

Define named file groups once, then reference them from import rules:

```yaml
arch:
  layers:
    core: ["src/core/**", "src/lib/**"]
    features: ["src/features/**"]
    adapters: ["src/adapters/**"]
```

### Import Boundaries

Use `deny` to block one layer from importing another:

```yaml
arch:
  imports:
    - from: features
      deny: adapters
      message: "Features must use core ports, not adapter implementations"
      severity: error
```

Use `allow` for whitelist mode. Anything not listed is denied:

```yaml
arch:
  imports:
    - from: ["src/data/**"]
      allow: ["src/data/**", "src/db/**", "src/types/**"]
      message: "Data modules may only depend on data, db, and types"
```

Allow type-only imports when runtime imports are forbidden:

```yaml
arch:
  imports:
    - from: core
      deny: adapters
      type-only: allow
      message: "Core may reference adapter types, but not adapter values"
```

### Forbidden Patterns

Block literal string patterns inside a scoped layer:

```yaml
arch:
  forbidden:
    - pattern: "console.log("
      in: ["src/hooks/**"]
      message: "Hook output must go through the structured output API"

    - pattern: "process.exit("
      in: ["src/lib/**"]
      message: "Libraries should return or throw, not terminate the process"
```

Block raw JSX/HTML elements with `jsx-element` to push consumers onto design-system primitives. It is AST-matched on intrinsic (lowercase) tag names, so it is robust to whitespace, attributes, and lookalikes like `<buttonGroup>`:

```yaml
arch:
  forbidden:
    - jsx-element: ["button", "input", "label"]
      in: ["src/app/**/*.tsx", "src/components/**/*.tsx", "!src/components/ui/**"]
      message: "Use the design-system primitives in @/components/ui/* instead of raw HTML elements"
```

`jsx-element` matches intrinsic HTML elements only — not custom components like `<Button>`. Use `arch.imports` to restrict those.

### Singleton Locations

Enforce that a pattern appears only in one designated file:

```yaml
arch:
  singleton:
    - pattern: "process.env.API_KEY"
      only: "src/lib/auth.ts"
      in: ["src/**"]
      message: "Use the auth module"
```

`singleton` accepts `jsx-element` too — pin a raw element to the one primitive file that is allowed to render it:

```yaml
arch:
  singleton:
    - jsx-element: "button"
      only: "src/components/ui/button.tsx"
      in: ["src/**/*.tsx"]
      message: "Raw <button> belongs only to the Button primitive"
```

### Python Architecture Checks

The Rust engine can apply architecture rules to Python projects:

```yaml
include: ["src"]

rules: {}

arch:
  layers:
    jobs: ["src/app/jobs/**"]
    lib: ["src/app/lib/**"]
    config: ["src/app/config.py"]

  imports:
    - from: jobs
      deny: lib
      message: "Jobs must use service APIs, not import lib directly"

  forbidden:
    - pattern: "print("
      in: jobs
      message: "Use the logger"

  singleton:
    - pattern: "os.environ[\"API_KEY\"]"
      only: "src/app/config.py"
      in: ["src/**"]
      message: "Use app config"
```

Python import checks support relative imports such as
`from ..lib.auth import load_key` and project-resolvable absolute imports such
as `from app.lib.auth import load_key`. External packages that do not resolve to
project files, such as `requests`, are ignored. This is architecture
enforcement, not full packaging or virtual-environment analysis.

### Swift Architecture Checks

The Rust engine can apply architecture rules to Swift projects:

```yaml
include: ["Sources"]

rules: {}

arch:
  layers:
    ui: ["Sources/App/UI/**"]
    core: ["Sources/App/Core/**"]
    config: ["Sources/App/Config/**"]

  imports:
    - from: ui
      deny: core
      message: "UI must use app services, not import Core directly"

  forbidden:
    - pattern: "URLSession.shared"
      in: ui
      message: "Use the networking client"

  singleton:
    - pattern: "ProcessInfo.processInfo.environment[\"API_KEY\"]"
      only: "Sources/App/Config/AppConfig.swift"
      in: ["Sources/**"]
      message: "Use AppConfig"
```

Swift import checks parse declarations such as `import Core`,
`@_exported import Core`, and `import struct Models.User`. Imported modules
resolve against discovered project Swift directories and file stems. System or
package modules that are not present in the project, such as `Foundation`, are
ignored. This is architecture enforcement, not full SwiftPM or Xcode build graph
analysis.

## Built-in Rules

| Rule | Type-aware | Description |
|------|------------|-------------|
| `no-unguarded-json-parse` | No | `JSON.parse()` called outside a try/catch |
| `no-sync-in-async` | No | Sync filesystem calls inside async functions |
| `no-floating-promise` | Yes | Promise-returning call whose result is discarded |
| `no-misused-promises` | Yes | Async function passed where a sync callback is expected |
| `no-async-predicate` | Yes | Async predicate passed to array filtering/search methods |
| `no-date-equality` | Yes | Date values compared by object identity |
| `no-optional-chain-on-non-nullable` | Yes | Optional chaining on values TypeScript knows are non-nullable |
| `no-object-in-template` | Yes | Object values interpolated into template strings |
| `no-nested-template-literals` | No | Template literals nested inside template literals |
| `no-consecutive-array-push` | No | Multiple consecutive `push()` calls that should be grouped |
| `no-string-match` | No | `String#match()` usage better expressed with clearer APIs |

## Plugins

Enable plugin rules with `plugins: [sonar]`, then override individual severities in `rules` when needed.

| Rule | Description |
|------|-------------|
| `sonar/prefer-string-replaceall` | Prefer `replaceAll()` when replacing every string occurrence |
| `sonar/prefer-string-raw-regexp` | Prefer `String.raw` for regex strings where escaping is easy to misread |
| `sonar/prefer-string-raw` | Prefer `String.raw` for escape-heavy strings |
| `sonar/prefer-nullish-coalescing-assign` | Prefer `??=` for nullish assignment patterns |
| `sonar/no-single-char-class` | Avoid regex character classes with a single character |
| `sonar/prefer-at` | Prefer `.at()` for relative index access |

## Agent and CI Integration

Agent hooks should run klint with `--json` so violations are machine-readable.

```ts
// .agents/hooks/klint.ts
import { runHook } from "./run-hook";

const exitCode = runHook(["bun", "run", "klint", "--", "--json"]);
process.exit(exitCode);
```

In CI, run the same command you use locally:

```sh
bun install
bun run klint
```

The important contract is simple: klint exits `0` when the project is clean, `2` when blocking errors exist, and can emit JSON for agents that need structured feedback.

## Custom Rules

Create `klint.rules.ts` next to `klint.yaml` and export a `Record<string, KlintRule>` as default. Each key is the rule name.

```ts
import { relative } from "node:path";
import type { KlintRule } from "@konvert7/klint/core/types";

const noForbiddenPattern: KlintRule = {
  check({ files, root, fileContents }, violations) {
    for (const file of files) {
      const lines = (fileContents.get(file) ?? "").split("\n");
      for (let i = 0; i < lines.length; i++) {
        if (/forbidden-pattern/.test(lines[i])) {
          violations.push({
            file: relative(root, file),
            line: i + 1,
            message: "Explain what is wrong and how to fix it.",
          });
        }
      }
    }
  },
};

export default {
  "no-forbidden-pattern": noForbiddenPattern,
};
```

All exported custom rules run at `"error"` severity by default. Override severity or scope through `rules`:

```yaml
rules:
  no-forbidden-pattern:
    severity: warn
    include: ["src/server/**"]
```

No separate registration step is required.

### Auto-fix Support

Add a `fix` field to a violation to make it auto-fixable with `--fix`. Fixes replace a line range with new text:

```ts
violations.push({
  file: relative(root, file),
  line: i + 1,
  message: "Use foo() instead of bar().",
  fix: {
    startLine: i + 1,
    endLine: i + 1,
    replacement: lines[i].replace("bar()", "foo()"),
  },
});
```

### Type-aware Rules

For rules that need the TypeScript AST, use `walkAst`:

```ts
import ts from "typescript";
import { walkAst } from "@konvert7/klint/core/ast";
import type { KlintRule } from "@konvert7/klint/core/types";

const noSpecificCall: KlintRule = {
  check({ files, fileContents }, violations) {
    for (const file of files) {
      const content = fileContents.get(file) ?? "";
      walkAst(file, content, (node) => {
        if (ts.isCallExpression(node)) {
          // Inspect node with the TypeScript AST.
        }
      });
    }
  },
};
```

## How It Fits

klint is not a replacement for Biome, oxlint, ESLint, knip, or TypeScript itself.

- Use TypeScript for type correctness.
- Use Biome or oxlint for fast syntax and style checks.
- Use knip for unused files and exports.
- Use klint for architecture, agent constraints, project policy, and custom type-aware checks.

The goal is a small, sharp gate: the rules your project cannot afford to leave as prose.
