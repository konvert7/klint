use super::grammar::{Grammar, Target, node_text};
use super::items::Item;
use super::keyword::Keyword;
use tree_sitter::Node;

pub(super) fn setup_applies(
    previous: &Item<'_>,
    target: &Target<'_>,
    grammar: &dyn Grammar,
    source: &[u8],
) -> bool {
    if !matches!(
        target.keyword,
        Keyword::If | Keyword::Guard | Keyword::Switch
    ) {
        return false;
    }
    let assigned = grammar.assigned_names(previous.node, source);
    !assigned.is_empty()
        && condition_names(target, grammar, source)
            .iter()
            .any(|name| assigned.contains(name))
}

fn condition_names(target: &Target<'_>, grammar: &dyn Grammar, source: &[u8]) -> Vec<String> {
    target
        .conditions
        .iter()
        .flat_map(|condition| subtree(*condition))
        .filter(|node| grammar.is_condition_name(*node))
        .map(|node| node_text(node, source))
        .collect()
}

fn subtree(node: Node<'_>) -> Vec<Node<'_>> {
    let mut nodes = vec![node];
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        nodes.extend(subtree(child));
    }
    nodes
}

#[cfg(test)]
mod tests {
    use super::super::grammar::grammar_for;
    use super::super::items::{ItemRole, block_items};
    use super::super::lines::SourceLines;
    use super::*;
    use crate::syntax::{SourceLanguage, parse_source};
    use std::path::Path;

    fn setup_applies_in(source: &str) -> bool {
        let tree = parse_source(Path::new("a.ts"), source).expect("source parses");
        let lines = SourceLines::new(source);
        let grammar = grammar_for(SourceLanguage::JavaScriptLike);
        let items = block_items(tree.root_node(), grammar, &lines);
        let target_index = items
            .iter()
            .rposition(|item| matches!(item.role, ItemRole::Statement(Some(_))))
            .expect("a target statement");
        let ItemRole::Statement(Some(target)) = &items[target_index].role else {
            panic!("target statement expected");
        };
        let previous = items[..target_index]
            .iter()
            .rev()
            .find(|item| matches!(item.role, ItemRole::Statement(_)))
            .expect("a previous statement");
        setup_applies(previous, target, grammar, source.as_bytes())
    }

    #[test]
    fn if_on_the_assigned_name_is_exempt() {
        assert!(setup_applies_in("x = f();\nif (x) { }\n"));
    }

    #[test]
    fn longer_name_containing_the_assigned_name_is_not_exempt() {
        assert!(!setup_applies_in("xy = f();\nif (x) { }\n"));
    }

    #[test]
    fn while_on_the_assigned_name_is_not_exempt() {
        assert!(!setup_applies_in("x = f();\nwhile (x) { }\n"));
    }

    #[test]
    fn member_assignment_is_not_exempt() {
        assert!(!setup_applies_in("self.x = f();\nif (self.x) { }\n"));
    }

    #[test]
    fn name_used_only_inside_the_body_is_not_exempt() {
        assert!(!setup_applies_in("x = f();\nif (z) { x; }\n"));
    }

    #[test]
    fn switch_on_a_declared_name_is_exempt() {
        assert!(setup_applies_in("const y = f();\nswitch (y) { }\n"));
    }

    #[test]
    fn comment_between_assignment_and_if_is_skipped() {
        assert!(setup_applies_in("x = f();\n// c\nif (x) { }\n"));
    }
}
