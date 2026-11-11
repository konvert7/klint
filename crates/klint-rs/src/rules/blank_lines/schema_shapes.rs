use crate::config::Severity;
use schemars::{JsonSchema, Schema, SchemaGenerator};

#[derive(JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(description = "A statement kind the rule targets. match is an alias of switch.")]
#[allow(dead_code)]
enum BeforeStatementKeyword {
    If,
    For,
    While,
    Switch,
    Match,
    Try,
    Return,
    Guard,
    Loop,
    With,
}

#[derive(JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(description = "A block statement kind the rule targets. match is an alias of switch.")]
#[allow(dead_code)]
enum AfterStatementKeyword {
    If,
    For,
    While,
    Switch,
    Match,
    Try,
    Guard,
    Loop,
    With,
}

#[derive(JsonSchema)]
#[serde(untagged)]
#[allow(dead_code)]
enum BlankLineBeforeConfig {
    Severity(Severity),
    #[schemars(
        extend("additionalProperties" = false),
        description = "Rule options object. Omit severity to default to \"error\". Add include to scope the rule to specific files."
    )]
    Options {
        severity: Option<Severity>,
        #[schemars(
            description = "Glob patterns scoping this rule to a subset of files. Prefix with ! to exclude. Example: [\"src/hooks/**\", \"!src/hooks/scripts/**\"]"
        )]
        include: Option<Vec<String>>,
        #[schemars(
            description = "Statement kinds this rule targets. Defaults to every kind: if, for, while, switch, try, return, guard, loop, with. match is an alias of switch. An empty list turns the rule off."
        )]
        statements: Option<Vec<BeforeStatementKeyword>>,
        #[serde(rename = "allowSetup")]
        #[schemars(
            description = "Skip an if, guard or switch statement whose condition uses a name assigned by the statement directly above it. Defaults to true."
        )]
        allow_setup: Option<bool>,
    },
}

#[derive(JsonSchema)]
#[serde(untagged)]
#[allow(dead_code)]
enum BlankLineAfterConfig {
    Severity(Severity),
    #[schemars(
        extend("additionalProperties" = false),
        description = "Rule options object. Omit severity to default to \"error\". Add include to scope the rule to specific files."
    )]
    Options {
        severity: Option<Severity>,
        #[schemars(
            description = "Glob patterns scoping this rule to a subset of files. Prefix with ! to exclude. Example: [\"src/hooks/**\", \"!src/hooks/scripts/**\"]"
        )]
        include: Option<Vec<String>>,
        #[schemars(
            description = "Block statement kinds this rule targets. Defaults to every kind: if, for, while, switch, try, guard, loop, with. match is an alias of switch. return is not a block statement. An empty list turns the rule off."
        )]
        statements: Option<Vec<AfterStatementKeyword>>,
    },
}

pub(crate) fn before_schema(generator: &mut SchemaGenerator) -> Schema {
    generator.subschema_for::<BlankLineBeforeConfig>()
}

pub(crate) fn after_schema(generator: &mut SchemaGenerator) -> Schema {
    generator.subschema_for::<BlankLineAfterConfig>()
}
