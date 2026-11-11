use super::grammar::{Grammar, Role, Target};
use super::lines::{SourceLines, last_row};
use tree_sitter::Node;

pub(super) enum ItemRole<'a> {
    Comment,
    Statement(Option<Target<'a>>),
}

pub(super) struct Item<'a> {
    pub role: ItemRole<'a>,
    pub node: Node<'a>,
    pub start_row: usize,
    pub start_column: usize,
    pub last_row: usize,
    pub head_row: usize,
    pub head_column: usize,
}

pub(super) fn block_items<'a>(
    container: Node<'a>,
    grammar: &dyn Grammar,
    lines: &SourceLines<'_>,
) -> Vec<Item<'a>> {
    let mut items = Vec::new();
    let mut pending_start: Option<(usize, usize)> = None;
    for member in body_members(container, grammar) {
        match grammar.classify(member) {
            Role::Prefix => {
                pending_start.get_or_insert_with(|| start_of(member));
            }
            Role::Comment => items.push(new_item(ItemRole::Comment, member, start_of(member))),
            Role::Statement(target) => {
                let start = pending_start.take().unwrap_or_else(|| start_of(member));
                items.push(new_item(ItemRole::Statement(target), member, start));
            }
        }
    }
    attach_leading_comments(&mut items, lines);
    items
}

fn body_members<'a>(container: Node<'a>, grammar: &dyn Grammar) -> Vec<Node<'a>> {
    let mut cursor = container.walk();
    let mut members = Vec::new();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if grammar.is_body_member(child, cursor.field_name()) {
                members.push(child);
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    members
}

fn new_item<'a>(role: ItemRole<'a>, node: Node<'a>, start: (usize, usize)) -> Item<'a> {
    Item {
        role,
        node,
        start_row: start.0,
        start_column: start.1,
        last_row: last_row(node),
        head_row: start.0,
        head_column: start.1,
    }
}

fn start_of(node: Node<'_>) -> (usize, usize) {
    let position = node.start_position();
    (position.row, position.column)
}

fn attach_leading_comments(items: &mut [Item<'_>], lines: &SourceLines<'_>) {
    for index in 0..items.len() {
        if matches!(items[index].role, ItemRole::Statement(_)) {
            let (row, column) = leading_comment_head(items, index, lines);
            items[index].head_row = row;
            items[index].head_column = column;
        }
    }
}

fn leading_comment_head(
    items: &[Item<'_>],
    index: usize,
    lines: &SourceLines<'_>,
) -> (usize, usize) {
    let mut head = (items[index].start_row, items[index].start_column);
    let mut position = index;
    while position > 0 && is_leading_comment(&items[position - 1], head.0, lines) {
        position -= 1;
        head = (items[position].start_row, items[position].start_column);
    }
    head
}

fn is_leading_comment(item: &Item<'_>, next_row: usize, lines: &SourceLines<'_>) -> bool {
    matches!(item.role, ItemRole::Comment)
        && lines.starts_line(item.start_row, item.start_column)
        && item.last_row + 1 == next_row
}

#[cfg(test)]
mod tests {
    use super::super::grammar::grammar_for;
    use super::super::lines::SourceLines;
    use super::*;
    use crate::syntax::{SourceLanguage, parse_source};
    use std::path::Path;

    fn role_label(role: &ItemRole<'_>) -> &'static str {
        match role {
            ItemRole::Comment => "comment",
            ItemRole::Statement(None) => "statement",
            ItemRole::Statement(Some(target)) => target.written,
        }
    }

    fn describe_items(source: &str) -> Vec<(&'static str, usize, usize, usize)> {
        let tree = parse_source(Path::new("a.ts"), source).expect("source parses");
        let lines = SourceLines::new(source);
        let grammar = grammar_for(SourceLanguage::JavaScriptLike);
        block_items(tree.root_node(), grammar, &lines)
            .iter()
            .map(|item| {
                (
                    role_label(&item.role),
                    item.start_row,
                    item.last_row,
                    item.head_row,
                )
            })
            .collect()
    }

    #[test]
    fn if_after_a_statement_spans_its_braces() {
        assert_eq!(
            describe_items("foo();\nif (a) {\n}\n"),
            vec![("statement", 0, 0, 0), ("if", 1, 2, 1)]
        );
    }

    #[test]
    fn leading_comment_moves_the_if_head_up() {
        assert_eq!(
            describe_items("foo();\n// c\nif (a) {}\n"),
            vec![
                ("statement", 0, 0, 0),
                ("comment", 1, 1, 1),
                ("if", 2, 2, 1)
            ]
        );
    }

    #[test]
    fn blank_line_between_comment_and_if_breaks_the_head() {
        assert_eq!(
            describe_items("foo();\n// c\n\nif (a) {}\n"),
            vec![
                ("statement", 0, 0, 0),
                ("comment", 1, 1, 1),
                ("if", 3, 3, 3)
            ]
        );
    }

    #[test]
    fn comment_sharing_a_line_does_not_move_the_if_head() {
        assert_eq!(
            describe_items("foo(); // c\nif (a) {}\n"),
            vec![
                ("statement", 0, 0, 0),
                ("comment", 0, 0, 0),
                ("if", 1, 1, 1)
            ]
        );
    }
}
