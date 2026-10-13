use std::collections::HashMap;

pub struct OptionSpec {
    pub name: &'static str,
    pub description: String,
    pub value: Option<&'static str>,
    pub short: Option<char>,
    pub aliases: &'static [&'static str],
}

impl OptionSpec {
    pub fn flag(name: &'static str, description: impl Into<String>) -> Self {
        Self {
            name,
            description: description.into(),
            value: None,
            short: None,
            aliases: &[],
        }
    }

    pub fn valued(name: &'static str, value: &'static str, description: impl Into<String>) -> Self {
        Self {
            value: Some(value),
            ..Self::flag(name, description)
        }
    }

    pub fn short(self, short: char) -> Self {
        Self {
            short: Some(short),
            ..self
        }
    }

    pub fn aliases(self, aliases: &'static [&'static str]) -> Self {
        Self { aliases, ..self }
    }

    fn matches(&self, token: &str) -> bool {
        token.strip_prefix("--") == Some(self.name)
            || self.short.is_some_and(|short| {
                token.strip_prefix('-') == Some(short.encode_utf8(&mut [0; 4]))
            })
            || self.aliases.contains(&token)
    }

    fn label(&self) -> String {
        let short = self.short.map(|s| format!("-{s}, ")).unwrap_or_default();
        let value = self.value.map(|v| format!(" {v}")).unwrap_or_default();
        format!("{short}--{}{value}", self.name)
    }
}

pub struct Leaf {
    pub summary: &'static str,
    pub options: Vec<OptionSpec>,
    pub details: Vec<String>,
}

pub struct Group {
    pub summary: &'static str,
    pub commands: Vec<(&'static str, Leaf)>,
    pub aliases: &'static [(&'static str, &'static str)],
    pub default_command: Option<Leaf>,
    pub details: Vec<String>,
}

#[derive(Default)]
pub struct Values(HashMap<&'static str, Option<String>>);

impl Values {
    pub fn flag(&self, name: &str) -> bool {
        self.0.contains_key(name)
    }

    pub fn text(&self, name: &str) -> Option<&str> {
        self.0.get(name).and_then(|value| value.as_deref())
    }
}

pub enum Dispatch {
    Run {
        command: Option<&'static str>,
        values: Values,
    },
    Exit(i32),
}

const HELP_WORD: &str = "help";

fn is_help_flag(token: &str) -> bool {
    token == "-h" || token == "--help"
}

#[derive(Default)]
struct Parsed {
    values: Values,
    positionals: Vec<String>,
    help: bool,
    error: Option<String>,
    rest: Vec<String>,
}

fn parse_options(args: &[String], options: &[OptionSpec], stop_at_positional: bool) -> Parsed {
    let mut parsed = Parsed::default();
    let mut i = 0;
    while i < args.len() {
        let token = args[i].as_str();
        let matched = options.iter().find(|spec| spec.matches(token));
        if is_help_flag(token) {
            parsed.help = true;
        } else if let Some(spec) = matched.filter(|spec| spec.value.is_some()) {
            match args.get(i + 1) {
                Some(value) => {
                    parsed.values.0.insert(spec.name, Some(value.clone()));
                }
                None => {
                    let needs = spec.value.unwrap_or_default();
                    parsed
                        .error
                        .get_or_insert(format!("option '--{}' needs {needs}", spec.name));
                }
            }
            i += 1;
        } else if let Some(spec) = matched {
            parsed.values.0.insert(spec.name, None);
        } else if token.starts_with('-') {
            parsed
                .error
                .get_or_insert(format!("unknown option '{token}'"));
        } else if stop_at_positional {
            parsed.positionals.push(token.to_string());
            parsed.rest = args[i + 1..].to_vec();
            return parsed;
        } else {
            parsed.positionals.push(token.to_string());
        }
        i += 1;
    }
    parsed
}

pub fn dispatch(group: &Group, args: &[String], path: &str) -> Dispatch {
    let help = group_help(group, path);
    let default_options = group
        .default_command
        .as_ref()
        .map_or(&[][..], |leaf| &leaf.options[..]);
    let parsed = parse_options(args, default_options, true);
    let name = parsed.positionals.first().map(String::as_str);
    let resolved = name.map(|name| alias_target(group, name).unwrap_or(name));

    if parsed.help || resolved == Some(HELP_WORD) {
        return show_help(&help);
    }
    if let Some(resolved) = resolved {
        let Some((command, leaf)) = group.commands.iter().find(|(n, _)| *n == resolved) else {
            return usage_failure(
                &format!("unknown command '{}'", name.unwrap_or_default()),
                &help,
            );
        };
        if let Some(error) = &parsed.error {
            return usage_failure(error, &help);
        }
        let leaf_help = leaf_help(leaf, &format!("{path} {command}"));
        return dispatch_leaf(leaf, &parsed.rest, &leaf_help, Some(command));
    }
    match &group.default_command {
        Some(leaf) => dispatch_leaf(leaf, args, &help, None),
        None => show_help(&help),
    }
}

fn alias_target(group: &Group, name: &str) -> Option<&'static str> {
    group
        .aliases
        .iter()
        .find(|(alias, _)| *alias == name)
        .map(|(_, target)| *target)
}

fn dispatch_leaf(
    leaf: &Leaf,
    args: &[String],
    help: &str,
    command: Option<&'static str>,
) -> Dispatch {
    let parsed = parse_options(args, &leaf.options, false);
    if parsed.help || parsed.positionals.iter().any(|p| p == HELP_WORD) {
        return show_help(help);
    }
    if let Some(error) = &parsed.error {
        return usage_failure(error, help);
    }
    if let Some(extra) = parsed.positionals.first() {
        return usage_failure(&format!("unexpected argument '{extra}'"), help);
    }
    Dispatch::Run {
        command,
        values: parsed.values,
    }
}

fn show_help(help: &str) -> Dispatch {
    println!("{help}");
    Dispatch::Exit(0)
}

fn usage_failure(message: &str, help: &str) -> Dispatch {
    eprintln!("error: {message}\n\n{help}");
    Dispatch::Exit(1)
}

fn option_rows(options: &[OptionSpec]) -> Vec<String> {
    let help = OptionSpec::flag("help", "Show this help").short('h');
    table(
        options
            .iter()
            .chain(std::iter::once(&help))
            .map(|spec| (spec.label(), spec.description.clone()))
            .collect(),
    )
}

fn leaf_help(leaf: &Leaf, path: &str) -> String {
    let mut lines = vec![
        format!("Usage: {path} [options]"),
        String::new(),
        format!("  {}", leaf.summary),
        String::new(),
        "Options:".to_string(),
    ];
    lines.extend(option_rows(&leaf.options));
    lines.extend(detail_lines(&leaf.details));
    lines.join("\n")
}

fn group_help(group: &Group, path: &str) -> String {
    let fallback = group.default_command.as_ref();
    let synopsis = if fallback.is_some() {
        "[<command>] [options]"
    } else {
        "<command>"
    };
    let mut lines = vec![
        format!("Usage: {path} {synopsis}"),
        String::new(),
        format!("  {}", group.summary),
        String::new(),
        "Commands:".to_string(),
    ];
    lines.extend(table(
        group
            .commands
            .iter()
            .map(|(name, leaf)| (name.to_string(), leaf.summary.to_string()))
            .collect(),
    ));
    if let Some(leaf) = fallback {
        lines.extend([String::new(), "Options:".to_string()]);
        lines.extend(option_rows(&leaf.options));
    }
    lines.push(String::new());
    if let Some(leaf) = fallback {
        lines.push(format!("With no command, {}.", leaf.summary.to_lowercase()));
    }
    lines.push(format!("Run '{path} <command> --help' for its options."));
    lines.extend(detail_lines(&group.details));
    lines.join("\n")
}

fn detail_lines(details: &[String]) -> Vec<String> {
    if details.is_empty() {
        return Vec::new();
    }
    std::iter::once(String::new())
        .chain(details.iter().cloned())
        .collect()
}

fn table(rows: Vec<(String, String)>) -> Vec<String> {
    let width = rows.iter().map(|(label, _)| label.len()).max().unwrap_or(0);
    rows.into_iter()
        .map(|(label, text)| format!("  {label:width$}  {text}"))
        .collect()
}
