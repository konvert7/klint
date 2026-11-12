use jsonc_parser::ast::{self, Value};
use jsonc_parser::common::Ranged;
use jsonc_parser::{CollectOptions, ParseOptions, parse_to_ast};

use super::json::{line_at, quoted};
use super::levels::{LevelChange, LevelPlan};

const NOT_AN_OBJECT: &str = "the config is not a JSON object";

struct Splice {
    start: usize,
    end: usize,
    text: String,
}

pub(super) fn migrate_levels_json(text: &str, plans: &[LevelPlan]) -> Result<String, String> {
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

    let mut splices = plans
        .iter()
        .map(|plan| splice_for(root, body, plan, eol))
        .collect::<Result<Vec<_>, _>>()?;
    splices.sort_by_key(|splice| std::cmp::Reverse(splice.start));

    let mut out = body.to_string();

    for splice in splices {
        out.replace_range(splice.start..splice.end, &splice.text);
    }

    Ok(if bom { format!("\u{feff}{out}") } else { out })
}

fn splice_for(
    root: &ast::Object,
    body: &str,
    plan: &LevelPlan,
    eol: &str,
) -> Result<Splice, String> {
    let prop = root
        .get_object("rules")
        .and_then(|rules| rules.get_object("core"))
        .and_then(|core| core.get(plan.rule))
        .ok_or_else(|| {
            format!(
                "rules.core.{} was not found — migrate it by hand",
                plan.rule
            )
        })?;
    let range = prop.value.range();
    let text = new_value_text(
        plan.rule,
        body,
        &prop.value,
        plan.change,
        indent_of_line(body, prop.range.start),
        eol,
    )?;
    Ok(Splice {
        start: range.start,
        end: range.end,
        text,
    })
}

fn indent_of_line(body: &str, offset: usize) -> usize {
    let line_start = body[..offset].rfind('\n').map_or(0, |index| index + 1);
    body[line_start..].chars().take_while(|c| *c == ' ').count()
}

fn new_value_text(
    rule: &str,
    body: &str,
    value: &Value,
    change: LevelChange,
    base_indent: usize,
    eol: &str,
) -> Result<String, String> {
    if change == LevelChange::TurnOff {
        return Ok(quoted("off"));
    }

    let Value::Object(object) = value else {
        return Err(format!(
            "line {}: rules.core.{rule} is not an object",
            line_at(body, value.range().start)
        ));
    };
    let members = object_members(body, object, change);
    Ok(layout(body, object, &members, base_indent, eol))
}

fn object_members(body: &str, object: &ast::Object, change: LevelChange) -> Vec<String> {
    let mut members = Vec::new();

    if let LevelChange::InsertLevel { limit } = change {
        members.push(format!("\"error\": {limit}"));
    }

    members.extend(
        object
            .properties
            .iter()
            .filter_map(|prop| member_text(body, prop, change)),
    );
    members
}

fn member_text(body: &str, prop: &ast::ObjectProp, change: LevelChange) -> Option<String> {
    match (prop.name.as_str(), change) {
        ("max", LevelChange::RenameMax { to }) => Some(format!(
            "{}: {}",
            quoted(to.as_str()),
            value_source(body, &prop.value)
        )),
        ("severity", LevelChange::RenameMax { .. }) => None,
        ("severity", LevelChange::ReplaceSeverity { to, limit }) => {
            Some(format!("{}: {limit}", quoted(to.as_str())))
        }
        _ => Some(body[prop.range.start..prop.range.end].to_string()),
    }
}

fn value_source<'b>(body: &'b str, value: &Value) -> &'b str {
    let range = value.range();
    &body[range.start..range.end]
}

fn layout(
    body: &str,
    object: &ast::Object,
    members: &[String],
    base_indent: usize,
    eol: &str,
) -> String {
    if members.is_empty() {
        return "{}".to_string();
    }

    if !body[object.range.start..object.range.end].contains('\n') {
        return format!("{{{}}}", members.join(", "));
    }

    let member_indent = " ".repeat(base_indent + 2);
    let separator = format!(",{eol}");
    let lines = members
        .iter()
        .map(|member| format!("{member_indent}{member}"))
        .collect::<Vec<_>>()
        .join(&separator);
    format!("{{{eol}{lines}{eol}{}}}", " ".repeat(base_indent))
}
