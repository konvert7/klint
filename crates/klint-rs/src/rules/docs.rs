pub(crate) struct RuleDoc {
    pub name: &'static str,
    pub description: &'static str,
    pub examples: &'static [&'static str],
}

pub(crate) const RULE_DOCS: &[RuleDoc] = &[
    RuleDoc {
        name: "no-unguarded-json-parse",
        description: "Flags `JSON.parse()` calls that aren't wrapped in try/catch — malformed input crashes the process.",
        examples: &["no-unguarded-json-parse: error"],
    },
    RuleDoc {
        name: "no-sync-in-async",
        description: "Flags synchronous I/O (`readFileSync`, `execSync`, etc.) inside async functions — blocks the event loop when an async equivalent is available.",
        examples: &[
            "no-sync-in-async: error",
            "no-sync-in-async:\n  severity: warn\n  include: [\"src/server/**\"]",
        ],
    },
    RuleDoc {
        name: "no-async-predicate",
        description: "Flags `async` functions passed to `.filter`/`.some`/`.every`/`.find`/`.findIndex`. Array methods coerce the returned promise to truthy, so every element appears to match.",
        examples: &["no-async-predicate: error"],
    },
    RuleDoc {
        name: "no-nested-template-literals",
        description: "Flags template literals nested inside template literals — almost always unreadable; extract to a variable.",
        examples: &["no-nested-template-literals: warn"],
    },
    RuleDoc {
        name: "no-consecutive-array-push",
        description: "Flags consecutive `arr.push(x); arr.push(y);` calls — combine into `arr.push(x, y)` or `arr.push(...items)`.",
        examples: &["no-consecutive-array-push: warn"],
    },
    RuleDoc {
        name: "no-string-match",
        description: "Flags `String#match` — prefer `.matchAll()` for iteration or `.test()` for boolean checks. `.match()` returns different shapes depending on the `/g` flag.",
        examples: &["no-string-match: error"],
    },
    RuleDoc {
        name: "sonar/prefer-string-replaceall",
        description: "Flags `.replace(/x/g, y)` on strings — `.replaceAll('x', y)` is clearer and faster for all-occurrence replacements.",
        examples: &["sonar/prefer-string-replaceall: error"],
    },
    RuleDoc {
        name: "sonar/prefer-string-raw-regexp",
        description: "Flags regex literals built from escape-heavy string concatenation — `String.raw` drops the double backslashes.",
        examples: &["sonar/prefer-string-raw-regexp: warn"],
    },
    RuleDoc {
        name: "sonar/prefer-string-raw",
        description: "Flags string literals with multiple backslash escapes — `String.raw` is more readable (Windows paths, regex source, etc.).",
        examples: &["sonar/prefer-string-raw: warn"],
    },
    RuleDoc {
        name: "sonar/prefer-nullish-coalescing-assign",
        description: "Flags explicit nullish assignment guards — `x ??= y` is the dedicated nullish assignment operator.",
        examples: &["sonar/prefer-nullish-coalescing-assign: warn"],
    },
    RuleDoc {
        name: "sonar/no-single-char-class",
        description: "Flags single-character character classes in regex (e.g. `/[a]/`) — drop the brackets; the regex matches the same.",
        examples: &["sonar/no-single-char-class: warn"],
    },
    RuleDoc {
        name: "sonar/prefer-at",
        description: "Flags negative-index access via `arr[arr.length - 1]` — `.at(-1)` reads cleaner for last-element access.",
        examples: &["sonar/prefer-at: warn"],
    },
];
