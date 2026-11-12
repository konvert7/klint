use tree_sitter::Node;

use crate::syntax::{
    SourceLanguage, is_async_function_like, is_function_like, is_scored_function,
    is_statement_container,
};

/// A `call_expression` plus the descent state its rules need. Both
/// `no-unguarded-json-parse` and `no-sync-in-async` decide from an ancestor —
/// a `try_statement` or the nearest function — so the state is captured while
/// descending rather than by re-walking upwards.
pub(crate) struct IndexedCall<'a> {
    pub node: Node<'a>,
    pub inside_try: bool,
    pub in_async: bool,
}

/// Which buckets the configured rules actually read. Filling a bucket nothing
/// reads is wasted work, and `in_async` in particular costs a text read at
/// every function node.
#[derive(Default, Clone, Copy)]
pub(crate) struct Wants {
    pub calls: bool,
    pub inside_try: bool,
    pub in_async: bool,
    pub subscripts: bool,
    pub regexes: bool,
    pub templates: bool,
    pub strings: bool,
    pub new_expressions: bool,
    pub if_statements: bool,
    pub containers: bool,
    pub statement_blocks: bool,
    pub functions: bool,
}

impl Wants {
    pub(crate) fn wants_anything(self) -> bool {
        self.calls
            || self.subscripts
            || self.regexes
            || self.templates
            || self.strings
            || self.new_expressions
            || self.if_statements
            || self.containers
            || self.statement_blocks
            || self.functions
    }
}

/// Every node a configured rule might care about, bucketed by kind in a single
/// pre-order walk. Each bucket is in the same order the per-rule walks visited
/// them, so rules reading a bucket emit exactly the order they used to.
#[derive(Default)]
pub(crate) struct NodeIndex<'a> {
    pub calls: Vec<IndexedCall<'a>>,
    pub subscripts: Vec<Node<'a>>,
    pub regexes: Vec<Node<'a>>,
    pub templates: Vec<Node<'a>>,
    pub strings: Vec<Node<'a>>,
    pub new_expressions: Vec<Node<'a>>,
    pub if_statements: Vec<Node<'a>>,
    pub containers: Vec<Node<'a>>,
    pub statement_blocks: Vec<Node<'a>>,
    pub functions: Vec<Node<'a>>,
    pub has_syntax_error: bool,
}

impl<'a> NodeIndex<'a> {
    pub(crate) fn build(
        root: Node<'a>,
        source: &[u8],
        wants: Wants,
        language: SourceLanguage,
    ) -> Self {
        let mut index = Self {
            has_syntax_error: root.has_error(),
            ..Self::default()
        };
        if wants.wants_anything() {
            index.visit(root, source, wants, language, Descent::default());
        }
        index
    }

    fn visit(
        &mut self,
        node: Node<'a>,
        source: &[u8],
        wants: Wants,
        language: SourceLanguage,
        descent: Descent,
    ) {
        if wants.functions && !descent.in_function && is_scored_function(language, node.kind()) {
            self.functions.push(node);
        }
        let descent = descent.enter(node, source, wants, language);
        if wants.statement_blocks && is_statement_container(language, node.kind()) {
            self.statement_blocks.push(node);
        }
        match node.kind() {
            "call_expression" if wants.calls => self.calls.push(IndexedCall {
                node,
                inside_try: descent.inside_try,
                in_async: descent.in_async,
            }),
            "subscript_expression" if wants.subscripts => self.subscripts.push(node),
            "regex" if wants.regexes => self.regexes.push(node),
            "template_string" if wants.templates => self.templates.push(node),
            "string" if wants.strings => self.strings.push(node),
            "new_expression" if wants.new_expressions => self.new_expressions.push(node),
            "if_statement" if wants.if_statements => self.if_statements.push(node),
            "program" | "statement_block" if wants.containers => self.containers.push(node),
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.visit(child, source, wants, language, descent);
        }
    }
}

#[derive(Default, Clone, Copy)]
struct Descent {
    inside_try: bool,
    in_async: bool,
    in_function: bool,
}

impl Descent {
    fn enter(self, node: Node<'_>, source: &[u8], wants: Wants, language: SourceLanguage) -> Self {
        Self {
            inside_try: wants.inside_try && (self.inside_try || node.kind() == "try_statement"),
            in_async: wants.in_async
                && if is_function_like(node) {
                    is_async_function_like(node, source)
                } else {
                    self.in_async
                },
            in_function: wants.functions
                && (self.in_function || is_scored_function(language, node.kind())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::parse_source;
    use std::path::Path;

    fn in_function_after(source: &str, descent: Descent, wants: Wants) -> bool {
        let tree = parse_source(Path::new("a.ts"), source).expect("typescript parses");
        let node = tree.root_node().child(0).expect("first statement");
        descent
            .enter(
                node,
                source.as_bytes(),
                wants,
                SourceLanguage::JavaScriptLike,
            )
            .in_function
    }

    fn functions_wanted() -> Wants {
        Wants {
            functions: true,
            ..Wants::default()
        }
    }

    fn inside_a_function() -> Descent {
        Descent {
            in_function: true,
            ..Descent::default()
        }
    }

    #[test]
    fn a_function_is_entered_only_when_functions_are_wanted() {
        let declaration = "function f() {}";
        assert!(!in_function_after(
            declaration,
            Descent::default(),
            Wants::default()
        ));
        assert!(in_function_after(
            declaration,
            Descent::default(),
            functions_wanted()
        ));
    }

    #[test]
    fn a_statement_inside_a_function_stays_inside_it() {
        assert!(in_function_after(
            "if (a) {}",
            inside_a_function(),
            functions_wanted()
        ));
        assert!(!in_function_after(
            "if (a) {}",
            Descent::default(),
            functions_wanted()
        ));
    }
}
