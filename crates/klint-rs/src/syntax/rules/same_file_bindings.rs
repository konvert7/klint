use std::collections::BTreeMap;

use tree_sitter::Node;

use super::{is_async_function_like, raw_node_text};

pub(crate) struct SameFileBindings<'a> {
    source: &'a [u8],
    bindings: BTreeMap<String, Vec<Node<'a>>>,
}

impl<'a> SameFileBindings<'a> {
    pub(crate) fn new(identifiers: &[Node<'a>], source: &'a [u8]) -> Self {
        let mut bindings: BTreeMap<String, Vec<Node<'a>>> = BTreeMap::new();

        for node in identifiers
            .iter()
            .copied()
            .filter(|node| !is_reference(*node))
        {
            if let Some(text) = raw_node_text(node, source) {
                bindings.entry(text).or_default().push(node);
            }
        }

        Self { source, bindings }
    }

    pub(crate) fn is_unbound_global(&self, name: &str) -> bool {
        !self.bindings.contains_key(name)
    }

    pub(crate) fn is_const_date(&self, use_site: Node<'a>) -> bool {
        self.resolves_to(use_site, Self::is_date_declaration)
    }

    pub(crate) fn is_same_file_async_function(&self, use_site: Node<'a>) -> bool {
        self.resolves_to(use_site, Self::is_async_declaration)
    }

    pub(crate) fn is_new_date(&self, node: Node<'a>) -> bool {
        node.kind() == "new_expression"
            && node
                .child_by_field_name("constructor")
                .is_some_and(|constructor| is_named_identifier(constructor, self.source, "Date"))
            && self.is_unbound_global("Date")
    }

    fn resolves_to(&self, use_site: Node<'a>, provable: fn(&Self, Node<'a>) -> bool) -> bool {
        if use_site.kind() != "identifier" {
            return false;
        }

        let Some(name) = raw_node_text(use_site, self.source) else {
            return false;
        };
        let Some(bindings) = self.bindings.get(&name) else {
            return false;
        };

        bindings.iter().any(|declaration| {
            provable(self, *declaration)
                && declaring_container(*declaration).is_some_and(|container| {
                    contains(container, use_site)
                        && !is_overridden(bindings, *declaration, container, use_site)
                })
        })
    }

    fn is_date_declaration(&self, binding: Node<'a>) -> bool {
        let Some(declarator) = const_declarator(binding) else {
            return false;
        };

        declarator
            .child_by_field_name("value")
            .is_some_and(|value| self.is_new_date(strip_parentheses(value)))
            && is_date_annotation_or_absent(declarator, self.source)
    }

    fn is_async_declaration(&self, binding: Node<'a>) -> bool {
        is_async_function_declaration(binding, self.source)
            || is_async_const_function(binding, self.source)
    }
}

pub(crate) fn strip_parentheses(node: Node<'_>) -> Node<'_> {
    if node.kind() != "parenthesized_expression" {
        return node;
    }

    let mut cursor = node.walk();
    match node
        .named_children(&mut cursor)
        .find(|child| child.kind() != "comment")
    {
        Some(inner) => strip_parentheses(inner),
        None => node,
    }
}

pub(crate) fn is_named_identifier(node: Node<'_>, source: &[u8], text: &str) -> bool {
    node.kind() == "identifier" && raw_node_text(node, source).as_deref() == Some(text)
}

fn is_date_annotation_or_absent(declarator: Node<'_>, source: &[u8]) -> bool {
    match declarator.child_by_field_name("type") {
        None => true,
        Some(annotation) => is_date_annotation(annotation, source),
    }
}

fn is_date_annotation(annotation: Node<'_>, source: &[u8]) -> bool {
    annotation.kind() == "type_annotation"
        && annotation.named_child_count() == 1
        && annotation
            .named_child(0)
            .is_some_and(|inner| is_named_type(inner, source, "Date"))
}

fn is_named_type(node: Node<'_>, source: &[u8], text: &str) -> bool {
    node.kind() == "type_identifier" && raw_node_text(node, source).as_deref() == Some(text)
}

fn is_async_function_declaration(binding: Node<'_>, source: &[u8]) -> bool {
    let Some(parent) = binding.parent() else {
        return false;
    };

    parent.kind() == "function_declaration"
        && is_field(parent, "name", binding)
        && is_async_function_like(parent, source)
        && is_statement_level(parent)
}

fn is_async_const_function(binding: Node<'_>, source: &[u8]) -> bool {
    let Some(declarator) = const_declarator(binding) else {
        return false;
    };

    declarator
        .child_by_field_name("value")
        .is_some_and(|value| {
            matches!(
                value.kind(),
                "arrow_function" | "function_expression" | "function"
            ) && is_async_function_like(value, source)
        })
}

fn is_reference(node: Node<'_>) -> bool {
    match node.kind() {
        "shorthand_property_identifier_pattern" => false,
        "type_identifier" => !is_class_name(node),
        "identifier" => is_identifier_reference(node),
        _ => false,
    }
}

fn is_class_name(node: Node<'_>) -> bool {
    node.parent().is_some_and(|parent| {
        matches!(
            parent.kind(),
            "class_declaration" | "abstract_class_declaration" | "class"
        )
    })
}

const REFERENCE_PARENTS: &[&str] = &[
    "binary_expression",
    "unary_expression",
    "await_expression",
    "return_statement",
    "expression_statement",
    "ternary_expression",
    "arguments",
    "array",
    "spread_element",
    "template_substitution",
    "sequence_expression",
    "throw_statement",
    "jsx_expression",
    "subscript_expression",
];

const REFERENCE_FIELDS: &[(&str, &str)] = &[
    ("call_expression", "function"),
    ("new_expression", "constructor"),
    ("member_expression", "object"),
    ("pair", "value"),
    ("variable_declarator", "value"),
    ("assignment_expression", "right"),
    ("augmented_assignment_expression", "right"),
    ("arrow_function", "body"),
];

fn is_identifier_reference(node: Node<'_>) -> bool {
    let position = climb_wrappers(node);
    let Some(parent) = position.parent() else {
        return false;
    };

    REFERENCE_PARENTS.contains(&parent.kind())
        || REFERENCE_FIELDS
            .iter()
            .any(|(kind, field)| parent.kind() == *kind && is_field(parent, field, position))
}

fn climb_wrappers(node: Node<'_>) -> Node<'_> {
    match node.parent() {
        Some(parent) if is_wrapper(parent.kind()) => climb_wrappers(parent),
        _ => node,
    }
}

fn is_wrapper(kind: &str) -> bool {
    matches!(
        kind,
        "parenthesized_expression"
            | "non_null_expression"
            | "as_expression"
            | "satisfies_expression"
    )
}

fn is_field(parent: Node<'_>, field: &str, child: Node<'_>) -> bool {
    parent
        .child_by_field_name(field)
        .is_some_and(|candidate| candidate.id() == child.id())
}

fn const_declarator(binding: Node<'_>) -> Option<Node<'_>> {
    let declarator = binding.parent()?;
    if declarator.kind() != "variable_declarator" || !is_field(declarator, "name", binding) {
        return None;
    }

    let declaration = declarator.parent()?;
    let is_const = declaration.kind() == "lexical_declaration"
        && declaration
            .child_by_field_name("kind")
            .is_some_and(|kind| kind.kind() == "const");

    (is_const && is_statement_level(declaration)).then_some(declarator)
}

fn is_statement_level(declaration: Node<'_>) -> bool {
    declaration.parent().is_some_and(|parent| {
        matches!(
            parent.kind(),
            "program" | "statement_block" | "export_statement"
        )
    })
}

fn declaring_container(binding: Node<'_>) -> Option<Node<'_>> {
    nearest_block(binding)
}

fn contains(outer: Node<'_>, inner: Node<'_>) -> bool {
    outer.start_byte() <= inner.start_byte() && inner.end_byte() <= outer.end_byte()
}

fn is_overridden(
    bindings: &[Node<'_>],
    declaration: Node<'_>,
    container: Node<'_>,
    use_site: Node<'_>,
) -> bool {
    bindings
        .iter()
        .any(|other| other.id() != declaration.id() && overrides(*other, container, use_site))
}

fn overrides(other: Node<'_>, container: Node<'_>, use_site: Node<'_>) -> bool {
    if !contains(container, other) {
        return false;
    }

    match declaration_scope(other) {
        Some(scope) => contains(scope, use_site),
        None => true,
    }
}

fn declaration_scope(binding: Node<'_>) -> Option<Node<'_>> {
    let site = pattern_root(binding);
    let parent = site.parent()?;

    match parent.kind() {
        "variable_declarator" if is_field(parent, "name", site) => declarator_scope(parent),
        "required_parameter" | "optional_parameter" | "formal_parameters" => {
            nearest_function_or_program(site)
        }
        "arrow_function" if is_field(parent, "parameter", site) => {
            nearest_function_or_program(site)
        }
        "function_declaration" | "generator_function_declaration"
            if is_field(parent, "name", site) =>
        {
            nearest_function_or_program(parent)
        }
        "class_declaration" | "abstract_class_declaration" if is_field(parent, "name", site) => {
            nearest_block(parent)
        }
        "catch_clause" if is_field(parent, "parameter", site) => Some(parent),
        "import_specifier" | "import_clause" | "namespace_import" | "import_require_clause" => {
            Some(root_of(parent))
        }
        _ => None,
    }
}

fn declarator_scope(declarator: Node<'_>) -> Option<Node<'_>> {
    let declaration = declarator.parent()?;

    match declaration.kind() {
        "lexical_declaration" => nearest_block(declaration),
        "variable_declaration" => nearest_function_or_program(declaration),
        _ => None,
    }
}

fn pattern_root(binding: Node<'_>) -> Node<'_> {
    match binding.parent() {
        Some(parent) if is_pattern_part(parent.kind()) => pattern_root(parent),
        _ => binding,
    }
}

fn is_pattern_part(kind: &str) -> bool {
    matches!(
        kind,
        "object_pattern"
            | "array_pattern"
            | "pair_pattern"
            | "assignment_pattern"
            | "object_assignment_pattern"
            | "rest_pattern"
    )
}

fn nearest_block(node: Node<'_>) -> Option<Node<'_>> {
    find_ancestor(node, &["program", "statement_block"])
}

fn nearest_function_or_program(node: Node<'_>) -> Option<Node<'_>> {
    find_ancestor(
        node,
        &[
            "program",
            "function_declaration",
            "generator_function_declaration",
            "function_expression",
            "function",
            "generator_function",
            "arrow_function",
            "method_definition",
        ],
    )
}

fn find_ancestor<'t>(node: Node<'t>, kinds: &[&str]) -> Option<Node<'t>> {
    let mut current = node.parent();

    while let Some(ancestor) = current {
        if kinds.contains(&ancestor.kind()) {
            return Some(ancestor);
        }

        current = ancestor.parent();
    }

    None
}

fn root_of(node: Node<'_>) -> Node<'_> {
    match node.parent() {
        Some(parent) => root_of(parent),
        None => node,
    }
}
