use tree_sitter::Node;

use super::case::Format;
use crate::syntax::SourceLanguage;

#[derive(Clone, Copy)]
pub(crate) struct Convention {
    pub label: &'static str,
    pub formats: &'static [Format],
    pub strict: bool,
    pub ascii_only: bool,
}

const GLOBAL_JS_FUNCTION: Convention = Convention {
    label: "Function",
    formats: &[Format::Camel, Format::Pascal, Format::Upper],
    strict: true,
    ascii_only: true,
};
const NESTED_JS_FUNCTION: Convention = Convention {
    label: "Function",
    formats: &[Format::Camel, Format::Pascal],
    strict: true,
    ascii_only: true,
};
const JS_CLASS: Convention = Convention {
    label: "Class",
    formats: &[Format::Pascal],
    strict: true,
    ascii_only: true,
};
const SNAKE_FUNCTION: Convention = Convention {
    label: "Function",
    formats: &[Format::Snake],
    strict: false,
    ascii_only: false,
};
const CAMEL_FUNCTION: Convention = Convention {
    label: "Function",
    formats: &[Format::Camel],
    strict: false,
    ascii_only: false,
};
const PASCAL_METHOD: Convention = Convention {
    label: "Method",
    formats: &[Format::Pascal],
    strict: false,
    ascii_only: false,
};
const PASCAL_CLASS: Convention = Convention {
    label: "Class",
    formats: &[Format::Pascal],
    strict: false,
    ascii_only: false,
};
const PASCAL_TYPE: Convention = Convention {
    label: "Type",
    formats: &[Format::Pascal],
    strict: false,
    ascii_only: false,
};

const JS_FUNCTIONS: &[&str] = &[
    "function_declaration",
    "generator_function_declaration",
    "function_expression",
    "function",
    "generator_function",
    "function_signature",
];
const JS_CLASSES: &[&str] = &["class_declaration", "abstract_class_declaration", "class"];
const JS_SCOPES: &[&str] = &[
    "function_declaration",
    "generator_function_declaration",
    "function_expression",
    "function",
    "generator_function",
    "arrow_function",
    "method_definition",
];
const NAME_KINDS: &[&str] = &["identifier", "type_identifier", "simple_identifier"];

pub(crate) fn convention_of(node: Node<'_>, language: SourceLanguage) -> Option<Convention> {
    let kind = node.kind();

    match language {
        SourceLanguage::JavaScriptLike => javascript(node),
        SourceLanguage::Python => match kind {
            "function_definition" => Some(SNAKE_FUNCTION),
            "class_definition" => Some(PASCAL_CLASS),
            _ => None,
        },
        SourceLanguage::Rust => match kind {
            "function_item" | "function_signature_item" => Some(SNAKE_FUNCTION),
            "struct_item" | "enum_item" | "trait_item" | "union_item" | "type_item" => {
                Some(PASCAL_TYPE)
            }
            _ => None,
        },
        SourceLanguage::CSharp => csharp(kind),
        SourceLanguage::Swift => match kind {
            "function_declaration" => Some(CAMEL_FUNCTION),
            "class_declaration" | "protocol_declaration" => Some(PASCAL_TYPE),
            _ => None,
        },
    }
}

fn javascript(node: Node<'_>) -> Option<Convention> {
    let kind = node.kind();

    if JS_CLASSES.contains(&kind) {
        return Some(JS_CLASS);
    }

    if !JS_FUNCTIONS.contains(&kind) {
        return None;
    }

    if is_global(node) {
        Some(GLOBAL_JS_FUNCTION)
    } else {
        Some(NESTED_JS_FUNCTION)
    }
}

fn is_global(node: Node<'_>) -> bool {
    let mut ancestor = node.parent();

    while let Some(current) = ancestor {
        if JS_SCOPES.contains(&current.kind()) {
            return false;
        }

        ancestor = current.parent();
    }

    true
}

fn csharp(kind: &str) -> Option<Convention> {
    match kind {
        "method_declaration" | "local_function_statement" => Some(PASCAL_METHOD),
        "class_declaration"
        | "struct_declaration"
        | "interface_declaration"
        | "record_declaration"
        | "enum_declaration" => Some(PASCAL_TYPE),
        _ => None,
    }
}

pub(crate) fn name_node(node: Node<'_>) -> Option<Node<'_>> {
    node.child_by_field_name("name")
        .filter(|name| NAME_KINDS.contains(&name.kind()))
}

pub(crate) fn is_renamable_declaration(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "function_declaration"
            | "generator_function_declaration"
            | "class_declaration"
            | "abstract_class_declaration"
    )
}
