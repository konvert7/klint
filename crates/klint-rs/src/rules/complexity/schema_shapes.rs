use crate::config::Severity;
use schemars::{JsonSchema, Schema, SchemaGenerator};

#[derive(JsonSchema)]
#[serde(untagged)]
#[allow(dead_code)]
enum CognitiveComplexityV1Config {
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

#[derive(JsonSchema)]
#[schemars(
    deny_unknown_fields,
    description = "warn and/or error limits. A function gets one diagnostic, at the highest level it exceeds.",
    extend("allOf" = [{"anyOf": [{"required": ["warn"]}, {"required": ["error"]}]}])
)]
#[allow(dead_code)]
struct CognitiveComplexityLevels {
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
}

#[derive(JsonSchema)]
#[serde(untagged)]
#[allow(dead_code)]
enum CognitiveComplexityConfig {
    Severity(Severity),
    Levels(CognitiveComplexityLevels),
    #[schemars(
        description = "Per-glob limits: the last entry whose include matches a file wins; entries do not merge. An empty list turns the rule off."
    )]
    List(Vec<CognitiveComplexityLevels>),
}

pub(crate) fn cognitive_complexity_schema(generator: &mut SchemaGenerator) -> Schema {
    generator.subschema_for::<CognitiveComplexityConfig>()
}

pub(crate) fn cognitive_complexity_v1_schema(generator: &mut SchemaGenerator) -> Schema {
    generator.subschema_for::<CognitiveComplexityV1Config>()
}
