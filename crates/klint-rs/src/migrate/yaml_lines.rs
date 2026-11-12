pub(super) struct Line {
    pub(super) text: String,
    pub(super) terminator: String,
}

pub(super) struct Lines {
    pub(super) bom: bool,
    pub(super) eol: &'static str,
    pub(super) lines: Vec<Line>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Span {
    pub(super) start: usize,
    pub(super) end: usize,
}

impl Span {
    pub(super) fn contains(self, index: usize) -> bool {
        (self.start..self.end).contains(&index)
    }
}

pub(super) fn split(text: &str) -> Lines {
    let body = text.strip_prefix('\u{feff}');
    let bom = body.is_some();
    let lines: Vec<Line> = body
        .unwrap_or(text)
        .split_inclusive('\n')
        .map(split_terminator)
        .collect();
    let eol = lines
        .iter()
        .find(|line| !line.terminator.is_empty())
        .map_or("\n", |line| terminator_kind(&line.terminator));
    Lines { bom, eol, lines }
}

fn terminator_kind(terminator: &str) -> &'static str {
    if terminator == "\r\n" { "\r\n" } else { "\n" }
}

fn split_terminator(segment: &str) -> Line {
    let (text, terminator) = if let Some(text) = segment.strip_suffix("\r\n") {
        (text, "\r\n")
    } else if let Some(text) = segment.strip_suffix('\n') {
        (text, "\n")
    } else {
        (segment, "")
    };
    Line {
        text: text.to_string(),
        terminator: terminator.to_string(),
    }
}

pub(super) fn join(lines: &Lines) -> String {
    let mut out = String::new();
    if lines.bom {
        out.push('\u{feff}');
    }
    for line in &lines.lines {
        out.push_str(&line.text);
        out.push_str(&line.terminator);
    }
    out
}

pub(super) fn indent_of(text: &str) -> usize {
    text.len() - text.trim_start_matches(' ').len()
}

pub(super) fn is_comment(text: &str) -> bool {
    text.trim_start().starts_with('#')
}

pub(super) fn is_blank(text: &str) -> bool {
    text.trim().is_empty()
}

fn opens_string(previous: Option<char>) -> bool {
    previous.is_none_or(|c| c.is_whitespace() || "[{,:".contains(c))
}

fn blanks(c: char) -> String {
    " ".repeat(c.len_utf8())
}

pub(super) fn masked(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut open: Option<char> = None;
    let mut previous: Option<char> = None;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match open {
            None => {
                out.push(c);
                if (c == '"' || c == '\'') && opens_string(previous) {
                    open = Some(c);
                }
            }
            Some(quote) => {
                let escaped = (quote == '"' && c == '\\') || (quote == '\'' && c == quote);
                if escaped && quote == '\'' && chars.peek() == Some(&'\'') {
                    out.push_str(&blanks(c));
                    out.push_str(&blanks(chars.next().unwrap_or(c)));
                } else if escaped && quote == '"' {
                    out.push_str(&blanks(c));
                    if let Some(next) = chars.next() {
                        out.push_str(&blanks(next));
                    }
                } else if c == quote {
                    out.push(c);
                    open = None;
                } else {
                    out.push_str(&blanks(c));
                }
            }
        }
        previous = Some(c);
    }
    out
}

fn comment_start(text: &str) -> Option<usize> {
    let hidden = masked(text);
    let bytes = hidden.as_bytes();
    (0..bytes.len()).find(|&i| bytes[i] == b'#' && (i == 0 || bytes[i - 1].is_ascii_whitespace()))
}

pub(super) fn trailing_comment(text: &str) -> Option<&str> {
    comment_start(text).map(|i| &text[i..])
}

pub(super) fn without_comment(text: &str) -> &str {
    comment_start(text).map_or(text, |i| text[..i].trim_end())
}

fn is_key_line_of(text: &str, key: &str) -> bool {
    let code = without_comment(text);
    [key.to_string(), format!("\"{key}\""), format!("'{key}'")]
        .iter()
        .filter_map(|spelling| code.strip_prefix(spelling.as_str()))
        .filter_map(|rest| rest.strip_prefix(':'))
        .any(|rest| rest.is_empty() || rest.starts_with(' '))
}

fn ends_top_level_entry(text: &str) -> bool {
    indent_of(text) == 0 && !is_blank(text) && !is_comment(text)
}

pub(super) fn top_level_entry(lines: &Lines, key: &str) -> Option<Span> {
    let start = lines
        .lines
        .iter()
        .position(|line| indent_of(&line.text) == 0 && is_key_line_of(&line.text, key))?;
    let mut end = lines.lines[start + 1..]
        .iter()
        .position(|line| ends_top_level_entry(&line.text))
        .map_or(lines.lines.len(), |offset| start + 1 + offset);
    while end > start + 1 && is_separator_line(&lines.lines[end - 1].text) {
        end -= 1;
    }
    Some(Span { start, end })
}

fn is_separator_line(text: &str) -> bool {
    is_blank(text) || (indent_of(text) == 0 && is_comment(text))
}

pub(super) fn plugin_names(lines: &Lines, span: Span) -> Result<Vec<String>, String> {
    let number = span.start + 1;
    let not_a_list = || format!("line {number}: plugins is not a list of names");
    let source = lines.lines[span.start..span.end]
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let parsed: serde_yaml::Value = serde_yaml::from_str(&source).map_err(|_| not_a_list())?;
    let names = match parsed.get("plugins") {
        Some(serde_yaml::Value::Null) | None => Vec::new(),
        Some(serde_yaml::Value::Sequence(items)) => items
            .iter()
            .map(|item| item.as_str().map(str::to_string).ok_or_else(not_a_list))
            .collect::<Result<Vec<_>, _>>()?,
        Some(_) => return Err(not_a_list()),
    };
    match names.iter().find(|name| name.as_str() != "sonar") {
        Some(name) => Err(format!(
            "line {number}: plugins lists \"{name}\" — only sonar can be migrated"
        )),
        None => Ok(names),
    }
}
