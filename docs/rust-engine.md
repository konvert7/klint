# Rust Engine Status

This document tracks the Rust engine migration for maintainers. The README explains the user-facing engine modes; this file records what is supported, what remains TypeScript-owned, and which boundaries should not be blurred.

## Current Model

klint has two implementations behind one CLI:

- TypeScript engine: full compatibility, type-aware rules, plugins, custom rules, and fixes.
- Rust engine: portable architecture checks and syntax-local built-in rules.

The package entrypoint is still `cli.ts`. It resolves the native Rust binary, validates engine support, and renders consistent text or JSON output.

## Engine Modes

| Engine | Status | Contract |
|--------|--------|----------|
| `ts` | Stable default | Runs the full TypeScript implementation. |
| `rust` | Experimental strict mode | Runs only Rust-supported config. Rejects unsupported TS-only rules, plugins, custom rules, `--rules`, and `--fix`. |
| `compare` | Migration guard | Runs TS and Rust on the same supported config and fails on JSON mismatch. Requires `--json`. |
| `auto` | Experimental dogfood mode | Runs Rust-supported rules in Rust, TS-owned rules in TypeScript, and merges output. |

## Rust-Supported Surface

Architecture rules:

| Area | Notes |
|------|-------|
| `arch.imports` | Includes static imports, dynamic imports, TS path aliases, allow/deny mode, and type-only allowance. |
| `arch.forbidden` | Supports literal pattern checks and JSX element checks. |
| `arch.singleton` | Supports literal pattern checks and JSX element checks. |

Top-level rules:

| Rule | Why it is Rust-portable |
|------|-------------------------|
| `no-unguarded-json-parse` | Syntax-only call detection plus `try_statement` ancestor tracking. |
| `no-sync-in-async` | Syntax-only sync-call detection with nearest async function tracking. |
| `no-nested-template-literals` | Syntax-only template-substitution traversal. |
| `no-consecutive-array-push` | Syntax-only statement-run detection. |
| `no-string-match` | Syntax-only call detection with regex literal flag handling. |

Every Rust-supported rule should have:

- Rust syntax scanner coverage in `crates/klint-rs/src/syntax.rs`.
- Rust rule dispatch in `crates/klint-rs/src/rules.rs`.
- CLI compare coverage in `tests/rust-engine-cli.test.ts`.
- Golden parity coverage in `tests/golden/rule-cases.json`.

## TypeScript-Owned Rules

These rules must stay TypeScript-owned unless klint gains a real Rust semantic layer:

| Rule | Why tree-sitter alone is not enough |
|------|-------------------------------------|
| `no-floating-promise` | Needs return-type information to know whether a call is Promise-like. |
| `no-misused-promises` | Needs resolved call signatures and callback return types. |
| `no-async-predicate` | Needs receiver type information to avoid flagging custom `.filter()`/`.some()` methods. |
| `no-date-equality` | Needs static type information to know both operands are Date-like. |
| `no-optional-chain-on-non-nullable` | Needs strict-nullability type information. |
| `no-object-in-template` | Needs symbol/type analysis to distinguish primitives, safe builtins, custom `toString()`, and plain objects. |

Do not port these as tree-sitter approximations. A false Rust port would make `compare` and `auto` look safer while silently changing rule meaning.

## Custom Rules And Plugins

Custom rules and plugin rules are TypeScript-owned in `auto` mode. The Rust engine does not load `klint.rules.ts` or plugin implementations.

Strict `rust` and `compare` modes reject custom rule files, plugins, and unsupported top-level rules so they cannot silently skip behavior.

## Next Decisions

Near-term:

- Keep `auto` as the repo dogfood mode.
- Keep the published default as `ts` until there is enough release history.
- Add new Rust rules only when they are syntax-local or have a proven semantic source.

Open architectural question:

- Keep hybrid mode permanently, or research a Rust semantic layer for TypeScript.

The current default answer is hybrid mode. Tree-sitter is excellent for syntax and structure; it is not a TypeScript type checker.
