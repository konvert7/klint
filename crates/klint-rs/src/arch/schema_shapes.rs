use schemars::JsonSchema;

use super::StringOrVec;

pub(super) const COUNT_DOC_COMMENTS: &str = "Count doc-comments toward the limit. Doc-comments are `/** … */` JSDoc blocks (TS/JS), `///` (Swift and C#), and `///` or `//!` (Rust); ordinary `//`, `/* */`, and `#` comments are always counted. Defaults to false, so documentation is exempt and only explanatory/inline comments are capped. Has no effect on Python: docstrings are string expressions rather than comments, so they never count toward either limit.";

pub(super) const COMMENT_IGNORE: &str = "Comment lines matching these patterns are structural, not prose — marker comments a build or codegen step reads, for example — and count toward neither limit. A literal substring by default; prefix with `re:` for a regex. Ignored lines still count in the density denominator (they are physical lines) and still hold a comment block together, so a marker cannot be used to split an over-tall block.";

const FORBIDDEN_SCOPE: &str = "Layer name(s) or glob(s) the pattern is forbidden in. Files outside the scope are not checked.";

#[derive(JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(
    description = "Severity for the arch rule. \"error\" exits with code 2; \"warn\" reports but exits 0. (arch rules cannot be \"off\" — remove the entry instead.)"
)]
#[allow(dead_code)]
pub(super) enum ArchSeverity {
    Error,
    Warn,
}

#[derive(JsonSchema)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
pub(super) enum TypeOnly {
    Allow,
}

#[derive(JsonSchema)]
#[serde(untagged)]
#[schemars(extend("examples" = [
    "arch:\n  forbidden:\n    - pattern: \"debugLog(\"\n      in: core\n      message: \"Leaks into the agent event stream\"",
    "arch:\n  forbidden:\n    - jsx-element: [\"button\", \"input\", \"label\"]\n      in: [\"src/app/**/*.tsx\", \"src/components/**/*.tsx\", \"!src/components/ui/**\"]\n      message: \"Use the design-system primitives in @/components/ui/* instead of raw HTML elements.\""
]))]
#[allow(dead_code)]
pub(super) enum ForbiddenRule {
    #[schemars(extend("additionalProperties" = false))]
    Pattern {
        #[schemars(
            description = "Pattern to search for, scanned per line. A literal substring by default (e.g. a console logging call). Prefix with `re:` to match a regular expression instead (e.g. `re:\\bp-\\[\\d+px\\]`). Regexes must use the common JS/RE2 subset — no lookaround or backreferences — so the TS and Rust engines agree. Note: a literal pattern that itself begins with `re:` cannot be expressed; such a value is always read as a regex."
        )]
        pattern: String,
        #[serde(rename = "in")]
        #[schemars(description = FORBIDDEN_SCOPE)]
        in_scope: StringOrVec,
        #[schemars(
            description = "Required error message explaining why the pattern is forbidden. Shown to the agent so it can fix the call site."
        )]
        message: String,
        severity: Option<ArchSeverity>,
    },
    #[schemars(extend("additionalProperties" = false))]
    JsxElement {
        #[serde(rename = "jsx-element")]
        #[schemars(
            description = "JSX intrinsic element name(s) to forbid (e.g. `button`, `input`, `label`). AST-matched on opening and self-closing tag names — robust to whitespace, attributes, and naming collisions like `<buttonGroup>`."
        )]
        jsx_element: StringOrVec,
        #[serde(rename = "in")]
        #[schemars(
            description = "Layer name(s) or glob(s) the element is forbidden in. Files outside the scope are not checked."
        )]
        in_scope: StringOrVec,
        #[schemars(
            description = "Required error message explaining why the raw element is forbidden. Typically points at the design-system replacement (e.g. `Use <Button /> from @/components/ui/button`)."
        )]
        message: String,
        severity: Option<ArchSeverity>,
    },
}

#[derive(JsonSchema)]
#[serde(untagged)]
#[schemars(extend("examples" = [
    "arch:\n  singleton:\n    - pattern: \"process.env.API_KEY\"\n      only: \"src/lib/auth.ts\"\n      message: \"API key access must funnel through the auth module.\"",
    "arch:\n  singleton:\n    - jsx-element: \"button\"\n      only: \"src/components/ui/button.tsx\"\n      in: [\"src/**/*.tsx\"]\n      message: \"Raw <button> belongs only to the Button primitive.\""
]))]
#[allow(dead_code)]
pub(super) enum SingletonRule {
    #[schemars(extend("additionalProperties" = false))]
    Pattern {
        #[schemars(
            description = "Pattern whose appearance is allowed only at one location, scanned per line. A literal substring by default (e.g. `process.env.API_KEY`). Prefix with `re:` to match a regular expression instead. Regexes must use the common JS/RE2 subset — no lookaround or backreferences — so the TS and Rust engines agree. Note: a literal pattern that itself begins with `re:` cannot be expressed; such a value is always read as a regex."
        )]
        pattern: String,
        #[schemars(
            description = "The single file path where this pattern is allowed. Every other occurrence becomes a violation."
        )]
        only: String,
        #[serde(rename = "in")]
        #[schemars(
            description = "Optional scope. Limit the scan to these layers/globs; defaults to all files in `include`."
        )]
        in_scope: Option<StringOrVec>,
        message: String,
        severity: Option<ArchSeverity>,
    },
    #[schemars(extend("additionalProperties" = false))]
    JsxElement {
        #[serde(rename = "jsx-element")]
        #[schemars(
            description = "JSX intrinsic element name(s) allowed only inside the `only` file. AST-matched on tag names."
        )]
        jsx_element: StringOrVec,
        #[schemars(
            description = "The single file path where the element is allowed (typically the design-system primitive that wraps it)."
        )]
        only: String,
        #[serde(rename = "in")]
        in_scope: Option<StringOrVec>,
        message: String,
        severity: Option<ArchSeverity>,
    },
}
