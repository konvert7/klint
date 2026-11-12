use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Value;

use crate::config::{RawConfig, RuleConfig};
use crate::config_check::line_of_key;
use crate::migrate::v2_location;
use crate::output::Violation;
use crate::plugins::{SONAR_PREFIX, effective_rules, recommended_rules};

pub(crate) const V1_MESSAGE: &str =
    "This config uses the v1 format — run `klint migrate` to upgrade it in place.";
const RULE: &str = "klint/config";
const V2_KEYS: &[&str] = &["recommended", "core", "sonar", "experimental"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Group {
    Core,
    Sonar,
    Experimental,
}

impl Group {
    pub(crate) fn key(self) -> &'static str {
        match self {
            Group::Core => "core",
            Group::Sonar => "sonar",
            Group::Experimental => "experimental",
        }
    }
}

pub(crate) struct Misgrouped {
    pub(crate) name: String,
    pub(crate) written_in: Group,
    pub(crate) belongs_in: Group,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ConfigShape {
    V1,
    V2,
    Mixed { keys: Vec<String> },
}

pub(crate) enum ResolveError {
    InvalidValue,
    UnknownPlugin(String),
}

impl ResolveError {
    pub(crate) fn into_message(self, config_path: &Path) -> String {
        match self {
            ResolveError::InvalidValue => {
                format!("klint: failed to parse {}", config_path.display())
            }
            ResolveError::UnknownPlugin(message) => message,
        }
    }
}

pub(crate) struct ResolvedRules {
    pub(crate) rules: BTreeMap<String, RuleConfig>,
    pub(crate) written: BTreeMap<String, String>,
    pub(crate) misplaced: Vec<String>,
    pub(crate) unknown_rules_keys: Vec<String>,
    pub(crate) shape: ConfigShape,
    pub(crate) experimental: BTreeMap<String, RuleConfig>,
    pub(crate) misgrouped: Vec<Misgrouped>,
}

impl ResolvedRules {
    pub(crate) fn written_key<'a>(&'a self, rule_id: &'a str) -> &'a str {
        self.written.get(rule_id).map_or(rule_id, String::as_str)
    }
}

pub(crate) fn is_v1_rule_name(key: &str) -> bool {
    crate::rules::is_supported_rule(key)
        || crate::unsupported::is_type_checker_rule(key)
        || key.starts_with(SONAR_PREFIX)
}

pub(crate) fn is_misplaced(group: Group, key: &str) -> bool {
    is_v1_rule_name(key) && v2_location(key) != (group, key)
}

fn is_v2_key(key: &str) -> bool {
    V2_KEYS.contains(&key)
}

pub(crate) fn config_shape(raw: &RawConfig) -> ConfigShape {
    let keys: Vec<&str> = raw
        .rules
        .as_ref()
        .map_or_else(Vec::new, |rules| rules.keys().map(String::as_str).collect());
    let has_v2 = keys.iter().any(|key| is_v2_key(key));
    let v1_names: Vec<&str> = keys
        .iter()
        .copied()
        .filter(|key| !is_v2_key(key) && is_v1_rule_name(key))
        .collect();
    let has_flat = keys.iter().any(|key| !is_v2_key(key));

    match (has_v2, raw.plugins.is_some() || has_flat) {
        (true, _) => shape_with_v2_keys(raw, &v1_names),
        (false, true) => ConfigShape::V1,
        (false, false) => ConfigShape::V2,
    }
}

fn shape_with_v2_keys(raw: &RawConfig, v1_names: &[&str]) -> ConfigShape {
    let mut keys = Vec::new();

    if raw.plugins.is_some() {
        keys.push("plugins".to_string());
    }

    keys.extend(v1_names.iter().map(|name| format!("rules.{name}")));

    if keys.is_empty() {
        ConfigShape::V2
    } else {
        ConfigShape::Mixed { keys }
    }
}

pub(crate) fn resolve_rules(raw: &RawConfig) -> Result<ResolvedRules, ResolveError> {
    let shape = config_shape(raw);
    if shape == ConfigShape::V1 {
        resolve_v1(raw)
    } else {
        resolve_v2(raw, shape)
    }
}

fn resolve_v1(raw: &RawConfig) -> Result<ResolvedRules, ResolveError> {
    let all = all_rules(raw);
    let mut flat = BTreeMap::new();

    for (key, value) in &all {
        flat.insert(key.clone(), to_rule_config(value, false)?.into_v1());
    }

    let written: BTreeMap<String, String> =
        flat.keys().map(|key| (key.clone(), key.clone())).collect();
    let plugins = raw.plugins.clone().unwrap_or_default();
    let rules = effective_rules(&plugins, flat).map_err(ResolveError::UnknownPlugin)?;
    Ok(ResolvedRules {
        rules,
        written,
        misplaced: Vec::new(),
        unknown_rules_keys: Vec::new(),
        shape: ConfigShape::V1,
        experimental: BTreeMap::new(),
        misgrouped: Vec::new(),
    })
}

fn resolve_v2(raw: &RawConfig, shape: ConfigShape) -> Result<ResolvedRules, ResolveError> {
    let all = all_rules(raw);
    let rules = if recommended_enabled(&all)? {
        recommended_rules()
    } else {
        BTreeMap::new()
    };
    let mut placed = Placed {
        rules,
        experimental: BTreeMap::new(),
        written: BTreeMap::new(),
        misplaced: Vec::new(),
        misgrouped: Vec::new(),
    };

    for group in [Group::Core, Group::Sonar, Group::Experimental] {
        for (name, value) in group_entries(&all, group)? {
            place_entry(group, name, value, &mut placed)?;
        }
    }

    Ok(ResolvedRules {
        rules: placed.rules,
        written: placed.written,
        misplaced: placed.misplaced,
        unknown_rules_keys: all
            .keys()
            .filter(|key| !is_v2_key(key) && !is_v1_rule_name(key))
            .cloned()
            .collect(),
        shape,
        experimental: placed.experimental,
        misgrouped: placed.misgrouped,
    })
}

struct Placed {
    rules: BTreeMap<String, RuleConfig>,
    experimental: BTreeMap<String, RuleConfig>,
    written: BTreeMap<String, String>,
    misplaced: Vec<String>,
    misgrouped: Vec<Misgrouped>,
}

fn place_entry(
    group: Group,
    name: &str,
    value: &Value,
    placed: &mut Placed,
) -> Result<(), ResolveError> {
    if group == Group::Experimental {
        return place_experimental_entry(name, value, placed);
    }

    if home_group(name) == Some(Group::Experimental) {
        placed.misgrouped.push(Misgrouped {
            name: name.to_string(),
            written_in: group,
            belongs_in: Group::Experimental,
        });

        return Ok(());
    }

    place_grouped_entry(group, name, value, placed)
}

fn place_grouped_entry(
    group: Group,
    name: &str,
    value: &Value,
    placed: &mut Placed,
) -> Result<(), ResolveError> {
    if is_misplaced(group, name) {
        placed.misplaced.push(name.to_string());

        return Ok(());
    }

    let id = rule_id(group, name);
    let list_allowed = crate::rules::accepts_list_form(&id);
    placed
        .rules
        .insert(id.clone(), to_rule_config(value, list_allowed)?);
    placed.written.insert(id, name.to_string());
    Ok(())
}

fn place_experimental_entry(
    name: &str,
    value: &Value,
    placed: &mut Placed,
) -> Result<(), ResolveError> {
    match home_group(name) {
        Some(Group::Experimental) => {
            placed
                .experimental
                .insert(name.to_string(), to_rule_config(value, false)?);
            placed.written.insert(name.to_string(), name.to_string());
        }
        Some(belongs_in) => placed.misgrouped.push(Misgrouped {
            name: name.to_string(),
            written_in: Group::Experimental,
            belongs_in,
        }),
        None => return place_grouped_entry(Group::Core, name, value, placed),
    }

    Ok(())
}

fn home_group(name: &str) -> Option<Group> {
    if crate::rules::is_experimental_rule(name) {
        return Some(Group::Experimental);
    }

    if crate::rules::is_supported_rule(name) {
        return Some(v2_location(name).0);
    }

    if crate::rules::is_supported_rule(&format!("{SONAR_PREFIX}{name}")) {
        return Some(Group::Sonar);
    }

    None
}

fn all_rules(raw: &RawConfig) -> BTreeMap<String, Value> {
    raw.rules.clone().unwrap_or_default()
}

fn recommended_enabled(all: &BTreeMap<String, Value>) -> Result<bool, ResolveError> {
    match all.get("recommended") {
        None => Ok(false),
        Some(Value::Bool(enabled)) => Ok(*enabled),
        Some(_) => Err(ResolveError::InvalidValue),
    }
}

fn group_entries(
    all: &BTreeMap<String, Value>,
    group: Group,
) -> Result<Vec<(&str, &Value)>, ResolveError> {
    match all.get(group.key()) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Object(entries)) => Ok(entries
            .iter()
            .map(|(name, value)| (name.as_str(), value))
            .collect()),
        Some(_) => Err(ResolveError::InvalidValue),
    }
}

fn rule_id(group: Group, name: &str) -> String {
    match group {
        Group::Sonar => format!("{SONAR_PREFIX}{name}"),
        Group::Core | Group::Experimental => name.to_string(),
    }
}

fn to_rule_config(value: &Value, list_allowed: bool) -> Result<RuleConfig, ResolveError> {
    let config: RuleConfig =
        serde_json::from_value(value.clone()).map_err(|_| ResolveError::InvalidValue)?;
    if config.list_items().is_some() && !list_allowed {
        return Err(ResolveError::InvalidValue);
    }

    Ok(config)
}

pub(crate) fn check_rule_values(raw: &RawConfig) -> Result<(), String> {
    match resolve_rules(raw) {
        Err(ResolveError::InvalidValue) => Err("invalid rule value".to_string()),
        _ => Ok(()),
    }
}

pub(crate) fn shape_advisories(resolved: &ResolvedRules, text: &str, file: &str) -> Vec<Violation> {
    shape_warning(&resolved.shape, text, file)
        .into_iter()
        .chain(resolved.unknown_rules_keys.iter().map(|key| {
            advisory(
                file,
                line_of_key(text, key),
                format!(
                    "Unknown key \"rules.{key}\" is ignored — rules accepts recommended, core, sonar and experimental."
                ),
            )
        }))
        .chain(resolved.misgrouped.iter().map(|entry| {
            advisory(file, line_of_key(text, &entry.name), misgrouped_message(entry))
        }))
        .collect()
}

fn misgrouped_message(entry: &Misgrouped) -> String {
    format!(
        "Rule \"{}\" belongs in rules.{}, not rules.{}, so this entry is ignored.",
        entry.name,
        entry.belongs_in.key(),
        entry.written_in.key()
    )
}

fn shape_warning(shape: &ConfigShape, text: &str, file: &str) -> Option<Violation> {
    match shape {
        ConfigShape::V1 => Some(advisory(file, 1, V1_MESSAGE.to_string())),
        ConfigShape::Mixed { keys } => Some(advisory(
            file,
            line_of_key(text, first_key_name(keys)),
            mixed_message(keys),
        )),
        ConfigShape::V2 => None,
    }
}

fn first_key_name(keys: &[String]) -> &str {
    keys.first()
        .map_or("", |key| key.strip_prefix("rules.").unwrap_or(key))
}

fn mixed_message(keys: &[String]) -> String {
    format!(
        "This config mixes the v1 and v2 formats ({}) — the v1 entries are ignored; move them into rules.core / rules.sonar or run `klint migrate`.",
        keys.join(", ")
    )
}

fn advisory(file: &str, line: usize, message: String) -> Violation {
    Violation {
        file: file.to_string(),
        line,
        rule: RULE.to_string(),
        message,
        severity: "warn".to_string(),
        fix: None,
    }
}
