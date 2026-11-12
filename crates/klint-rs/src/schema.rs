use schemars::generate::SchemaSettings;
use schemars::{Schema, SchemaGenerator, json_schema};
use serde_json::{Map, Value, json};

use crate::config::{RawConfig, RuleConfig};
use crate::plugins::SONAR_PREFIX;
use crate::rules::{RULE_DOCS, v1_options_schema};
use crate::version::schema_url_for_version;

const DRAFT_07: &str = "http://json-schema.org/draft-07/schema#";

pub fn config_schema(version: &str) -> Value {
    let mut settings = SchemaSettings::draft07();
    settings.inline_subschemas = true;
    let root = settings
        .into_generator()
        .into_root_schema_for::<RawConfig>();

    let mut schema = root.to_value();
    reject_null(&mut schema);
    let object = schema
        .as_object_mut()
        .expect("a derived root schema is a JSON object");
    object.remove("title");
    object.insert("$schema".to_string(), json!(DRAFT_07));
    object.insert("$id".to_string(), json!(schema_url_for_version(version)));
    schema
}

pub(crate) fn rule_entries(
    generator: &mut SchemaGenerator,
    v1: bool,
) -> Vec<(&'static str, Value)> {
    let rule_value = generator.subschema_for::<RuleConfig>().to_value();
    RULE_DOCS
        .iter()
        .map(|doc| {
            let options_schema = if v1 {
                v1_options_schema(doc.name).or(doc.options_schema)
            } else {
                doc.options_schema
            };
            let mut entry = options_schema.map_or_else(
                || rule_value.clone(),
                |schema_for| schema_for(generator).to_value(),
            );
            let fields = entry
                .as_object_mut()
                .expect("an inlined rule value schema is a JSON object");
            fields.insert("description".to_string(), json!(doc.description));
            fields.insert("examples".to_string(), json!(doc.examples));
            (doc.name, entry)
        })
        .collect()
}

fn rules_map<'a>(
    entries: &[(&'a str, Value)],
    keep: impl Fn(&'a str) -> Option<&'a str>,
) -> Map<String, Value> {
    entries
        .iter()
        .filter_map(|(name, entry)| keep(name).map(|key| (key.to_string(), entry.clone())))
        .collect()
}

fn core_name(name: &str) -> Option<&str> {
    (!name.starts_with(SONAR_PREFIX)).then_some(name)
}

pub(crate) fn rules_schema(generator: &mut SchemaGenerator) -> Schema {
    let rule_value = generator.subschema_for::<RuleConfig>().to_value();
    let entries = rule_entries(generator, false);
    let core = rules_map(&entries, core_name);
    let sonar = rules_map(&entries, |name| name.strip_prefix(SONAR_PREFIX));
    let v1_entries = rule_entries(generator, true);
    let v1: Map<String, Value> = v1_entries
        .iter()
        .map(|(name, entry)| (name.to_string(), entry.clone()))
        .collect();
    json_schema!({
        "anyOf": [
            {
                "type": "object",
                "description": "v2 format: rule groups.",
                "properties": {
                    "recommended": {
                        "type": "boolean",
                        "default": false,
                        "description": "true enables the six sonar rules at error, as plugins: [sonar] did.",
                    },
                    "core": {
                        "type": "object",
                        "description": "Built-in rules by name.",
                        "properties": core,
                        "additionalProperties": rule_value,
                    },
                    "sonar": {
                        "type": "object",
                        "description": "Sonar rules without the sonar/ prefix.",
                        "properties": sonar,
                        "additionalProperties": rule_value,
                    },
                },
                "additionalProperties": false,
            },
            {
                "type": "object",
                "description": "v1 format (deprecated): flat map of rule name → severity or options, sonar rules prefixed with sonar/. Run `klint migrate` to upgrade it to rules.recommended / rules.core / rules.sonar.",
                "properties": v1,
                "additionalProperties": rule_value,
            },
        ],
    })
}

fn reject_null(schema: &mut Value) {
    match schema {
        Value::Object(fields) => {
            drop_null_type(fields);
            drop_null_alternative(fields);
            fields.values_mut().for_each(reject_null);
        }
        Value::Array(items) => items.iter_mut().for_each(reject_null),
        _ => {}
    }
}

fn drop_null_type(fields: &mut Map<String, Value>) {
    let Some(Value::Array(types)) = fields.get_mut("type") else {
        return;
    };
    types.retain(|kind| kind != "null");

    if let [only] = types.as_slice() {
        let only = only.clone();
        fields.insert("type".to_string(), only);
    }
}

fn drop_null_alternative(fields: &mut Map<String, Value>) {
    let Some(Value::Array(alternatives)) = fields.get_mut("anyOf") else {
        return;
    };
    alternatives.retain(|alternative| alternative.get("type") != Some(&json!("null")));

    if let [only] = alternatives.as_slice() {
        let Value::Object(only) = only.clone() else {
            return;
        };
        fields.remove("anyOf");

        for (key, value) in only {
            fields.entry(key).or_insert(value);
        }
    }
}
