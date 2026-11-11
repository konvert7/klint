use super::BlankLineFinding;
use super::items::{Item, ItemRole};
use super::keyword::KeywordSet;
use super::lines::SourceLines;

pub(super) fn after_findings(
    items: &[Item<'_>],
    lines: &SourceLines<'_>,
    targets: KeywordSet,
) -> Vec<BlankLineFinding> {
    items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            let ItemRole::Statement(Some(target)) = &item.role else {
                return None;
            };
            let next = next_statement(&items[index + 1..])?;
            (targets.contains(target.keyword)
                && target.ends_with_block
                && next.start_row > item.last_row
                && !lines.is_blank(item.last_row + 1))
            .then(|| finding(item, target.written))
        })
        .collect()
}

fn next_statement<'i, 'a>(items: &'i [Item<'a>]) -> Option<&'i Item<'a>> {
    items
        .iter()
        .find(|item| matches!(item.role, ItemRole::Statement(_)))
}

fn finding(item: &Item<'_>, written: &'static str) -> BlankLineFinding {
    BlankLineFinding {
        line: item.last_row + 1,
        insertion_line: item.last_row + 2,
        keyword: written,
    }
}

#[cfg(test)]
mod tests {
    use super::super::BlankLineFinding;
    use super::super::grammar::grammar_for;
    use super::super::items::block_items;
    use super::super::keyword::KeywordSet;
    use super::super::lines::SourceLines;
    use super::*;
    use crate::syntax::{SourceLanguage, parse_source};
    use std::path::Path;

    fn after_findings_in(source: &str) -> Vec<BlankLineFinding> {
        let tree = parse_source(Path::new("a.ts"), source).expect("source parses");
        let lines = SourceLines::new(source);
        let grammar = grammar_for(SourceLanguage::JavaScriptLike);
        let items = block_items(tree.root_node(), grammar, &lines);
        after_findings(&items, &lines, KeywordSet::ALL_AFTER)
    }

    fn finding(line: usize, insertion_line: usize, keyword: &'static str) -> BlankLineFinding {
        BlankLineFinding {
            line,
            insertion_line,
            keyword,
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
