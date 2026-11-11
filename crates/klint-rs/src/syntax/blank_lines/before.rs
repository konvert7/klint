use super::BlankLineFinding;
use super::grammar::Grammar;
use super::items::{Item, ItemRole};
use super::keyword::KeywordSet;
use super::lines::SourceLines;
use super::setup::setup_applies;

pub(super) fn before_findings(
    items: &[Item<'_>],
    lines: &SourceLines<'_>,
    grammar: &dyn Grammar,
    source: &[u8],
    targets: KeywordSet,
    allow_setup: bool,
) -> Vec<BlankLineFinding> {
    items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            let ItemRole::Statement(Some(target)) = &item.role else {
                return None;
            };
            let previous = previous_statement(&items[..index])?;
            (targets.contains(target.keyword)
                && is_reportable_gap(previous, item, lines, grammar)
                && !(allow_setup && setup_applies(previous, target, grammar, source)))
            .then(|| finding(item, target.written))
        })
        .collect()
}

fn previous_statement<'i, 'a>(items: &'i [Item<'a>]) -> Option<&'i Item<'a>> {
    items
        .iter()
        .rev()
        .find(|item| matches!(item.role, ItemRole::Statement(_)))
}

fn is_reportable_gap(
    previous: &Item<'_>,
    item: &Item<'_>,
    lines: &SourceLines<'_>,
    grammar: &dyn Grammar,
) -> bool {
    item.head_row > previous.last_row
        && lines.starts_line(item.start_row, item.start_column)
        && lines.starts_line(item.head_row, item.head_column)
        && !lines.separates(item.head_row - 1, grammar)
}

fn finding(item: &Item<'_>, written: &'static str) -> BlankLineFinding {
    BlankLineFinding {
        line: item.start_row + 1,
        insertion_line: item.head_row + 1,
        keyword: written,
    }
}

#[cfg(test)]
mod tests {
    use super::super::BlankLineFinding;
    use super::super::grammar::grammar_for;
    use super::super::items::block_items;
    use super::super::keyword::{Keyword, KeywordSet};
    use super::super::lines::SourceLines;
    use super::*;
    use crate::syntax::{SourceLanguage, parse_source};
    use std::path::Path;

    fn before_findings_in(
        source: &str,
        targets: KeywordSet,
        allow_setup: bool,
    ) -> Vec<BlankLineFinding> {
        let tree = parse_source(Path::new("a.ts"), source).expect("source parses");
        let lines = SourceLines::new(source);
        let grammar = grammar_for(SourceLanguage::JavaScriptLike);
        let items = block_items(tree.root_node(), grammar, &lines);
        before_findings(
            &items,
            &lines,
            grammar,
            source.as_bytes(),
            targets,
            allow_setup,
        )
    }

    fn finding(line: usize, insertion_line: usize, keyword: &'static str) -> BlankLineFinding {
        BlankLineFinding {
            line,
            insertion_line,
            keyword,
        }
    }

    #[test]
    fn if_directly_under_a_statement_is_reported() {
        assert_eq!(
            before_findings_in("foo();\nif (a) {}\n", KeywordSet::ALL_BEFORE, true),
            vec![finding(2, 2, "if")]
        );
    }

    #[test]
    fn for_under_a_comment_is_reported_above_the_comment() {
        assert_eq!(
            before_findings_in("foo();\n// c\nfor (;;) {}\n", KeywordSet::ALL_BEFORE, true),
            vec![finding(3, 2, "for")]
        );
    }

    #[test]
    fn if_after_a_blank_line_is_not_reported() {
        assert!(
            before_findings_in("foo();\n\nif (a) {}\n", KeywordSet::ALL_BEFORE, true).is_empty()
        );
    }

    #[test]
    fn if_on_the_same_line_as_a_statement_is_not_reported() {
        assert!(before_findings_in("foo(); if (a) {}\n", KeywordSet::ALL_BEFORE, true).is_empty());
    }

    #[test]
    fn if_is_not_reported_when_only_return_is_targeted() {
        assert!(
            before_findings_in(
                "foo();\nif (a) {}\n",
                KeywordSet::default().with(Keyword::Return),
                true
            )
            .is_empty()
        );
    }

    #[test]
    fn if_on_an_assigned_name_is_reported_when_setup_is_not_allowed() {
        assert_eq!(
            before_findings_in("x = 1;\nif (x) {}\n", KeywordSet::ALL_BEFORE, false),
            vec![finding(2, 2, "if")]
        );
    }
}
