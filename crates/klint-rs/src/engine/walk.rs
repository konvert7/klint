use tree_sitter::Node;

use crate::syntax::{is_async_function_like, is_function_like};

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
}

impl<'a> NodeIndex<'a> {
    pub(crate) fn build(root: Node<'a>, source: &[u8], wants: Wants) -> Self {
        let mut index = Self::default();
        if wants.wants_anything() {
            index.visit(root, source, wants, Descent::default());
        }
        index
    }

    fn visit(&mut self, node: Node<'a>, source: &[u8], wants: Wants, descent: Descent) {
        let descent = descent.enter(node, source, wants);
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
            self.visit(child, source, wants, descent);
        }
    }
}

#[derive(Default, Clone, Copy)]
struct Descent {
    inside_try: bool,
    in_async: bool,
}

impl Descent {
    fn enter(self, node: Node<'_>, source: &[u8], wants: Wants) -> Self {
        Self {
            inside_try: wants.inside_try && (self.inside_try || node.kind() == "try_statement"),
            in_async: wants.in_async
                && if is_function_like(node) {
                    is_async_function_like(node, source)
                } else {
                    self.in_async
                },
        }
    }
}
