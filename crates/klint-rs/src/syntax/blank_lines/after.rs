use super::BlankLineFinding;
use super::grammar::Grammar;
use super::items::{Item, ItemRole, closing_bytes};
use super::keyword::{Keyword, KeywordSet};
use super::lines::SourceLines;

pub(super) fn after_findings(
    items: &[Item<'_>],
    lines: &SourceLines<'_>,
    grammar: &dyn Grammar,
    targets: KeywordSet,
    guards: bool,
) -> Vec<BlankLineFinding> {
    let is_guard = |item: &Item<'_>| guards && is_exiting_guard(item, grammar);

    items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            let ItemRole::Statement(Some(target)) = &item.role else {
                return None;
            };
            let next = next_statement(&items[index + 1..])?;
            let guard = is_guard(item);
            let closes_paragraph = target.ends_with_block || (guard && !is_guard(next));
            (targets.contains(target.keyword)
                && closes_paragraph
                && next.start_row > item.last_row
                && !lines.separates(item.last_row + 1, grammar))
            .then(|| finding(item, target.written, guard))
        })
        .collect()
}

fn is_exiting_guard(item: &Item<'_>, grammar: &dyn Grammar) -> bool {
    matches!(&item.role, ItemRole::Statement(Some(target)) if target.keyword == Keyword::If)
        && grammar.is_exiting_guard(item.node)
}

fn next_statement<'i, 'a>(items: &'i [Item<'a>]) -> Option<&'i Item<'a>> {
    items
        .iter()
        .find(|item| matches!(item.role, ItemRole::Statement(_)))
}

fn finding(item: &Item<'_>, written: &'static str, guard: bool) -> BlankLineFinding {
    BlankLineFinding {
        line: item.last_row + 1,
        insertion_line: item.last_row + 2,
        keyword: written,
        guard,
        bytes: closing_bytes(item),
    }
}

#[cfg(test)]
mod tests {
    use super::super::grammar::grammar_for;
    use super::super::items::block_items;
    use super::super::keyword::KeywordSet;
    use super::super::lines::SourceLines;
    use super::super::{BlankLineFinding, without_bytes};
    use super::*;
    use crate::syntax::{parse_source, source_language_for_path};
    use std::path::Path;

    fn findings_with_bytes(path: &str, source: &str) -> Vec<BlankLineFinding> {
        let path = Path::new(path);
        let tree = parse_source(path, source).expect("source parses");
        let lines = SourceLines::new(source);
        let grammar = grammar_for(source_language_for_path(path));
        let items = block_items(tree.root_node(), grammar, &lines);
        after_findings(&items, &lines, grammar, KeywordSet::ALL_AFTER, false)
    }

    fn after_findings_in(source: &str) -> Vec<BlankLineFinding> {
        without_bytes(findings_with_bytes("a.ts", source))
    }

    fn reported_text<'s>(path: &str, source: &'s str) -> Vec<Option<&'s str>> {
        findings_with_bytes(path, source)
            .into_iter()
            .map(|finding| finding.bytes.map(|bytes| &source[bytes]))
            .collect()
    }

    #[test]
    fn the_report_points_at_the_token_that_closes_the_block() {
        assert_eq!(
            reported_text("a.ts", "if (a) {\n}\nfoo();\n"),
            vec![Some("}")]
        );
    }

    #[test]
    fn a_python_block_points_at_its_last_token() {
        assert_eq!(
            reported_text("a.py", "if a:\n    b()\nc()\n"),
            vec![Some(")")]
        );
    }

    fn finding(line: usize, insertion_line: usize, keyword: &'static str) -> BlankLineFinding {
        BlankLineFinding {
            line,
            insertion_line,
            keyword,
            guard: false,
            bytes: None,
        }
    }

    #[test]
    fn block_directly_above_a_statement_is_reported() {
        assert_eq!(
            after_findings_in("if (a) {\n}\nfoo();\n"),
            vec![finding(2, 3, "if")]
        );
    }

    #[test]
    fn block_above_a_comment_is_reported_above_the_comment() {
        assert_eq!(
            after_findings_in("if (a) {\n}\n// c\nfoo();\n"),
            vec![finding(2, 3, "if")]
        );
    }

    #[test]
    fn block_with_no_following_statement_is_not_reported() {
        assert!(after_findings_in("if (a) {\n}\n// only\n").is_empty());
    }

    #[test]
    fn do_while_is_not_reported() {
        assert!(after_findings_in("do {\n} while (a);\nfoo();\n").is_empty());
    }

    #[test]
    fn braceless_if_is_not_reported() {
        assert!(after_findings_in("if (a) b();\nfoo();\n").is_empty());
    }

    #[test]
    fn block_and_next_statement_on_one_line_are_not_reported() {
        assert!(after_findings_in("if (a) { } foo();\n").is_empty());
    }

    #[test]
    fn whitespace_line_below_the_block_counts_as_blank() {
        assert!(after_findings_in("if (a) {\n}\n  \nfoo();\n").is_empty());
    }
}
