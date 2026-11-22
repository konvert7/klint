mod csharp;
mod grammar;
mod javascript;
mod python;
mod rust;
mod swift;
mod walker;

use std::ops::Range;

use crate::syntax::SourceLanguage;
use grammar::complexity_grammar_for;
use tree_sitter::Node;

#[derive(Clone)]
pub(crate) struct FunctionScore<'a> {
    pub node: Node<'a>,
    pub score: u32,
    leaked_nesting: u32,
}

pub(crate) type ScoringContext<'a> = walker::Context<'a>;

pub(crate) fn score_node<'a>(
    node: Node<'a>,
    context: ScoringContext<'a>,
    language: SourceLanguage,
    scores: &mut Vec<FunctionScore<'a>>,
) -> ScoringContext<'a> {
    walker::visit(node, context, complexity_grammar_for(language), scores)
}

pub(crate) fn sort_by_position(scores: &mut [FunctionScore<'_>]) {
    scores.sort_by_key(|score| {
        let start = score.node.start_position();
        (start.row, start.column)
    });
}

#[cfg(test)]
fn score_functions<'a>(roots: &[Node<'a>], language: SourceLanguage) -> Vec<FunctionScore<'a>> {
    let grammar = complexity_grammar_for(language);
    let mut scores = Vec::new();

    for root in roots {
        walker::score_root(*root, grammar, &mut scores);
    }

    sort_by_position(&mut scores);
    scores
}

pub(crate) fn function_name(function: Node<'_>, source: &[u8], language: SourceLanguage) -> String {
    let grammar = complexity_grammar_for(language);
    grammar
        .declared_name(function, source)
        .or_else(|| grammar.assigned_name(function, source))
        .unwrap_or_else(|| "<anonymous>".to_string())
}

/// Where the function's name is written, when it sits on the line the
/// function starts on, so a report can point at the name.
pub(crate) fn function_name_bytes(function: Node<'_>) -> Option<Range<usize>> {
    let name = declared_name_node(function).or_else(|| assigned_name_node(function))?;

    (name.start_position().row == function.start_position().row).then(|| name.byte_range())
}

fn declared_name_node(function: Node<'_>) -> Option<Node<'_>> {
    function.child_by_field_name("name")
}

fn assigned_name_node(function: Node<'_>) -> Option<Node<'_>> {
    let parent = function.parent()?;
    let field = match parent.kind() {
        "variable_declarator" | "public_field_definition" => "name",
        "assignment_expression" | "assignment" => "left",
        "let_declaration" => "pattern",
        "pair" => "key",
        _ => return None,
    };

    parent.child_by_field_name(field)
}

pub(crate) fn is_scored_function(language: SourceLanguage, kind: &str) -> bool {
    complexity_grammar_for(language)
        .function_kinds()
        .contains(&kind)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::complexity::grammar::testing::{all_of_kind, parse};

    #[test]
    fn scored_kinds_follow_the_language() {
        assert!(is_scored_function(
            SourceLanguage::JavaScriptLike,
            "arrow_function"
        ));
        assert!(!is_scored_function(
            SourceLanguage::JavaScriptLike,
            "method_signature"
        ));
        assert!(!is_scored_function(
            SourceLanguage::Rust,
            "function_signature_item"
        ));
        assert!(is_scored_function(
            SourceLanguage::CSharp,
            "accessor_declaration"
        ));
    }

    #[test]
    fn sibling_functions_are_scored_in_column_order() {
        let source = "const a = (x) => { if (x) { if (x) {} } }; const b = (y) => { if (y) { if (y) { if (y) {} } } };";
        let tree = parse("a.ts", source);
        let arrows = all_of_kind(tree.root_node(), "arrow_function");
        let scores = score_functions(&arrows, SourceLanguage::JavaScriptLike);
        let values: Vec<u32> = scores.iter().map(|score| score.score).collect();
        assert_eq!(values, vec![3, 6]);
    }

    #[test]
    fn declared_name_wins_and_anonymous_is_the_fallback() {
        let source = "function declared(a) {}\n(() => {})();";
        let tree = parse("a.ts", source);
        let declaration = all_of_kind(tree.root_node(), "function_declaration")[0];
        let iife = all_of_kind(tree.root_node(), "arrow_function")[0];
        assert_eq!(
            function_name(
                declaration,
                source.as_bytes(),
                SourceLanguage::JavaScriptLike
            ),
            "declared"
        );
        assert_eq!(
            function_name(iife, source.as_bytes(), SourceLanguage::JavaScriptLike),
            "<anonymous>"
        );
    }

    #[test]
    fn name_bytes_point_at_the_declared_or_assigned_name() {
        let source = "function declared() {}\nconst assigned = () => {};\n(() => {})();";
        let tree = parse("a.ts", source);
        let declaration = all_of_kind(tree.root_node(), "function_declaration")[0];
        let arrows = all_of_kind(tree.root_node(), "arrow_function");
        let text = |bytes: Range<usize>| &source[bytes];

        assert_eq!(function_name_bytes(declaration).map(text), Some("declared"));
        assert_eq!(function_name_bytes(arrows[0]).map(text), Some("assigned"));
        assert_eq!(function_name_bytes(arrows[1]), None);
    }

    #[test]
    fn a_name_on_another_line_gives_no_bytes() {
        let source = "const assigned =\n  () => {};";
        let tree = parse("a.ts", source);
        let arrow = all_of_kind(tree.root_node(), "arrow_function")[0];

        assert_eq!(function_name_bytes(arrow), None);
    }
}
