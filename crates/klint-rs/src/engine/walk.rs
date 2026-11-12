use tree_sitter::Node;

use crate::syntax::{
    FunctionLength, FunctionScore, LineTally, LineTotals, ScoringContext, SourceLanguage,
    is_async_function_like, is_comment, is_function_like, is_scored_function,
    is_statement_container, is_token, measured_body, score_node, starts_uncounted_region,
};

fn is_scored_node(language: SourceLanguage, node: Node<'_>) -> bool {
    node.is_named() && is_scored_function(language, node.kind())
}

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
    pub function_lengths: bool,
    pub binary_expressions: bool,
    pub expression_statements: bool,
    pub identifiers: bool,
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
            || self.function_lengths
            || self.binary_expressions
            || self.expression_statements
            || self.identifiers
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
    pub scores: Vec<FunctionScore<'a>>,
    pub function_lengths: Vec<FunctionLength<'a>>,
    pub binary_expressions: Vec<Node<'a>>,
    pub expression_statements: Vec<Node<'a>>,
    pub identifiers: Vec<Node<'a>>,
    pub has_syntax_error: bool,
    tally: LineTally,
    pending_bodies: Vec<(usize, Node<'a>)>,
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

    fn score(
        &mut self,
        node: Node<'a>,
        wants: Wants,
        language: SourceLanguage,
        descent: Descent<'a>,
    ) -> ScoringContext<'a> {
        if wants.functions && (descent.in_function || is_scored_node(language, node)) {
            score_node(node, descent.scoring, language, &mut self.scores)
        } else {
            ScoringContext::default()
        }
    }

    fn tally_node(&mut self, node: Node<'a>, source: &[u8], language: SourceLanguage) {
        if node.is_extra() {
            if is_comment(node) {
                self.tally
                    .absorb_comment(source, node.start_byte(), node.end_byte());
            }
        } else if is_token(language, node) {
            let (start, end) = (node.start_byte(), node.end_byte());
            let kind = if end - start == 1 { node.kind() } else { "" };
            self.tally.add_token(source, kind, start, end);
        }
    }

    fn take_pending_body(&mut self, node: Node<'a>) -> Option<Node<'a>> {
        let (body_id, _) = self.pending_bodies.last()?;
        if *body_id != node.id() {
            return None;
        }

        self.pending_bodies.pop().map(|(_, function)| function)
    }

    fn record_length(&mut self, function: Node<'a>, before: LineTotals) {
        let counted = self.tally.totals().since(before);
        self.function_lengths.push(FunctionLength {
            node: function,
            lines: counted.lines,
            non_blank_lines: counted.non_blank_lines,
        });
    }

    fn push_expression_buckets(&mut self, node: Node<'a>, wants: Wants) {
        match node.kind() {
            "binary_expression" if wants.binary_expressions => self.binary_expressions.push(node),
            "expression_statement" if wants.expression_statements => {
                self.expression_statements.push(node);
            }
            "identifier" | "shorthand_property_identifier_pattern" | "type_identifier"
                if wants.identifiers =>
            {
                self.identifiers.push(node);
            }
            _ => {}
        }
    }

    fn visit_children(
        &mut self,
        node: Node<'a>,
        source: &[u8],
        wants: Wants,
        language: SourceLanguage,
        descent: Descent<'a>,
    ) {
        let measures_lengths = wants.function_lengths && node.child_count() > 0;
        if measures_lengths {
            if let Some(body) = measured_body(language, node) {
                self.pending_bodies.push((body.id(), node));
            }
        }

        let absorbs_jsx_text = measures_lengths
            && language == SourceLanguage::JavaScriptLike
            && node.kind() == "jsx_element";
        let mut cursor = node.walk();

        for (position, child) in node.children(&mut cursor).enumerate() {
            if absorbs_jsx_text && position > 0 {
                self.tally.absorb_gap_before(child.start_byte());
            }

            self.visit(child, source, wants, language, descent);
        }
    }

    fn visit(
        &mut self,
        node: Node<'a>,
        source: &[u8],
        wants: Wants,
        language: SourceLanguage,
        descent: Descent<'a>,
    ) {
        let measured_function = self.take_pending_body(node);
        let before_body = self.tally.totals();
        let scoring = self.score(node, wants, language, descent);

        if wants.function_lengths && !descent.in_uncounted {
            self.tally_node(node, source, language);
        }

        let descent = Descent {
            scoring,
            ..descent.enter(node, source, wants, language)
        };

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
            _ => self.push_expression_buckets(node, wants),
        }

        self.visit_children(node, source, wants, language, descent);

        if let Some(function) = measured_function {
            self.record_length(function, before_body);
        }
    }
}

#[derive(Default, Clone, Copy)]
struct Descent<'a> {
    scoring: ScoringContext<'a>,
    inside_try: bool,
    in_async: bool,
    in_function: bool,
    in_uncounted: bool,
}

impl Descent<'_> {
    fn enter(self, node: Node<'_>, source: &[u8], wants: Wants, language: SourceLanguage) -> Self {
        Self {
            scoring: self.scoring,
            inside_try: wants.inside_try && (self.inside_try || node.kind() == "try_statement"),
            in_async: wants.in_async
                && if is_function_like(node) {
                    is_async_function_like(node, source)
                } else {
                    self.in_async
                },
            in_function: wants.functions && (self.in_function || is_scored_node(language, node)),
            in_uncounted: wants.function_lengths
                && (self.in_uncounted || starts_uncounted_region(language, node)),
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

    fn inside_a_function() -> Descent<'static> {
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
