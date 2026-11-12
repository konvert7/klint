use crate::config::Severity;
use schemars::{JsonSchema, Schema, SchemaGenerator};

#[derive(JsonSchema)]
#[serde(untagged)]
#[allow(dead_code)]
enum FunctionLengthV1Config {
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
            range(min = 1),
            extend("default" = 50),
            description = "Highest number of lines a function body may have; a function counting more is reported. An integer of at least 1. Defaults to 50."
        )]
        max: Option<u32>,
        #[serde(rename = "skipBlankLines")]
        #[schemars(
            extend("default" = false),
            description = "Leave blank lines out of the count. Defaults to false."
        )]
        skip_blank_lines: Option<bool>,
    },
}

#[derive(JsonSchema)]
#[schemars(
    deny_unknown_fields,
    description = "warn and/or error limits. A function gets one diagnostic, at the highest level it exceeds.",
    extend("allOf" = [{"anyOf": [{"required": ["warn"]}, {"required": ["error"]}]}])
)]
#[allow(dead_code)]
struct FunctionLengthLevels {
    #[schemars(
        range(min = 1),
        description = "Report a function above this limit as a warning."
    )]
    warn: Option<u32>,
    #[schemars(
        range(min = 1),
        description = "Report a function above this limit as an error. If error is below warn, only error applies."
    )]
    error: Option<u32>,
    #[schemars(
        description = "Glob patterns scoping this rule to a subset of files. Prefix with ! to exclude. Example: [\"src/hooks/**\", \"!src/hooks/scripts/**\"]"
    )]
    include: Option<Vec<String>>,
    #[serde(rename = "skipBlankLines")]
    #[schemars(
        extend("default" = false),
        description = "Leave blank lines out of the count. Defaults to false."
    )]
    skip_blank_lines: Option<bool>,
}

#[derive(JsonSchema)]
#[serde(untagged)]
#[allow(dead_code)]
enum FunctionLengthConfig {
    Severity(Severity),
    Levels(FunctionLengthLevels),
    #[schemars(
        description = "Per-glob limits: the last entry whose include matches a file wins; entries do not merge. An empty list turns the rule off."
    )]
    List(Vec<FunctionLengthLevels>),
}

pub(crate) fn function_length_schema(generator: &mut SchemaGenerator) -> Schema {
    generator.subschema_for::<FunctionLengthConfig>()
}

pub(crate) fn function_length_v1_schema(generator: &mut SchemaGenerator) -> Schema {
    generator.subschema_for::<FunctionLengthV1Config>()
}
