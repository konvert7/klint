use jsonc_parser::ast::{self, Value};
use jsonc_parser::{CollectOptions, ParseOptions, parse_to_ast};

use super::v2_location;
use crate::config_shape::Group;

enum Printed<'v, 'a> {
    Ast(&'v Value<'a>),
    True,
    Object(Vec<(String, Printed<'v, 'a>)>),
}

const NOT_AN_OBJECT: &str = "the config is not a JSON object";

pub(super) fn line_at(text: &str, offset: usize) -> usize {
    1 + text[..offset].matches('\n').count()
}

pub(super) fn quoted(text: &str) -> String {
    serde_json::to_string(text).expect("a string serializes")
}

fn print_members(members: &[(String, Printed)], indent: usize, out: &mut String) {
    if members.is_empty() {
        out.push_str("{}");

        return;
    }

    out.push_str("{\n");

    for (i, (key, member)) in members.iter().enumerate() {
        out.push_str(&" ".repeat(indent + 2));
        out.push_str(&quoted(key));
        out.push_str(": ");
        print_printed(member, indent + 2, out);
        out.push_str(if i + 1 < members.len() { ",\n" } else { "\n" });
    }

    out.push_str(&" ".repeat(indent));
    out.push('}');
}

fn print_printed(printed: &Printed, indent: usize, out: &mut String) {
    match printed {
        Printed::Ast(value) => print_value(value, indent, out),
        Printed::True => out.push_str("true"),
        Printed::Object(members) => print_members(members, indent, out),
    }
}

fn print_array(elements: &[Value], indent: usize, out: &mut String) {
    if elements.is_empty() {
        out.push_str("[]");

        return;
    }

    out.push_str("[\n");

    for (i, element) in elements.iter().enumerate() {
        out.push_str(&" ".repeat(indent + 2));
        print_value(element, indent + 2, out);
        out.push_str(if i + 1 < elements.len() { ",\n" } else { "\n" });
    }

    out.push_str(&" ".repeat(indent));
    out.push(']');
}

fn print_value(value: &Value, indent: usize, out: &mut String) {
    match value {
        Value::StringLit(lit) => out.push_str(&quoted(&lit.value)),
        Value::NumberLit(lit) => out.push_str(lit.value),
        Value::BooleanLit(lit) => out.push_str(if lit.value { "true" } else { "false" }),
        Value::NullKeyword(_) => out.push_str("null"),
        Value::Array(array) => print_array(&array.elements, indent, out),
        Value::Object(object) => {
            let members: Vec<(String, Printed)> = object
                .properties
                .iter()
                .map(|prop| (prop.name.as_str().to_string(), Printed::Ast(&prop.value)))
                .collect();
            print_members(&members, indent, out);
        }
    }
}

fn lists_sonar(text: &str, plugins: &Value) -> Result<bool, String> {
    let (elements, start) = match plugins {
        Value::Array(array) => (&array.elements, array.range.start),
        Value::NullKeyword(_) => return Ok(false),
        other => {
            return Err(format!(
                "line {}: plugins is not a list of names",
                line_at(text, range_start(other))
            ));
        }
    };

    for element in elements {
        match element {
            Value::StringLit(lit) if lit.value == "sonar" => {}
            Value::StringLit(lit) => return Err(unknown_plugin(text, &lit.value, lit.range.start)),
            _ => {
                return Err(format!(
                    "line {}: plugins is not a list of names",
                    line_at(text, start)
                ));
            }
        }
    }

    Ok(!elements.is_empty())
}

fn range_start(value: &Value) -> usize {
    match value {
        Value::StringLit(lit) => lit.range.start,
        Value::NumberLit(lit) => lit.range.start,
        Value::BooleanLit(lit) => lit.range.start,
        Value::NullKeyword(lit) => lit.range.start,
        Value::Object(object) => object.range.start,
        Value::Array(array) => array.range.start,
    }
}

fn unknown_plugin(text: &str, name: &str, offset: usize) -> String {
    format!(
        "line {}: plugins lists \"{name}\" — only sonar can be migrated",
        line_at(text, offset)
    )
}

fn grouped<'v, 'a>(flat: &'v ast::Object<'a>, group: Group) -> Vec<(String, Printed<'v, 'a>)> {
    flat.properties
        .iter()
        .filter_map(|prop| {
            let (found, short) = v2_location(prop.name.as_str());
            (found == group).then(|| (short.to_string(), Printed::Ast(&prop.value)))
        })
        .collect()
}

fn new_rules<'v, 'a>(
    flat: Option<&'v ast::Object<'a>>,
    recommended: bool,
) -> Option<Printed<'v, 'a>> {
    let mut members: Vec<(String, Printed)> = Vec::new();

    if recommended {
        members.push(("recommended".to_string(), Printed::True));
    }

    for group in [Group::Core, Group::Sonar, Group::Experimental] {
        let found = flat.map_or_else(Vec::new, |flat| grouped(flat, group));
        if !found.is_empty() {
            members.push((group.key().to_string(), Printed::Object(found)));
        }
    }

    (!members.is_empty()).then_some(Printed::Object(members))
}

fn top_level<'v, 'a>(
    root: &'v ast::Object<'a>,
    mut rules: Option<Printed<'v, 'a>>,
) -> Vec<(String, Printed<'v, 'a>)> {
    let has_rules_prop = root.get("rules").is_some();
    let mut out = Vec::new();

    for prop in &root.properties {
        let name = prop.name.as_str();
        match name {
            "plugins" if has_rules_prop => {}
            "plugins" => out.extend(rules.take().map(|new| ("rules".to_string(), new))),
            "rules" => out.push((
                name.to_string(),
                rules.take().unwrap_or(Printed::Ast(&prop.value)),
            )),
            _ => out.push((name.to_string(), Printed::Ast(&prop.value))),
        }
    }

    out
}

pub(super) fn migrate_json(text: &str) -> Result<String, String> {
    let (bom, body) = match text.strip_prefix('\u{feff}') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let eol = if body.contains("\r\n") { "\r\n" } else { "\n" };
    let parsed = parse_to_ast(body, &CollectOptions::default(), &ParseOptions::default())
        .map_err(|_| NOT_AN_OBJECT.to_string())?;
    let Some(Value::Object(root)) = &parsed.value else {
        return Err(NOT_AN_OBJECT.to_string());
    };
    let recommended = match root.get("plugins") {
        Some(prop) => lists_sonar(body, &prop.value)?,
        None => false,
    };
    let flat = root.get_object("rules");
    let members = top_level(root, new_rules(flat, recommended));
    let mut out = String::new();
    print_members(&members, 0, &mut out);
    out.push('\n');
    let out = out.replace('\n', eol);
    Ok(if bom { format!("\u{feff}{out}") } else { out })
}
