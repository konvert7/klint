mod tally;

pub(crate) use tally::{LineTally, LineTotals};

use tree_sitter::Node;

use crate::syntax::SourceLanguage;

#[derive(Clone, Copy)]
pub(crate) struct FunctionLength<'a> {
    pub node: Node<'a>,
    pub lines: u32,
    pub non_blank_lines: u32,
}

fn is_length_kind(language: SourceLanguage, kind: &str) -> bool {
    match language {
        SourceLanguage::JavaScriptLike => matches!(
            kind,
            "function_declaration"
                | "function_expression"
                | "generator_function_declaration"
                | "generator_function"
                | "arrow_function"
                | "method_definition"
        ),
        SourceLanguage::Python => matches!(kind, "function_definition" | "lambda"),
        SourceLanguage::Rust => matches!(kind, "function_item" | "closure_expression"),
        SourceLanguage::Swift => matches!(
            kind,
            "function_declaration" | "init_declaration" | "deinit_declaration" | "lambda_literal"
        ),
        SourceLanguage::CSharp => matches!(
            kind,
            "method_declaration"
                | "constructor_declaration"
                | "destructor_declaration"
                | "operator_declaration"
                | "conversion_operator_declaration"
                | "local_function_statement"
                | "lambda_expression"
                | "anonymous_method_expression"
        ),
    }
}

fn single_token_kinds(language: SourceLanguage) -> &'static [&'static str] {
    match language {
        SourceLanguage::JavaScriptLike | SourceLanguage::Python => &["string"],
        SourceLanguage::Rust => &["string_literal", "raw_string_literal", "char_literal"],
        SourceLanguage::Swift => &[
            "line_string_literal",
            "multi_line_string_literal",
            "raw_string_literal",
        ],
        SourceLanguage::CSharp => &[
            "string_literal",
            "verbatim_string_literal",
            "raw_string_literal",
            "interpolated_string_expression",
            "character_literal",
        ],
    }
}

pub(crate) fn is_single_token_kind(language: SourceLanguage, kind: &str) -> bool {
    single_token_kinds(language).contains(&kind)
}

pub(crate) fn starts_uncounted_region(language: SourceLanguage, node: Node<'_>) -> bool {
    node.is_extra()
        || (node.child_count() > 0
            && node.is_named()
            && is_single_token_kind(language, node.kind()))
}

pub(crate) fn is_comment(node: Node<'_>) -> bool {
    node.kind().contains("comment")
}

pub(crate) fn is_token(language: SourceLanguage, node: Node<'_>) -> bool {
    match node.child_count() {
        0 => true,
        _ => node.is_named() && is_single_token_kind(language, node.kind()),
    }
}

fn is_accessor(function: Node<'_>) -> bool {
    if function.kind() != "method_definition" {
        return false;
    }

    let mut cursor = function.walk();
    function
        .children(&mut cursor)
        .any(|child| !child.is_named() && matches!(child.kind(), "get" | "set"))
}

fn child_of_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| child.kind() == kind)
}

fn last_child_of_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .filter(|child| child.kind() == kind)
        .last()
}

fn expression_of_arrow_clause(body: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = body.walk();
    body.named_children(&mut cursor).next()
}

pub(crate) fn measured_body<'a>(language: SourceLanguage, function: Node<'a>) -> Option<Node<'a>> {
    if !function.is_named() || !is_length_kind(language, function.kind()) || is_accessor(function) {
        return None;
    }

    match function.kind() {
        "lambda_literal" => child_of_kind(function, "statements"),
        "anonymous_method_expression" => last_child_of_kind(function, "block"),
        _ => {
            let body = function.child_by_field_name("body")?;
            if body.kind() == "arrow_expression_clause" {
                expression_of_arrow_clause(body)
            } else {
                Some(body)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::{SourceLanguage, parse_source, source_language_for_path};
    use std::path::Path;
    use tree_sitter::{Node, Tree};

    fn parsed(path: &str, source: &str) -> Tree {
        parse_source(Path::new(path), source).expect("snippet parses")
    }

    fn first_of_kind<'a>(root: Node<'a>, kind: &str) -> Node<'a> {
        let mut stack = vec![root];

        while let Some(node) = stack.pop() {
            if node.kind() == kind {
                return node;
            }

            let mut cursor = node.walk();
            let children: Vec<Node<'a>> = node.children(&mut cursor).collect();
            stack.extend(children.into_iter().rev());
        }

        panic!("snippet has no {kind} node");
    }

    fn measured_body_of(path: &str, source: &str, kind: &str) -> Option<(String, String)> {
        let tree = parsed(path, source);
        let function = first_of_kind(tree.root_node(), kind);
        let language = source_language_for_path(Path::new(path));
        measured_body(language, function).map(|body| {
            let text = body
                .utf8_text(source.as_bytes())
                .expect("utf-8 snippet")
                .to_string();
            (body.kind().to_string(), text)
        })
    }

    fn body(kind: &str, text: &str) -> Option<(String, String)> {
        Some((kind.to_string(), text.to_string()))
    }

    #[test]
    fn javascript_string_is_a_single_token_kind() {
        assert!(is_single_token_kind(
            SourceLanguage::JavaScriptLike,
            "string"
        ));
    }

    #[test]
    fn javascript_template_string_is_not_a_single_token_kind() {
        assert!(!is_single_token_kind(
            SourceLanguage::JavaScriptLike,
            "template_string"
        ));
    }

    #[test]
    fn rust_raw_string_literal_is_a_single_token_kind() {
        assert!(is_single_token_kind(
            SourceLanguage::Rust,
            "raw_string_literal"
        ));
    }

    #[test]
    fn csharp_string_is_not_a_single_token_kind() {
        assert!(!is_single_token_kind(SourceLanguage::CSharp, "string"));
    }

    #[test]
    fn function_declaration_is_measured_by_its_block() {
        assert_eq!(
            measured_body_of("a.ts", "function f() {}", "function_declaration"),
            body("statement_block", "{}")
        );
    }

    #[test]
    fn a_getter_is_not_measured() {
        assert_eq!(
            measured_body_of("a.ts", "class C { get g() { a(); } }", "method_definition"),
            None
        );
    }

    #[test]
    fn a_method_named_get_is_measured_by_its_block() {
        assert_eq!(
            measured_body_of("a.ts", "class C { get() { a(); } }", "method_definition"),
            body("statement_block", "{ a(); }")
        );
    }

    #[test]
    fn arrow_function_is_measured_by_its_expression_body() {
        assert_eq!(
            measured_body_of("a.ts", "const f = (x) => x + 1;", "arrow_function"),
            body("binary_expression", "x + 1")
        );
    }

    #[test]
    fn swift_closure_is_measured_by_its_statements() {
        assert_eq!(
            measured_body_of("a.swift", "let f = { a() }", "lambda_literal"),
            body("statements", "a()")
        );
    }

    #[test]
    fn swift_closure_parameters_are_outside_its_statements() {
        assert_eq!(
            measured_body_of(
                "a.swift",
                "let f = { (a: Int,\n b: Int) in\n a() }",
                "lambda_literal"
            ),
            body("statements", "a()")
        );
    }

    #[test]
    fn empty_swift_closure_is_not_measured() {
        assert_eq!(
            measured_body_of("a.swift", "let f = {}", "lambda_literal"),
            None
        );
    }

    #[test]
    fn csharp_expression_bodied_method_is_measured_by_its_expression() {
        assert_eq!(
            measured_body_of("a.cs", "class C { int F() => 1; }", "method_declaration"),
            body("integer_literal", "1")
        );
    }

    #[test]
    fn csharp_abstract_method_without_a_body_is_not_measured() {
        assert_eq!(
            measured_body_of(
                "a.cs",
                "abstract class C { abstract void M(); }",
                "method_declaration"
            ),
            None
        );
    }

    #[test]
    fn rust_trait_method_signature_is_not_measured() {
        assert_eq!(
            measured_body_of(
                "a.rs",
                "trait T { fn m(&self); }",
                "function_signature_item"
            ),
            None
        );
    }

    #[test]
    fn csharp_property_accessor_is_not_measured() {
        assert_eq!(
            measured_body_of(
                "a.cs",
                "class C { int P { get { return 1; } } }",
                "accessor_declaration"
            ),
            None
        );
    }
}
