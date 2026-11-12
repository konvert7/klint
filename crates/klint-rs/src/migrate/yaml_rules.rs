use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;

use super::v2_location;
use super::yaml_lines::{
    Line, Lines, Span, indent_of, is_blank, is_comment, join, masked, plugin_names, split,
    top_level_entry, trailing_comment, without_comment,
};
use crate::config_shape::Group;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Quote {
    Plain,
    Double,
    Single,
}

pub(super) struct KeyLine<'a> {
    pub(super) indent: usize,
    pub(super) quote: Quote,
    pub(super) key: &'a str,
    pub(super) rest: &'a str,
}

pub(super) struct Entry {
    pub(super) key_line: usize,
    pub(super) key: String,
    pub(super) lines: Vec<usize>,
}

static ANCHOR_OR_ALIAS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(^|[\s\[{,:])[&*][A-Za-z0-9_-]").expect("valid regex"));
static TAG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(^|[\s\[{,:])![A-Za-z!<]").expect("valid regex"));

fn closing_quote(text: &str, quote: char) -> Option<usize> {
    let mut escaped = false;

    for (i, c) in text.char_indices() {
        if escaped {
            escaped = false;
        } else if quote == '"' && c == '\\' {
            escaped = true;
        } else if c == quote {
            return Some(i);
        }
    }

    None
}

fn plain_key_end(text: &str) -> Option<usize> {
    text.match_indices(':')
        .map(|(i, _)| i)
        .find(|&i| text[i + 1..].is_empty() || text[i + 1..].starts_with(' '))
}

pub(super) fn split_key(text: &str) -> Option<KeyLine<'_>> {
    let indent = indent_of(text);
    let body = &text[indent..];
    let (quote, key_start, key_end) = match body.chars().next()? {
        '"' => (Quote::Double, 1, closing_quote(&body[1..], '"')? + 1),
        '\'' => (Quote::Single, 1, closing_quote(&body[1..], '\'')? + 1),
        '#' => return None,
        _ => (Quote::Plain, 0, plain_key_end(body)?),
    };
    let rest = &body[key_end + usize::from(quote != Quote::Plain)..];
    rest.starts_with(':').then_some(KeyLine {
        indent,
        quote,
        key: &body[key_start..key_end],
        rest,
    })
}

pub(super) fn renamed_key_line(text: &str, new_key: &str) -> String {
    let Some(parts) = split_key(text) else {
        return text.to_string();
    };
    let quote = match parts.quote {
        Quote::Plain => "",
        Quote::Double => "\"",
        Quote::Single => "'",
    };
    format!(
        "{}{quote}{new_key}{quote}{}",
        " ".repeat(parts.indent),
        parts.rest
    )
}

fn child_indent_of(lines: &Lines, body: Span) -> Option<usize> {
    lines.lines[body.start..body.end]
        .iter()
        .find(|line| !is_blank(&line.text) && !is_comment(&line.text))
        .map(|line| indent_of(&line.text))
}

pub(super) fn entries(lines: &Lines, body: Span) -> (Option<usize>, Vec<Entry>) {
    let Some(child) = child_indent_of(lines, body) else {
        return (None, Vec::new());
    };
    let mut found: Vec<Entry> = Vec::new();
    let mut pending: Vec<usize> = Vec::new();

    for i in body.start..body.end {
        let text = &lines.lines[i].text;
        if is_blank(text) || is_comment(text) {
            pending.push(i);
        } else if indent_of(text) == child || found.is_empty() {
            let key = split_key(text).map_or(String::new(), |parts| parts.key.to_string());
            pending.push(i);
            found.push(Entry {
                key_line: i,
                key,
                lines: std::mem::take(&mut pending),
            });
        } else if let Some(last) = found.last_mut() {
            last.lines.append(&mut pending);
            last.lines.push(i);
        }
    }

    if let Some(last) = found.last_mut() {
        last.lines.append(&mut pending);
    }

    (Some(child), found)
}

fn header_flow_map(text: &str) -> bool {
    let value = without_comment(text)
        .trim_start_matches(|c: char| c != ':')
        .trim_start_matches(':')
        .trim();
    value.starts_with('{') && value != "{}"
}

fn token_problem(code: &str, number: usize) -> Option<String> {
    let hidden = masked(code);
    if ANCHOR_OR_ALIAS.is_match(&hidden) {
        return Some(format!("line {number}: YAML anchor or alias in rules"));
    }

    TAG.is_match(&hidden)
        .then(|| format!("line {number}: YAML tag in rules"))
}

struct DuplicateScan {
    child: Option<usize>,
    top: HashSet<String>,
    group: Option<GroupScan>,
}

struct GroupScan {
    indent: Option<usize>,
    seen: HashSet<String>,
}

impl DuplicateScan {
    fn new(child: Option<usize>) -> DuplicateScan {
        DuplicateScan {
            child,
            top: HashSet::new(),
            group: None,
        }
    }

    fn duplicate_in(&mut self, text: &str) -> Option<String> {
        let parts = split_key(text)?;
        if Some(parts.indent) == self.child {
            self.group = matches!(parts.key, "core" | "sonar").then(|| GroupScan {
                indent: None,
                seen: HashSet::new(),
            });

            return (!self.top.insert(parts.key.to_string())).then(|| parts.key.to_string());
        }

        let group = self.group.as_mut()?;
        let indent = *group.indent.get_or_insert(parts.indent);
        (indent == parts.indent && !group.seen.insert(parts.key.to_string()))
            .then(|| parts.key.to_string())
    }
}

pub(super) fn check_supported(lines: &Lines, span: Span) -> Result<(), String> {
    let child = child_indent_of(
        lines,
        Span {
            start: span.start + 1,
            end: span.end,
        },
    );
    let mut scan = DuplicateScan::new(child);

    for i in span.start..span.end {
        let text = &lines.lines[i].text;
        let number = i + 1;
        let code = without_comment(text);

        if i == span.start && header_flow_map(text) {
            return Err(format!(
                "line {number}: rules is a flow map — write it as a block map first"
            ));
        }

        if let Some(problem) = token_problem(code, number) {
            return Err(problem);
        }

        if i > span.start
            && let Some(key) = scan.duplicate_in(text)
        {
            return Err(format!("line {number}: duplicate key \"{key}\""));
        }
    }

    Ok(())
}

pub(super) fn refusal(text: &str) -> Option<String> {
    let lines = split(text);
    let span = top_level_entry(&lines, "rules")?;
    check_supported(&lines, span).err()
}

struct Migration<'a> {
    lines: &'a Lines,
    plugins: Option<Span>,
    rules: Option<Span>,
    recommended: bool,
}

fn plugin_comments(lines: &Lines, span: Span, indent: usize) -> Vec<String> {
    lines.lines[span.start..span.end]
        .iter()
        .filter_map(|line| trailing_comment(&line.text))
        .map(|comment| format!("{}{comment}", " ".repeat(indent)))
        .collect()
}

fn rules_header(lines: &Lines, rules: Option<Span>) -> Result<String, String> {
    let Some(span) = rules else {
        return Ok("rules:".to_string());
    };
    let text = &lines.lines[span.start].text;
    let value = without_comment(text)
        .split_once(':')
        .map_or("", |(_, value)| value.trim());
    if value.is_empty() {
        return Ok(text.clone());
    }

    if value != "{}" {
        return Err(format!("line {}: rules is not a block map", span.start + 1));
    }

    Ok(match trailing_comment(text) {
        Some(comment) => format!("rules: {comment}"),
        None => "rules:".to_string(),
    })
}

fn indented(text: &str, by: usize) -> String {
    if is_blank(text) {
        text.to_string()
    } else {
        format!("{}{text}", " ".repeat(by))
    }
}

fn group_lines(lines: &Lines, found: &[Entry], group: Group, child: usize) -> Vec<String> {
    let members: Vec<&Entry> = found
        .iter()
        .filter(|entry| v2_location(&entry.key).0 == group)
        .collect();
    if members.is_empty() {
        return Vec::new();
    }

    let mut out = vec![format!("{}{}:", " ".repeat(child), group.key())];

    for entry in members {
        let short = v2_location(&entry.key).1;

        for &i in &entry.lines {
            let text = &lines.lines[i].text;
            let text = if i == entry.key_line {
                renamed_key_line(text, short)
            } else {
                text.clone()
            };
            out.push(indented(&text, child));
        }
    }

    out
}

impl Migration<'_> {
    fn new_block(&self) -> Result<Option<Vec<String>>, String> {
        let body = self.rules.map(|span| Span {
            start: span.start + 1,
            end: span.end,
        });
        let (found_child, found) = match body {
            Some(body) => entries(self.lines, body),
            None => (None, Vec::new()),
        };
        let child = found_child.unwrap_or(2);
        let mut out = Vec::new();

        if self.recommended {
            if let Some(span) = self.plugins {
                out.extend(plugin_comments(self.lines, span, child));
            }

            out.push(format!("{}recommended: true", " ".repeat(child)));
        }

        out.extend(group_lines(self.lines, &found, Group::Core, child));
        out.extend(group_lines(self.lines, &found, Group::Sonar, child));

        if out.is_empty() {
            return Ok(None);
        }

        if found.is_empty()
            && let Some(body) = body
        {
            out.extend(
                self.lines.lines[body.start..body.end]
                    .iter()
                    .map(|line| line.text.clone()),
            );
        }

        let mut block = vec![rules_header(self.lines, self.rules)?];
        block.extend(out);
        Ok(Some(block))
    }
}

fn assemble(migration: &Migration, block: Option<Vec<String>>, eol: &str) -> Vec<Line> {
    let fresh = |text: String| Line {
        text,
        terminator: eol.to_string(),
    };
    let original = &migration.lines.lines;
    let mut out: Vec<Line> = Vec::new();
    let mut block = block;

    for (i, line) in original.iter().enumerate() {
        if migration.plugins.is_some_and(|span| span.contains(i)) {
            if migration.rules.is_none() && migration.plugins.is_some_and(|span| span.start == i) {
                out.extend(block.take().into_iter().flatten().map(fresh));
            }

            continue;
        }

        match migration.rules {
            Some(span) if span.start == i => match block.take() {
                Some(lines) => out.extend(lines.into_iter().map(fresh)),
                None => out.extend(original[span.start..span.end].iter().map(copy_line)),
            },
            Some(span) if span.contains(i) => {}
            _ => out.push(copy_line(line)),
        }
    }

    out
}

fn copy_line(line: &Line) -> Line {
    Line {
        text: line.text.clone(),
        terminator: line.terminator.clone(),
    }
}

fn settle_terminators(out: &mut [Line], original: &Lines) {
    let unterminated = original
        .lines
        .last()
        .is_some_and(|line| line.terminator.is_empty());
    let last = out.len().saturating_sub(1);

    for (i, line) in out.iter_mut().enumerate() {
        if i == last && unterminated {
            line.terminator.clear();
        } else if line.terminator.is_empty() {
            line.terminator = original.eol.to_string();
        }
    }
}

pub(super) fn migrate_yaml(text: &str) -> Result<String, String> {
    let lines = split(text);
    let plugins = top_level_entry(&lines, "plugins");
    let rules = top_level_entry(&lines, "rules");
    if let Some(span) = rules {
        check_supported(&lines, span)?;
    }

    let recommended = match plugins {
        Some(span) => !plugin_names(&lines, span)?.is_empty(),
        None => false,
    };
    let migration = Migration {
        lines: &lines,
        plugins,
        rules,
        recommended,
    };
    let block = migration.new_block()?;
    let mut out = assemble(&migration, block, lines.eol);
    settle_terminators(&mut out, &lines);
    Ok(join(&Lines {
        bom: lines.bom,
        eol: lines.eol,
        lines: out,
    }))
}
