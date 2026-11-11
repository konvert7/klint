use schemars::generate::SchemaSettings;
use schemars::{Schema, SchemaGenerator, json_schema};
use serde_json::{Map, Value, json};

use crate::config::{RawConfig, RuleConfig};
use crate::rules::RULE_DOCS;
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

pub(crate) fn rules_schema(generator: &mut SchemaGenerator) -> Schema {
    let rule_value = generator.subschema_for::<RuleConfig>().to_value();
    let properties: Map<String, Value> = RULE_DOCS
        .iter()
        .map(|doc| {
            let mut entry = doc.options_schema.map_or_else(
                || rule_value.clone(),
                |schema_for| schema_for(generator).to_value(),
            );
            let fields = entry
                .as_object_mut()
                .expect("an inlined rule value schema is a JSON object");
            fields.insert("description".to_string(), json!(doc.description));
            fields.insert("examples".to_string(), json!(doc.examples));
            (doc.name.to_string(), entry)
        })
        .collect();
    json_schema!({
        "type": "object",
        "properties": properties,
        "additionalProperties": rule_value,
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
