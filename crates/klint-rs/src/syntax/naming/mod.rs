mod case;
mod rename;
mod targets;

use std::ops::Range;

use tree_sitter::Node;

use crate::syntax::SourceLanguage;
use case::{Format, convert, is_in_any};
use targets::{Convention, convention_of, is_renamable_declaration, name_node};

pub(crate) struct NamingFinding<'a> {
    pub name: Node<'a>,
    pub message: String,
    pub rename: Option<Rename>,
}

pub(crate) struct Rename {
    pub edits: Vec<(Range<usize>, String)>,
}

const UNITTEST_HOOKS: &[&str] = &[
    "setUp",
    "tearDown",
    "setUpClass",
    "tearDownClass",
    "setUpModule",
    "tearDownModule",
    "asyncSetUp",
    "asyncTearDown",
    "setUpTestData",
];

pub(crate) fn declares_checked_name(node: Node<'_>, language: SourceLanguage) -> bool {
    convention_of(node, language).is_some()
}

pub(crate) fn naming_finding<'a>(
    node: Node<'a>,
    source: &[u8],
    language: SourceLanguage,
) -> Option<NamingFinding<'a>> {
    let convention = convention_of(node, language)?;
    let name = name_node(node)?;
    let text = name.utf8_text(source).ok()?;
    let parts = NameParts::split(text, language)?;

    if follows(parts.core, convention) {
        return None;
    }

    let suggestion = parts.suggestion(convention);
    let rename = suggestion
        .as_ref()
        .filter(|_| language == SourceLanguage::JavaScriptLike && is_renamable_declaration(node))
        .and_then(|new_name| rename::same_file(node, text, new_name, source));

    Some(NamingFinding {
        name,
        message: message(convention, text, suggestion.as_deref()),
        rename,
    })
}

struct NameParts<'t> {
    prefix: &'t str,
    core: &'t str,
    suffix: &'t str,
}

impl<'t> NameParts<'t> {
    fn split(text: &'t str, language: SourceLanguage) -> Option<Self> {
        if language == SourceLanguage::Python && UNITTEST_HOOKS.contains(&text) {
            return None;
        }

        let (outer_start, outer_end) = trimmable(language);

        if language == SourceLanguage::JavaScriptLike && text.starts_with('_') {
            return None;
        }

        let body = match language {
            SourceLanguage::Rust => text.strip_prefix("r#").unwrap_or(text),
            _ => text,
        };
        let without_start = body.trim_start_matches(outer_start);
        let core = without_start.trim_end_matches(outer_end);
        let prefix = &text[..text.len() - without_start.len()];
        let suffix = &without_start[core.len()..];

        let first = core.chars().next()?;
        let checkable = language == SourceLanguage::JavaScriptLike || first.is_alphabetic();

        checkable.then_some(Self {
            prefix,
            core,
            suffix,
        })
    }

    fn suggestion(&self, convention: Convention) -> Option<String> {
        let format = target_format(convention, self.core);
        let converted = convert(self.core, format);

        (converted != self.core && follows(&converted, convention))
            .then(|| format!("{}{converted}{}", self.prefix, self.suffix))
    }
}

fn follows(name: &str, convention: Convention) -> bool {
    (name.is_ascii() || !convention.ascii_only)
        && is_in_any(name, convention.formats, convention.strict)
}

fn trimmable(language: SourceLanguage) -> (&'static [char], &'static [char]) {
    match language {
        SourceLanguage::JavaScriptLike => (&['$', '_'], &['$', '_']),
        SourceLanguage::CSharp => (&['@', '_'], &['_']),
        SourceLanguage::Swift => (&['`', '_'], &['`', '_']),
        SourceLanguage::Python | SourceLanguage::Rust => (&['_'], &['_']),
    }
}

fn target_format(convention: Convention, core: &str) -> Format {
    match convention.formats {
        [only] => *only,
        _ if core.starts_with(char::is_uppercase) => Format::Pascal,
        _ => Format::Camel,
    }
}

fn message(convention: Convention, name: &str, suggestion: Option<&str>) -> String {
    let formats = format_list(convention.formats);
    let label = convention.label;

    match suggestion {
        Some(new_name) => {
            format!("{label} name `{name}` should be {formats} — rename it to `{new_name}`.")
        }
        None => format!("{label} name `{name}` should be {formats}."),
    }
}

fn format_list(formats: &[Format]) -> String {
    let labels: Vec<&str> = formats.iter().map(|format| format.label()).collect();

    match labels.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} or {last}", rest.join(", ")),
        _ => labels.concat(),
    }
}
