# Rust Engine Status

This document tracks the Rust engine migration for maintainers. The README explains the user-facing engine modes; this file records what is supported, what remains TypeScript-owned, and which boundaries should not be blurred.

## Current Model

klint has two implementations behind one CLI:

- TypeScript engine: full compatibility, type-aware rules, plugins, custom rules, and fixes.
- Rust engine: portable architecture checks and syntax-local built-in rules,
  including supported bundled plugin defaults.

The package entrypoint is still `cli.ts`. It resolves the native Rust binary, validates engine support, and renders consistent text or JSON output.

## Engine Modes

| Engine | Status | Contract |
|--------|--------|----------|
| `ts` | Stable default | Runs the full TypeScript implementation. |
| `rust` | Experimental strict mode | Runs only Rust-supported config. Rejects unsupported rules, custom rules, `--rules`, and `--fix`. |
| `compare` | Migration guard | Runs TS and Rust on the same supported config and fails on JSON mismatch. Requires `--json`. |
| `auto` | Experimental dogfood mode | Runs Rust-supported rules and supported plugin defaults in Rust, TS-owned rules in TypeScript, and merges output. |

## Rust-Supported Surface

Architecture rules:

| Area | Notes |
|------|-------|
| `arch.imports` | Supports TypeScript/JavaScript static imports, dynamic imports, re-export specifiers (`export … from`), TS path aliases, allow/deny mode, `deny-packages` for npm and `node:` specifiers (matching the TS engine), and type-only allowance. Supports Python relative imports, resolvable absolute project imports, every target of a multi-target statement, `importlib.import_module`/`__import__` dynamic imports, `if TYPE_CHECKING:` blocks as type-only allowance, and `deny-packages` against pip packages and stdlib modules matched per dotted segment; unresolved Python package imports are otherwise ignored. Supports Swift `import_declaration` nodes — plain, attributed (`@_exported`, `@testable`), keyword-qualified (`import struct Module.Type`), and submodule paths — when the module resolves to discovered project Swift files; imports written inside comments produce no record, `deny-packages` reaches unresolved Swift modules such as system frameworks at module granularity, and otherwise unresolved system/package imports are ignored. Supports Rust `use` declarations with every target of a braced list checked separately, aliases and wildcards resolved to their path, `pub use` re-exports, path-shaped `crate::`/`self::`/`super::` module resolution against the directory tree, and `deny-packages` against external crates matched per `::` segment. |
| `arch.forbidden` | Supports literal pattern checks for TypeScript/JavaScript, Python, Swift, and Rust files. JSX element checks are TypeScript/JavaScript only. |
| `arch.singleton` | Supports literal pattern checks for TypeScript/JavaScript, Python, Swift, and Rust files. JSX element checks are TypeScript/JavaScript only. |

Top-level rules:

| Rule | Why it is Rust-portable |
|------|-------------------------|
| `no-unguarded-json-parse` | Syntax-only call detection plus `try_statement` ancestor tracking. |
| `no-sync-in-async` | Syntax-only sync-call detection with nearest async function tracking. |
| `no-nested-template-literals` | Syntax-only template-substitution traversal. |
| `no-consecutive-array-push` | Syntax-only statement-run detection. |
| `no-string-match` | Syntax-only call detection with regex literal flag handling. |

Plugin rules:

| Rule | Why it is Rust-portable |
|------|-------------------------|
| `sonar/no-single-char-class` | Syntax-only regex literal parsing and character-class rewrite. |
| `sonar/prefer-at` | Syntax-only negative index access rewrite matching the TypeScript rule contract. |
| `sonar/prefer-nullish-coalescing-assign` | Syntax-only rewrite for explicit nullish assignment guards, excluding unsafe falsy checks. |
| `sonar/prefer-string-replaceall` | Syntax-only `.replace(/literal/g, value)` rewrite with plain-regex filtering. |
| `sonar/prefer-string-raw` | Syntax-only string literal rewrite for escaped-backslash strings that are safe as raw templates. |
| `sonar/prefer-string-raw-regexp` | Syntax-only `new RegExp(template)` rewrite for template literals with double backslashes. |

Every Rust-supported rule should have:

- Rust syntax scanner coverage in `crates/klint-rs/src/syntax/rules/<rule>.rs`.
- Rust rule dispatch in `crates/klint-rs/src/rules/mod.rs`, with the rule body in
  `builtin.rs` or `sonar.rs`.
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

## Python Support

Python support starts at the architecture layer. The Rust engine discovers `.py`
files and applies language-neutral `arch.forbidden` and `arch.singleton` pattern
rules to them. `arch.imports` supports Python relative imports such as
`from ..lib.auth import load_key` and absolute project imports such as
`from app.lib.auth import load_key`. Every target of a multi-target statement
produces its own record, so `import json, app.lib.auth` and
`from . import helper, sibling` are checked per target. Dynamic imports are read
off the AST as well: `importlib.import_module("…")` and `__import__("…")`,
including `import importlib as il` and `from importlib import import_module as
load` bindings — a call to a same-named method that is not bound to `importlib`
is not treated as an import. Absolute imports resolve against the project root
and direct child directories containing Python files, checking `<module>.py`,
`<module>/__init__.py`, and — for PEP 420 namespace packages — a `<module>/`
directory that the scan found `.py` files under. Imports inside an
`if TYPE_CHECKING:` block are marked type-only, so `type-only: allow` exempts
them exactly as it exempts `import type` in TypeScript; the `else`/`elif`
branches of that guard remain runtime imports. Unresolved package imports such
as `import requests` are ignored. TypeScript/Sonar syntax rules are still restricted to
TypeScript/JavaScript-like files.

PyPI packaging is a later distribution step. Land it after the Rust engine has
the Python behavior worth shipping and the package shape is decided.

## Swift Support

Swift support starts at the same architecture-pattern layer as Python did. The
Rust engine discovers `.swift` files and applies language-neutral
`arch.forbidden` and `arch.singleton` literal pattern rules to them.

`.swift` files parse with `tree-sitter-swift`, so Swift shares the same
`TreeCache` path as TypeScript and Python. `arch.imports` walks
`import_declaration` nodes: the module is the first `simple_identifier` under
the declaration's `identifier` child, which covers `import Module`,
`@_exported import Module`, `@testable import Module`,
`import struct Module.Type`, and `import Module.Submodule` in one shape.
Attributes live in a sibling `modifiers` node and carry no module of their own.
An `import` inside a `//` or `/* */` comment produces no record, nesting
included. Imported modules resolve against discovered project Swift directories
and file stems. This is intended for architecture boundaries, not complete
SwiftPM or Xcode build graph analysis. Unresolved system or package modules such
as `Foundation` are ignored.

The Swift grammar names block comments `multiline_comment` rather than
`comment`, so the comment scanner matches both kinds; `arch.maxCommentDensity`
and `arch.maxCommentBlock` treat `///` as a doc-comment in Swift, where
TypeScript reserves triple slashes for `/// <reference>` directives.

`deny-packages` splits Swift specifiers on dots. Because the recorded specifier
is the module — the first `simple_identifier` — matching is at module
granularity, which covers the intended target of system frameworks such as
`Foundation` and `UIKit`.

Swift cases live in `tests/rust-engine-cli.test.ts` and in Rust unit tests, never
in the golden harness — the TypeScript engine cannot parse Swift, so no golden
case can run through both engines.

## Rust Support

`.rs` files parse with `tree-sitter-rust` through the same `TreeCache` as every
other language. `arch.imports` walks `use_declaration` nodes and flattens the
`use` tree into one record per imported target, so a braced list multiplies its
prefix across every branch: `use a::{b::{c, d}, e as f}` records `a::b::c`,
`a::b::d`, and `a::e`. `use_as_clause` and `use_wildcard` contribute the path
they wrap, and a bare `self` inside a list names its own prefix. All records
from one declaration carry the declaration's line. `pub use` re-exports parse to
the same node shape as plain `use`.

Module resolution is path-shaped rather than a Cargo build graph. `crate::`
anchors at the innermost directory holding `lib.rs` or `main.rs`, `self::` at
the current file's module directory, and `super::` at its parent, with leading
`super::` segments applied repeatedly. From that base, klint takes the longest
prefix of the remaining segments that resolves to a real `foo.rs` or
`foo/mod.rs` and treats the rest as an item inside that module. Paths that do
not start with one of those three keywords name an external crate, resolve to no
file, and are therefore invisible to `deny`/`allow` — `deny-packages` is the
rule that reaches them. `#[path]` attributes and `mod` declarations that rename
a module are not followed, so an unconventional layout produces false negatives
rather than false positives.

`deny-packages` splits Rust specifiers on `::`, which is the first separator
that is not a single character; `package_separator` returns `&'static str` for
that reason.

The Rust grammar names comments `line_comment` and `block_comment`, so the
comment scanner matches four node kinds across the supported languages.
Doc-comments are detected from the tree rather than the text: a comment is a doc
when it holds an `outer_doc_comment_marker` or `inner_doc_comment_marker` child,
which covers `///` and `//!` without misreading a `////` separator line. Rust
doc-comment nodes extend past their last character to swallow the trailing
newline, so `end_line` comes from the comment text rather than the node's end
row.

Rust cases live in `tests/rust-engine-cli.test.ts` and in Rust unit tests, never
in the golden harness.

## Custom Rules And Plugins

Custom rules are TypeScript-owned in `auto` mode. The Rust engine does not load `klint.rules.ts`.

Plugin defaults are expanded by the TypeScript CLI wrapper before Rust receives a
config. The bundled `sonar` plugin is supported in `rust`, `compare`, and `auto`
because every current Sonar rule is native-backed. Unknown plugins and
unsupported active plugin rules still fail before Rust runs, so they cannot
silently skip behavior.

## Next Decisions

Near-term:

- Keep `auto` as the repo dogfood mode.
- Keep the published default as `ts` until there is enough release history.
- Add new Rust rules only when they are syntax-local or have a proven semantic source.

Open architectural question:

- Keep hybrid mode permanently, or research a Rust semantic layer for TypeScript.

The current default answer is hybrid mode. Tree-sitter is excellent for syntax and structure; it is not a TypeScript type checker.
