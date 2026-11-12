use crate::config::Severity;
use schemars::{JsonSchema, Schema, SchemaGenerator};

#[derive(JsonSchema)]
#[serde(untagged)]
#[allow(dead_code)]
enum CognitiveComplexityConfig {
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
            extend("default" = 15),
            description = "Highest cognitive complexity a function may have; a function scoring above it is reported. An integer of at least 1. Defaults to 15."
        )]
        max: Option<u32>,
    },
}

pub(crate) fn cognitive_complexity_schema(generator: &mut SchemaGenerator) -> Schema {
    generator.subschema_for::<CognitiveComplexityConfig>()
}
