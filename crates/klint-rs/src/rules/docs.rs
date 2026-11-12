use schemars::{Schema, SchemaGenerator};

pub(crate) type RuleSchema = fn(&mut SchemaGenerator) -> Schema;

pub(crate) struct RuleDoc {
    pub name: &'static str,
    pub description: &'static str,
    pub examples: &'static [&'static str],
    pub options_schema: Option<RuleSchema>,
}

pub(crate) fn v1_options_schema(name: &str) -> Option<RuleSchema> {
    match name {
        "cognitive-complexity" => Some(super::complexity::cognitive_complexity_v1_schema),
        "function-length" => Some(super::function_length::function_length_v1_schema),
        _ => None,
    }
}

pub(crate) const RULE_DOCS: &[RuleDoc] = &[
    RuleDoc {
        name: "no-unguarded-json-parse",
        description: "Flags `JSON.parse()` calls that aren't wrapped in try/catch — malformed input crashes the process.",
        examples: &["no-unguarded-json-parse: error"],
        options_schema: None,
    },
    RuleDoc {
        name: "no-sync-in-async",
        description: "Flags synchronous I/O (`readFileSync`, `execSync`, etc.) inside async functions — blocks the event loop when an async equivalent is available.",
        examples: &[
            "no-sync-in-async: error",
            "no-sync-in-async:\n  severity: warn\n  include: [\"src/server/**\"]",
        ],
        options_schema: None,
    },
    RuleDoc {
        name: "no-async-predicate",
        description: "Flags `async` functions passed to `.filter`/`.some`/`.every`/`.find`/`.findIndex`. Array methods coerce the returned promise to truthy, so every element appears to match.",
        examples: &["no-async-predicate: error"],
        options_schema: None,
    },
    RuleDoc {
        name: "no-nested-template-literals",
        description: "Flags template literals nested inside template literals — almost always unreadable; extract to a variable.",
        examples: &["no-nested-template-literals: warn"],
        options_schema: None,
    },
    RuleDoc {
        name: "no-consecutive-array-push",
        description: "Flags consecutive `arr.push(x); arr.push(y);` calls — combine into `arr.push(x, y)` or `arr.push(...items)`.",
        examples: &["no-consecutive-array-push: warn"],
        options_schema: None,
    },
    RuleDoc {
        name: "no-string-match",
        description: "Flags `String#match` — prefer `.matchAll()` for iteration or `.test()` for boolean checks. `.match()` returns different shapes depending on the `/g` flag.",
        examples: &["no-string-match: error"],
        options_schema: None,
    },
    RuleDoc {
        name: "sonar/prefer-string-replaceall",
        description: "Flags `.replace(/x/g, y)` on strings — `.replaceAll('x', y)` is clearer and faster for all-occurrence replacements.",
        examples: &["sonar/prefer-string-replaceall: error"],
        options_schema: None,
    },
    RuleDoc {
        name: "sonar/prefer-string-raw-regexp",
        description: "Flags regex literals built from escape-heavy string concatenation — `String.raw` drops the double backslashes.",
        examples: &["sonar/prefer-string-raw-regexp: warn"],
        options_schema: None,
    },
    RuleDoc {
        name: "sonar/prefer-string-raw",
        description: "Flags string literals with multiple backslash escapes — `String.raw` is more readable (Windows paths, regex source, etc.).",
        examples: &["sonar/prefer-string-raw: warn"],
        options_schema: None,
    },
    RuleDoc {
        name: "sonar/prefer-nullish-coalescing-assign",
        description: "Flags explicit nullish assignment guards — `x ??= y` is the dedicated nullish assignment operator.",
        examples: &["sonar/prefer-nullish-coalescing-assign: warn"],
        options_schema: None,
    },
    RuleDoc {
        name: "sonar/no-single-char-class",
        description: "Flags single-character character classes in regex (e.g. `/[a]/`) — drop the brackets; the regex matches the same.",
        examples: &["sonar/no-single-char-class: warn"],
        options_schema: None,
    },
    RuleDoc {
        name: "sonar/prefer-at",
        description: "Flags negative-index access via `arr[arr.length - 1]` — `.at(-1)` reads cleaner for last-element access.",
        examples: &["sonar/prefer-at: warn"],
        options_schema: None,
    },
    RuleDoc {
        name: "blank-line-before-control-flow",
        description: "Requires a blank line above a control-flow statement (if, for, while, switch/match, try, return, guard, loop, with) that follows another statement in the same block. Works on TypeScript/JavaScript, Python, Swift, Rust and C#; `klint --fix` inserts the line.",
        examples: &[
            "blank-line-before-control-flow: error",
            "blank-line-before-control-flow:\n  severity: error\n  include: [\"src/**\", \"!src/generated/**\"]\n  statements: [if, for, while, switch, try, return, guard, loop, with]\n  allowSetup: true",
        ],
        options_schema: Some(super::blank_lines::before_schema),
    },
    RuleDoc {
        name: "blank-line-after-block",
        description: "Requires a blank line below a block statement (if, for, while, switch/match, try, guard, loop, with) when another statement follows it in the same block, like C# IDE2003. Works on TypeScript/JavaScript, Python, Swift, Rust and C#; `klint --fix` inserts the line.",
        examples: &[
            "blank-line-after-block: warn",
            "blank-line-after-block:\n  severity: warn\n  include: [\"src/**\"]\n  statements: [if, for, while, switch, try, guard, loop, with]",
        ],
        options_schema: Some(super::blank_lines::after_schema),
    },
    RuleDoc {
        name: "cognitive-complexity",
        description: "Reports every function whose cognitive complexity is above a limit, scored like `eslint-plugin-sonarjs` 3.0.5's `cognitive-complexity` (SonarSource rule S3776). Set `warn` and/or `error` limits (bare `error`/`warn` uses 15); a function gets one diagnostic, at the highest level it exceeds. A list of `{warn, error, include}` entries sets limits per glob, and the last entry whose include matches a file wins. Works on TypeScript/JavaScript, Python, Swift, Rust and C#. Opt-in; no autofix.",
        examples: &[
            "cognitive-complexity: error",
            "cognitive-complexity: {warn: 10, error: 20}",
            "cognitive-complexity:\n  - {error: 15}\n  - {warn: 10, error: 20, include: [\"scripts/**\"]}",
        ],
        options_schema: Some(super::complexity::cognitive_complexity_schema),
    },
    RuleDoc {
        name: "function-length",
        description: "Reports every function whose body has more lines than a limit, counted like Biome 2.6.5's `noExcessiveLinesPerFunction`: brace lines do not count, comment and blank lines do (blank lines not with `skipBlankLines`), and a nested function's lines count toward its parent. Set `warn` and/or `error` limits (bare `error`/`warn` uses 50); a function gets one diagnostic, at the highest level it exceeds. A list of `{warn, error, include, skipBlankLines}` entries sets limits per glob, and the last entry whose include matches a file wins. Getters and setters are not checked. Works on TypeScript/JavaScript, Python, Swift, Rust and C#. Opt-in; no autofix.",
        examples: &[
            "function-length: error",
            "function-length: {warn: 60, error: 80, skipBlankLines: true}",
            "function-length:\n  - {error: 50}\n  - {warn: 60, error: 80, include: [\"scripts/**\"]}",
        ],
        options_schema: Some(super::function_length::function_length_schema),
    },
    RuleDoc {
        name: "no-date-equality",
        description: "Experimental, opt-in: list it under rules.experimental. Flags ==, ===, != and !== between two values the same file proves are Date objects (a new Date(...) or a const initialised with one) — these compare identity, not time. Stays silent unless the file proves both sides are Dates; no autofix.",
        examples: &[
            "no-date-equality: error",
            "no-date-equality:\n  severity: warn\n  include: [\"src/**\"]",
        ],
        options_schema: None,
    },
    RuleDoc {
        name: "no-floating-promise",
        description: "Experimental, opt-in: list it under rules.experimental. Flags an expression statement that drops a promise the same file proves it created: a call to a same-file async function, new Promise(...), or Promise.resolve/reject/all/allSettled/race/any(...), optionally followed by .then(onFulfilled) or .finally(...) with no .catch. Stays silent unless the file proves it; no autofix.",
        examples: &["no-floating-promise: error", "no-floating-promise: warn"],
        options_schema: None,
    },
];
