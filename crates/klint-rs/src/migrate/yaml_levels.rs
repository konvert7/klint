use std::collections::BTreeMap;

use super::levels::{LevelChange, LevelPlan};
use super::yaml_lines::{
    Line, Lines, Span, indent_of, is_blank, is_comment, join, masked, split, top_level_entry,
    trailing_comment, without_comment,
};
use super::yaml_rules::{
    Entry, check_supported, copy_line, entries, renamed_key_line, settle_terminators, split_key,
};

#[derive(Default)]
struct Edits {
    replace: BTreeMap<usize, Option<String>>,
    insert_before: BTreeMap<usize, String>,
}

impl Edits {
    fn set(&mut self, index: usize, text: String) {
        self.replace.insert(index, Some(text));
    }

    fn remove(&mut self, index: usize) {
        self.replace.insert(index, None);
    }

    fn merge(&mut self, other: Edits) {
        self.replace.extend(other.replace);
        self.insert_before.extend(other.insert_before);
    }
}

struct Child {
    key: String,
    line: usize,
}

pub(super) fn migrate_levels_yaml(text: &str, plans: &[LevelPlan]) -> Result<String, String> {
    let lines = split(text);
    let Some(rules) = top_level_entry(&lines, "rules") else {
        return Ok(text.to_string());
    };
    check_supported(&lines, rules)?;
    let core = core_entries(&lines, rules)?;

    let mut edits = Edits::default();

    for plan in plans {
        let entry = core
            .iter()
            .find(|entry| entry.key == plan.rule)
            .ok_or_else(|| not_a_block_entry(rules.start + 1, plan.rule))?;
        edits.merge(rule_edits(&lines, entry, plan)?);
    }

    let mut out = apply_edits(&lines, &edits);
    settle_terminators(&mut out, &lines);
    Ok(join(&Lines {
        bom: lines.bom,
        eol: lines.eol,
        lines: out,
    }))
}

fn not_a_block_entry(line: usize, rule: &str) -> String {
    format!(
        "line {line}: {rule} could not be found as a block entry in rules.core — write it as one first"
    )
}

fn core_entries(lines: &Lines, rules: Span) -> Result<Vec<Entry>, String> {
    let (_, top) = entries(
        lines,
        Span {
            start: rules.start + 1,
            end: rules.end,
        },
    );
    let Some(core) = top.into_iter().find(|entry| entry.key == "core") else {
        return Ok(Vec::new());
    };
    if !header_value(&lines.lines[core.key_line].text).is_empty() {
        return Err(format!(
            "line {}: rules.core is a flow map — write it as a block map first",
            core.key_line + 1
        ));
    }

    let last = core.lines.iter().copied().max().unwrap_or(core.key_line);
    let body = Span {
        start: core.key_line + 1,
        end: last + 1,
    };
    Ok(entries(lines, body).1)
}

fn rule_edits(lines: &Lines, entry: &Entry, plan: &LevelPlan) -> Result<Edits, String> {
    let value = header_value(&lines.lines[entry.key_line].text);
    if value.is_empty() {
        block_edits(lines, entry, plan)
    } else if value.starts_with('{') {
        flow_edits(lines, entry, plan)
    } else {
        Err(cannot_read(entry.key_line + 1, plan.rule))
    }
}

fn cannot_read(line: usize, rule: &str) -> String {
    format!(
        "line {line}: {rule} has a value this migration cannot read — write it as a block map first"
    )
}

fn header_value(text: &str) -> &str {
    split_key(text).map_or("", |parts| without_comment(parts.rest)[1..].trim())
}

fn is_content(text: &str) -> bool {
    !is_blank(text) && !is_comment(text)
}

fn child_keys(lines: &Lines, entry: &Entry) -> Result<(usize, Vec<Child>), String> {
    let body: Vec<usize> = entry
        .lines
        .iter()
        .copied()
        .filter(|&index| index > entry.key_line)
        .collect();
    let child_indent = body
        .iter()
        .map(|&index| lines.lines[index].text.as_str())
        .find(|text| is_content(text))
        .map_or(0, indent_of);

    let mut children: Vec<Child> = Vec::new();

    for &index in &body {
        let text = &lines.lines[index].text;
        if !is_content(text) || indent_of(text) != child_indent {
            continue;
        }

        let Some(parts) = split_key(text) else {
            continue;
        };
        if children.iter().any(|child| child.key == parts.key) {
            return Err(format!(
                "line {}: duplicate key \"{}\"",
                index + 1,
                parts.key
            ));
        }

        children.push(Child {
            key: parts.key.to_string(),
            line: index,
        });
    }

    Ok((child_indent, children))
}

fn ensure_values_on_their_line(
    lines: &Lines,
    children: &[Child],
    rule: &str,
) -> Result<(), String> {
    for child in children
        .iter()
        .filter(|child| matches!(child.key.as_str(), "max" | "severity"))
    {
        if header_value(&lines.lines[child.line].text).is_empty() {
            return Err(format!(
                "line {}: {} on {rule} has no value on its line — write it as key: value first",
                child.line + 1,
                child.key
            ));
        }
    }

    Ok(())
}

fn child_named<'a>(children: &'a [Child], key: &str) -> Option<&'a Child> {
    children.iter().find(|child| child.key == key)
}

fn block_edits(lines: &Lines, entry: &Entry, plan: &LevelPlan) -> Result<Edits, String> {
    let (child_indent, children) = child_keys(lines, entry)?;
    ensure_values_on_their_line(lines, &children, plan.rule)?;
    let missing = || cannot_read(entry.key_line + 1, plan.rule);
    let mut edits = Edits::default();

    match plan.change {
        LevelChange::RenameMax { to } => {
            let max = child_named(&children, "max").ok_or_else(missing)?;
            edits.set(
                max.line,
                renamed_key_line(&lines.lines[max.line].text, to.as_str()),
            );

            if let Some(severity) = child_named(&children, "severity") {
                edits.remove(severity.line);
            }
        }
        LevelChange::ReplaceSeverity { to, limit } => {
            let severity = child_named(&children, "severity").ok_or_else(missing)?;
            edits.set(
                severity.line,
                with_value(
                    &lines.lines[severity.line].text,
                    to.as_str(),
                    &limit.to_string(),
                ),
            );
        }
        LevelChange::InsertLevel { limit } => {
            let first = children.first().ok_or_else(missing)?;
            edits.insert_before.insert(
                first.line,
                format!("{}error: {limit}", " ".repeat(child_indent)),
            );
        }
        LevelChange::TurnOff => turn_off_block(lines, entry, &mut edits),
    }

    Ok(edits)
}

fn with_value(text: &str, new_key: &str, new_value: &str) -> String {
    let renamed = renamed_key_line(text, new_key);
    let Some(parts) = split_key(text) else {
        return renamed;
    };
    let head = &renamed[..renamed.len() - parts.rest.len()];
    format!(
        "{head}{}",
        parts.rest.replacen(header_value(text), new_value, 1)
    )
}

fn turn_off_block(lines: &Lines, entry: &Entry, edits: &mut Edits) {
    edits.set(
        entry.key_line,
        off_header(&lines.lines[entry.key_line].text),
    );

    for &index in entry.lines.iter().filter(|&&index| index > entry.key_line) {
        if is_content(&lines.lines[index].text) {
            edits.remove(index);
        }
    }
}

fn off_header(text: &str) -> String {
    let code = without_comment(text);
    format!("{code} off{}", &text[code.len()..])
}

fn flow_edits(lines: &Lines, entry: &Entry, plan: &LevelPlan) -> Result<Edits, String> {
    let text = &lines.lines[entry.key_line].text;
    let number = entry.key_line + 1;
    let code = without_comment(text);
    let several = || {
        format!(
            "line {number}: {} is a flow map over several lines — write it on one line or as a block map first",
            plan.rule
        )
    };
    let (Some(open), Some(close)) = (code.find('{'), code.rfind('}')) else {
        return Err(several());
    };
    if close < open || !code[close + 1..].trim().is_empty() {
        return Err(several());
    }

    let line = match plan.change {
        LevelChange::TurnOff => format!("{}off{}", &text[..open], &text[code.len()..]),
        _ => {
            let inner = &code[open + 1..close];
            let items = flow_items(inner);
            let edited = edited_flow_items(&items, plan, number)?;
            format!(
                "{}{{{}{}{}}}{}",
                &text[..open],
                leading_space(inner),
                edited.join(", "),
                trailing_space(inner),
                &text[close + 1..]
            )
        }
    };

    let mut edits = Edits::default();
    edits.set(entry.key_line, line);
    Ok(edits)
}

fn leading_space(inner: &str) -> &str {
    &inner[..inner.len() - inner.trim_start().len()]
}

fn trailing_space(inner: &str) -> &str {
    &inner[inner.trim_end().len()..]
}

fn flow_items(inner: &str) -> Vec<&str> {
    let hidden = masked(inner);
    let mut items = Vec::new();
    let mut depth = 0_usize;
    let mut start = 0;

    for (index, c) in hidden.char_indices() {
        match c {
            '[' | '{' => depth += 1,
            ']' | '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                items.push(&inner[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }

    items.push(&inner[start..]);

    items
        .into_iter()
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .collect()
}

fn split_flow_item(item: &str) -> Option<(&str, &str)> {
    match item.chars().next() {
        Some(quote @ ('"' | '\'')) => {
            let rest = &item[1..];
            let end = rest.find(quote)?;
            Some((&rest[..end], &rest[end + 1..]))
        }
        _ => {
            let colon = item
                .match_indices(':')
                .map(|(index, _)| index)
                .find(|&index| {
                    item[index + 1..].is_empty() || item[index + 1..].starts_with(' ')
                })?;
            Some((item[..colon].trim(), &item[colon..]))
        }
    }
}

fn renamed_flow_item(item: &str, new_key: &str) -> String {
    let quote = if item.starts_with(['"', '\'']) {
        &item[..1]
    } else {
        ""
    };

    match split_flow_item(item) {
        Some((_, after)) => format!("{quote}{new_key}{quote}{after}"),
        None => item.to_string(),
    }
}

fn rename_max_item(item: &str, to: &str) -> Option<String> {
    match split_flow_item(item) {
        Some(("max", _)) => Some(renamed_flow_item(item, to)),
        Some(("severity", _)) => None,
        _ => Some(item.to_string()),
    }
}

fn replace_severity_item(item: &str, to: &str, limit: u32) -> String {
    match split_flow_item(item) {
        Some(("severity", _)) => format!("{to}: {limit}"),
        _ => item.to_string(),
    }
}

fn edited_flow_items(
    items: &[&str],
    plan: &LevelPlan,
    number: usize,
) -> Result<Vec<String>, String> {
    let mut keys: Vec<&str> = Vec::new();

    for item in items {
        let Some((key, _)) = split_flow_item(item) else {
            return Err(cannot_read(number, plan.rule));
        };
        if keys.contains(&key) {
            return Err(format!("line {number}: duplicate key \"{key}\""));
        }

        keys.push(key);
    }

    Ok(match plan.change {
        LevelChange::RenameMax { to } => items
            .iter()
            .filter_map(|item| rename_max_item(item, to.as_str()))
            .collect(),
        LevelChange::ReplaceSeverity { to, limit } => items
            .iter()
            .map(|item| replace_severity_item(item, to.as_str(), limit))
            .collect(),
        LevelChange::InsertLevel { limit } => std::iter::once(format!("error: {limit}"))
            .chain(items.iter().map(|item| item.to_string()))
            .collect(),
        LevelChange::TurnOff => items.iter().map(|item| item.to_string()).collect(),
    })
}

fn apply_edits(lines: &Lines, edits: &Edits) -> Vec<Line> {
    let mut out = Vec::new();

    for (index, line) in lines.lines.iter().enumerate() {
        if let Some(text) = edits.insert_before.get(&index) {
            out.push(Line {
                text: text.clone(),
                terminator: lines.eol.to_string(),
            });
        }

        match edits.replace.get(&index) {
            Some(Some(text)) => out.push(Line {
                text: text.clone(),
                terminator: line.terminator.clone(),
            }),
            Some(None) => out.extend(leftover(&line.text).map(|text| Line {
                text,
                terminator: line.terminator.clone(),
            })),
            None => out.push(copy_line(line)),
        }
    }

    out
}

fn leftover(text: &str) -> Option<String> {
    if is_comment(text) {
        return None;
    }

    trailing_comment(text).map(|comment| format!("{}{comment}", " ".repeat(indent_of(text))))
}
