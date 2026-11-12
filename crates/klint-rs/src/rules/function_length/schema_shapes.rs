use crate::config::Severity;
use schemars::{JsonSchema, Schema, SchemaGenerator};

#[derive(JsonSchema)]
#[serde(untagged)]
#[allow(dead_code)]
enum FunctionLengthConfig {
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

pub(crate) fn function_length_schema(generator: &mut SchemaGenerator) -> Schema {
    generator.subschema_for::<FunctionLengthConfig>()
}
