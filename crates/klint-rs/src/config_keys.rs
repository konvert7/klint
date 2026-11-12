use serde_json::{Map, Value};

const RULE_GROUPS: [&str; 3] = ["core", "sonar", "experimental"];

#[doc(hidden)]
pub fn config_key_paths() -> Vec<String> {
    let schema = crate::schema::config_schema("0.0.0");
    let mut paths: Vec<String> = schema["properties"]
        .as_object()
        .map(|properties| {
            properties
                .keys()
                .filter(|key| key.as_str() != "plugins")
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    paths.push("rules.recommended".to_string());
    paths.extend(RULE_GROUPS.iter().map(|group| format!("rules.{group}")));
    paths.extend(v2_group_paths(&schema));
    paths.sort();
    paths.dedup();
    paths
}

fn v2_group_paths(schema: &Value) -> Vec<String> {
    let alternatives = schema["properties"]["rules"]["anyOf"]
        .as_array()
        .map_or(&[][..], Vec::as_slice);
    let Some(v2) = alternatives
        .iter()
        .find(|branch| branch["properties"].get("core").is_some())
    else {
        return Vec::new();
    };
    RULE_GROUPS
        .iter()
        .filter_map(|group| {
            let rules = v2["properties"][group]["properties"].as_object()?;
            Some(group_key_paths(group, rules))
        })
        .flatten()
        .collect()
}

fn group_key_paths(group: &str, rules: &Map<String, Value>) -> Vec<String> {
    rules
        .iter()
        .flat_map(|(name, entry)| {
            let entry_path = format!("rules.{group}.{name}");
            let options = option_keys(entry)
                .into_iter()
                .map(|option| format!("{entry_path}.{option}"))
                .collect::<Vec<_>>();
            std::iter::once(entry_path).chain(options)
        })
        .collect()
}

fn option_keys(entry: &Value) -> Vec<String> {
    let alternatives = entry["anyOf"].as_array().map_or(&[][..], Vec::as_slice);
    let mut keys: Vec<String> = alternatives
        .iter()
        .chain(std::iter::once(entry))
        .filter_map(|branch| branch["properties"].as_object())
        .flat_map(|properties| properties.keys().cloned())
        .collect();
    keys.sort();
    keys.dedup();
    keys
}
