use std::path::Path;
use tree_sitter::{Language, Node, Parser};

#[derive(Debug, PartialEq, Eq)]
pub struct ImportRecord {
    pub specifier: String,
    pub line: usize,
    pub is_type_only: bool,
    pub is_dynamic: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct JsxElementRecord {
    pub tag_name: String,
    pub line: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct StringMatchRecord {
    pub line: usize,
    pub receiver: String,
    pub regex: String,
    pub start_row: usize,
    pub end_row: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct NestedTemplateLiteralRecord {
    pub line: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ConsecutiveArrayPushRecord {
    pub line: usize,
    pub count: usize,
    pub receiver: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct UnguardedJsonParseRecord {
    pub line: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SyncInAsyncRecord {
    pub line: usize,
    pub name: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SingleCharClassRecord {
    pub line: usize,
    pub class: String,
    pub fixed_regex: String,
    pub start_row: usize,
    pub end_row: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct PreferAtRecord {
    pub line: usize,
    pub base: String,
    pub offset: i64,
    pub start_row: usize,
    pub end_row: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

pub fn scan_imports(path: &Path, content: &str) -> Result<Vec<ImportRecord>, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&language_for_path(path))
        .map_err(|err| format!("klint-rs: failed to load TypeScript parser: {err}"))?;
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| "klint-rs: failed to parse source".to_string())?;

    let root = tree.root_node();
    let mut imports = Vec::new();
    walk_imports(root, content.as_bytes(), &mut imports);
    Ok(imports)
}

pub fn scan_jsx_elements(path: &Path, content: &str) -> Result<Vec<JsxElementRecord>, String> {
    if !is_jsx_path(path) {
        return Ok(Vec::new());
    }

    let mut parser = Parser::new();
    parser
        .set_language(&language_for_path(path))
        .map_err(|err| format!("klint-rs: failed to load TSX parser: {err}"))?;
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| "klint-rs: failed to parse source".to_string())?;

    let root = tree.root_node();
    let mut elements = Vec::new();
    walk_jsx_elements(root, content.as_bytes(), &mut elements);
    Ok(elements)
}

pub fn scan_string_match(path: &Path, content: &str) -> Result<Vec<StringMatchRecord>, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&language_for_path(path))
        .map_err(|err| format!("klint-rs: failed to load TypeScript parser: {err}"))?;
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| "klint-rs: failed to parse source".to_string())?;

    let root = tree.root_node();
    let mut records = Vec::new();
    walk_string_match(root, content.as_bytes(), &mut records);
    Ok(records)
}

pub fn scan_nested_template_literals(
    path: &Path,
    content: &str,
) -> Result<Vec<NestedTemplateLiteralRecord>, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&language_for_path(path))
        .map_err(|err| format!("klint-rs: failed to load TypeScript parser: {err}"))?;
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| "klint-rs: failed to parse source".to_string())?;

    let root = tree.root_node();
    let mut records = Vec::new();
    walk_template_literals(root, &mut records);
    Ok(records)
}

pub fn scan_consecutive_array_push(
    path: &Path,
    content: &str,
) -> Result<Vec<ConsecutiveArrayPushRecord>, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&language_for_path(path))
        .map_err(|err| format!("klint-rs: failed to load TypeScript parser: {err}"))?;
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| "klint-rs: failed to parse source".to_string())?;

    let root = tree.root_node();
    let mut records = Vec::new();
    walk_statement_containers(root, content.as_bytes(), &mut records);
    Ok(records)
}

pub fn scan_unguarded_json_parse(
    path: &Path,
    content: &str,
) -> Result<Vec<UnguardedJsonParseRecord>, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&language_for_path(path))
        .map_err(|err| format!("klint-rs: failed to load TypeScript parser: {err}"))?;
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| "klint-rs: failed to parse source".to_string())?;

    let root = tree.root_node();
    let mut records = Vec::new();
    walk_json_parse_calls(root, false, content.as_bytes(), &mut records);
    Ok(records)
}

pub fn scan_sync_in_async(path: &Path, content: &str) -> Result<Vec<SyncInAsyncRecord>, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&language_for_path(path))
        .map_err(|err| format!("klint-rs: failed to load TypeScript parser: {err}"))?;
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| "klint-rs: failed to parse source".to_string())?;

    let root = tree.root_node();
    let mut records = Vec::new();
    walk_sync_calls(root, false, content.as_bytes(), &mut records);
    Ok(records)
}

pub fn scan_single_char_classes(
    path: &Path,
    content: &str,
) -> Result<Vec<SingleCharClassRecord>, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&language_for_path(path))
        .map_err(|err| format!("klint-rs: failed to load TypeScript parser: {err}"))?;
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| "klint-rs: failed to parse source".to_string())?;

    let root = tree.root_node();
    let mut records = Vec::new();
    walk_single_char_classes(root, content.as_bytes(), &mut records);
    Ok(records)
}

pub fn scan_prefer_at(path: &Path, content: &str) -> Result<Vec<PreferAtRecord>, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&language_for_path(path))
        .map_err(|err| format!("klint-rs: failed to load TypeScript parser: {err}"))?;
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| "klint-rs: failed to parse source".to_string())?;

    let root = tree.root_node();
    let mut records = Vec::new();
    walk_prefer_at(root, content.as_bytes(), &mut records);
    Ok(records)
}

fn language_for_path(path: &Path) -> Language {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("tsx" | "jsx") => tree_sitter_typescript::LANGUAGE_TSX.into(),
        _ => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
    }
}

fn is_jsx_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("tsx" | "jsx")
    )
}

fn walk_imports(node: Node<'_>, source: &[u8], imports: &mut Vec<ImportRecord>) {
    if node.kind() == "import_statement" {
        if let Some(record) = static_import_record(node, source) {
            imports.push(record);
        }
    } else if node.kind() == "call_expression"
        && let Some(record) = dynamic_import_record(node, source)
    {
        imports.push(record);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_imports(child, source, imports);
    }
}

fn static_import_record(node: Node<'_>, source: &[u8]) -> Option<ImportRecord> {
    let source_node = node.child_by_field_name("source")?;
    Some(ImportRecord {
        specifier: node_text(source_node, source)?,
        line: source_node.start_position().row + 1,
        is_type_only: static_import_is_type_only(node, source),
        is_dynamic: false,
    })
}

fn static_import_is_type_only(node: Node<'_>, source: &[u8]) -> bool {
    let Ok(text) = node.utf8_text(source) else {
        return false;
    };
    text.trim_start().starts_with("import type ")
}

fn dynamic_import_record(node: Node<'_>, source: &[u8]) -> Option<ImportRecord> {
    let function = node.child_by_field_name("function")?;
    if function.kind() != "import" {
        return None;
    }

    let arguments = node.child_by_field_name("arguments")?;
    let specifier = first_string_child(arguments)?;
    Some(ImportRecord {
        specifier: node_text(specifier, source)?,
        line: specifier.start_position().row + 1,
        is_type_only: false,
        is_dynamic: true,
    })
}

fn first_string_child(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| child.kind() == "string")
}

fn walk_jsx_elements(node: Node<'_>, source: &[u8], elements: &mut Vec<JsxElementRecord>) {
    if matches!(
        node.kind(),
        "jsx_opening_element" | "jsx_self_closing_element"
    ) && let Some(record) = jsx_element_record(node, source)
    {
        elements.push(record);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_jsx_elements(child, source, elements);
    }
}

fn walk_string_match(node: Node<'_>, source: &[u8], records: &mut Vec<StringMatchRecord>) {
    if node.kind() == "call_expression"
        && let Some(record) = string_match_record(node, source)
    {
        records.push(record);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_string_match(child, source, records);
    }
}

fn walk_template_literals(node: Node<'_>, records: &mut Vec<NestedTemplateLiteralRecord>) {
    if node.kind() == "template_string" {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "template_substitution" {
                find_nested_template_literals(child, records);
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_template_literals(child, records);
    }
}

fn find_nested_template_literals(node: Node<'_>, records: &mut Vec<NestedTemplateLiteralRecord>) {
    if is_tagged_template_call(node) {
        return;
    }

    if node.kind() == "template_string" {
        records.push(NestedTemplateLiteralRecord {
            line: node.start_position().row + 1,
        });
        return;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        find_nested_template_literals(child, records);
    }
}

fn is_tagged_template_call(node: Node<'_>) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }

    let mut cursor = node.walk();
    node.children(&mut cursor)
        .any(|child| child.kind() == "template_string")
}

fn walk_statement_containers(
    node: Node<'_>,
    source: &[u8],
    records: &mut Vec<ConsecutiveArrayPushRecord>,
) {
    if matches!(node.kind(), "program" | "statement_block") {
        scan_statement_run(node, source, records);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_statement_containers(child, source, records);
    }
}

fn scan_statement_run(
    node: Node<'_>,
    source: &[u8],
    records: &mut Vec<ConsecutiveArrayPushRecord>,
) {
    let mut run_start: Option<Node<'_>> = None;
    let mut run_receiver = String::new();
    let mut run_count = 0;

    let flush = |records: &mut Vec<ConsecutiveArrayPushRecord>,
                 run_start: &mut Option<Node<'_>>,
                 run_receiver: &mut String,
                 run_count: &mut usize| {
        if let Some(start) = run_start
            && *run_count >= 2
        {
            records.push(ConsecutiveArrayPushRecord {
                line: start.start_position().row + 1,
                count: *run_count,
                receiver: run_receiver.clone(),
            });
        }
        *run_start = None;
        run_receiver.clear();
        *run_count = 0;
    };

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        let receiver = push_receiver(child, source);
        if let Some(receiver) = receiver {
            if receiver == run_receiver {
                run_count += 1;
            } else {
                flush(records, &mut run_start, &mut run_receiver, &mut run_count);
                run_start = Some(child);
                run_receiver = receiver;
                run_count = 1;
            }
        } else {
            flush(records, &mut run_start, &mut run_receiver, &mut run_count);
        }
    }
    flush(records, &mut run_start, &mut run_receiver, &mut run_count);
}

fn push_receiver(node: Node<'_>, source: &[u8]) -> Option<String> {
    if node.kind() != "expression_statement" {
        return None;
    }

    let expression = node.named_child(0)?;
    if expression.kind() != "call_expression" {
        return None;
    }

    let function = expression.child_by_field_name("function")?;
    if function.kind() != "member_expression" {
        return None;
    }

    let property = function.child_by_field_name("property")?;
    if raw_node_text(property, source)? != "push" {
        return None;
    }

    function
        .child_by_field_name("object")
        .and_then(|object| raw_node_text(object, source))
}

fn walk_json_parse_calls(
    node: Node<'_>,
    inside_try: bool,
    source: &[u8],
    records: &mut Vec<UnguardedJsonParseRecord>,
) {
    let next_inside_try = inside_try || node.kind() == "try_statement";
    if !next_inside_try && is_json_parse_call(node, source) {
        records.push(UnguardedJsonParseRecord {
            line: node.start_position().row + 1,
        });
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_json_parse_calls(child, next_inside_try, source, records);
    }
}

fn is_json_parse_call(node: Node<'_>, source: &[u8]) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }

    let Some(function) = node.child_by_field_name("function") else {
        return false;
    };
    if function.kind() != "member_expression" {
        return false;
    }

    let Some(object) = function.child_by_field_name("object") else {
        return false;
    };
    let Some(property) = function.child_by_field_name("property") else {
        return false;
    };

    raw_node_text(object, source).as_deref() == Some("JSON")
        && raw_node_text(property, source).as_deref() == Some("parse")
}

fn walk_sync_calls(
    node: Node<'_>,
    nearest_function_is_async: bool,
    source: &[u8],
    records: &mut Vec<SyncInAsyncRecord>,
) {
    let next_nearest_function_is_async = if is_function_like(node) {
        is_async_function_like(node, source)
    } else {
        nearest_function_is_async
    };

    if next_nearest_function_is_async && let Some(name) = sync_call_name(node, source) {
        records.push(SyncInAsyncRecord {
            line: node.start_position().row + 1,
            name,
        });
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_sync_calls(child, next_nearest_function_is_async, source, records);
    }
}

fn walk_single_char_classes(
    node: Node<'_>,
    source: &[u8],
    records: &mut Vec<SingleCharClassRecord>,
) {
    if node.kind() == "regex"
        && let Some(record) = single_char_class_record(node, source)
    {
        records.push(record);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_single_char_classes(child, source, records);
    }
}

fn walk_prefer_at(node: Node<'_>, source: &[u8], records: &mut Vec<PreferAtRecord>) {
    if node.kind() == "subscript_expression"
        && let Some(record) = prefer_at_record(node, source)
    {
        records.push(record);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_prefer_at(child, source, records);
    }
}

fn is_function_like(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "function_declaration" | "function_expression" | "arrow_function" | "method_definition"
    )
}

fn is_async_function_like(node: Node<'_>, source: &[u8]) -> bool {
    let Ok(text) = node.utf8_text(source) else {
        return false;
    };
    text.trim_start().starts_with("async ")
}

fn sync_call_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    if node.kind() != "call_expression" {
        return None;
    }

    let function = node.child_by_field_name("function")?;
    let name = if function.kind() == "identifier" {
        raw_node_text(function, source)?
    } else if function.kind() == "member_expression" {
        let property = function.child_by_field_name("property")?;
        raw_node_text(property, source)?
    } else {
        return None;
    };

    if name.ends_with("Sync") && name != "existsSync" {
        Some(name)
    } else {
        None
    }
}

fn string_match_record(node: Node<'_>, source: &[u8]) -> Option<StringMatchRecord> {
    let function = node.child_by_field_name("function")?;
    if function.kind() != "member_expression" {
        return None;
    }

    let property = function.child_by_field_name("property")?;
    if raw_node_text(property, source)? != "match" {
        return None;
    }

    let arguments = node.child_by_field_name("arguments")?;
    let argument = single_named_argument(arguments)?;
    if argument.kind() != "regex" {
        return None;
    }

    let regex = raw_node_text(argument, source)?;
    if regex_flags(&regex).contains('g') {
        return None;
    }

    let receiver = function
        .child_by_field_name("object")
        .and_then(|object| raw_node_text(object, source))?;
    let start = node.start_position();
    let end = node.end_position();

    Some(StringMatchRecord {
        line: start.row + 1,
        receiver,
        regex,
        start_row: start.row,
        end_row: end.row,
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
    })
}

#[derive(Debug)]
struct CharClass {
    start: usize,
    end: usize,
    inner: String,
}

fn single_char_class_record(node: Node<'_>, source: &[u8]) -> Option<SingleCharClassRecord> {
    let regex = raw_node_text(node, source)?;
    let pattern_end = regex.rfind('/')?;
    let pattern = regex.get(1..pattern_end)?;
    let flags = regex.get(pattern_end + 1..)?;
    let classes = parse_single_char_classes(pattern);
    let to_fix = classes
        .iter()
        .filter(|class| !is_char_class_metachar_exception(&class.inner))
        .collect::<Vec<_>>();
    if to_fix.is_empty() {
        return None;
    }

    let mut fixed_pattern = String::new();
    let mut prev = 0;
    for class in &classes {
        if is_char_class_metachar_exception(&class.inner) {
            continue;
        }
        fixed_pattern.push_str(pattern.get(prev..class.start)?);
        fixed_pattern.push_str(&class.inner);
        prev = class.end + 1;
    }
    fixed_pattern.push_str(pattern.get(prev..)?);

    let start = node.start_position();
    let end = node.end_position();
    Some(SingleCharClassRecord {
        line: start.row + 1,
        class: to_fix[0].inner.clone(),
        fixed_regex: format!("/{fixed_pattern}/{flags}"),
        start_row: start.row,
        end_row: end.row,
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
    })
}

fn prefer_at_record(node: Node<'_>, source: &[u8]) -> Option<PreferAtRecord> {
    let object = node.child_by_field_name("object")?;
    let index = node.child_by_field_name("index")?;
    if index.kind() != "binary_expression" {
        return None;
    }

    let left = index.child_by_field_name("left")?;
    let right = index.child_by_field_name("right")?;
    if !binary_expression_has_operator(index, "-") {
        return None;
    }
    if left.kind() != "member_expression" {
        return None;
    }

    let property = left.child_by_field_name("property")?;
    if raw_node_text(property, source).as_deref() != Some("length") {
        return None;
    }

    let offset = numeric_literal_integer_value(right, source)?;
    if offset <= 0 {
        return None;
    }

    let base = raw_node_text(object, source)?;
    let length_base = left
        .child_by_field_name("object")
        .and_then(|base| raw_node_text(base, source))?;
    if base != length_base {
        return None;
    }

    let start = node.start_position();
    let end = node.end_position();
    Some(PreferAtRecord {
        line: start.row + 1,
        base,
        offset,
        start_row: start.row,
        end_row: end.row,
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
    })
}

fn binary_expression_has_operator(node: Node<'_>, operator: &str) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .any(|child| child.kind() == operator)
}

fn numeric_literal_integer_value(node: Node<'_>, source: &[u8]) -> Option<i64> {
    if node.kind() != "number" {
        return None;
    }
    let value = raw_node_text(node, source)?;
    let parsed = value.parse::<f64>().ok()?;
    if !parsed.is_finite() || parsed.fract() != 0.0 {
        return None;
    }
    Some(parsed as i64)
}

fn parse_single_char_classes(pattern: &str) -> Vec<CharClass> {
    let mut results = Vec::new();
    let mut i = 0;
    while i < pattern.len() {
        if byte_at(pattern, i) == Some(b'\\') {
            i += 2;
            continue;
        }
        if byte_at(pattern, i) != Some(b'[') {
            i += 1;
            continue;
        }

        let start = i;
        i += 1;
        if byte_at(pattern, i) == Some(b'^') {
            while i < pattern.len() && byte_at(pattern, i) != Some(b']') {
                if byte_at(pattern, i) == Some(b'\\') {
                    i += 1;
                }
                i += 1;
            }
            i += 1;
            continue;
        }

        let mut tokens = Vec::new();
        while i < pattern.len() && byte_at(pattern, i) != Some(b']') {
            if byte_at(pattern, i) == Some(b'\\') {
                let token_start = i;
                i += 1;
                if i >= pattern.len() {
                    break;
                }
                match byte_at(pattern, i) {
                    Some(b'u') if i + 5 <= pattern.len() => i += 5,
                    Some(b'x') if i + 3 <= pattern.len() => i += 3,
                    Some(b'c') if i + 1 < pattern.len() => i += 2,
                    Some(_) => i += 1,
                    None => break,
                }
                tokens.push(pattern[token_start..i].to_string());
            } else {
                let Some(ch) = pattern[i..].chars().next() else {
                    break;
                };
                let token_start = i;
                i += ch.len_utf8();
                tokens.push(pattern[token_start..i].to_string());
            }
        }

        if i >= pattern.len() {
            break;
        }
        let end = i;
        i += 1;
        if tokens.len() == 1 {
            results.push(CharClass {
                start,
                end,
                inner: tokens.remove(0),
            });
        }
    }
    results
}

fn byte_at(value: &str, index: usize) -> Option<u8> {
    value.as_bytes().get(index).copied()
}

fn is_char_class_metachar_exception(inner: &str) -> bool {
    matches!(
        inner,
        "." | "*" | "+" | "?" | "{" | "}" | "(" | ")" | "|" | "$"
    )
}

fn single_named_argument(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    let mut arguments = node
        .named_children(&mut cursor)
        .filter(|child| child.kind() != "comment");
    let argument = arguments.next()?;
    if arguments.next().is_some() {
        return None;
    }
    Some(argument)
}

fn regex_flags(literal: &str) -> &str {
    let Some(index) = literal.rfind('/') else {
        return "";
    };
    &literal[index + 1..]
}

fn jsx_element_record(node: Node<'_>, source: &[u8]) -> Option<JsxElementRecord> {
    let name = node
        .child_by_field_name("name")
        .or_else(|| first_identifier_child(node))?;
    if name.kind() != "identifier" {
        return None;
    }

    Some(JsxElementRecord {
        tag_name: node_text(name, source)?,
        line: name.start_position().row + 1,
    })
}

fn first_identifier_child(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| child.kind() == "identifier")
}

fn node_text(node: Node<'_>, source: &[u8]) -> Option<String> {
    let raw = raw_node_text(node, source)?;
    Some(raw.trim_matches(['"', '\'', '`']).to_string())
}

fn raw_node_text(node: Node<'_>, source: &[u8]) -> Option<String> {
    Some(node.utf8_text(source).ok()?.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn imports(content: &str) -> Vec<ImportRecord> {
        scan_imports(&PathBuf::from("index.ts"), content).expect("source should parse")
    }

    #[test]
    fn extracts_static_imports_with_line_numbers() {
        assert_eq!(
            imports("import { foo } from \"./foo\";\nimport bar from '../bar';\n"),
            vec![
                ImportRecord {
                    specifier: "./foo".to_string(),
                    line: 1,
                    is_type_only: false,
                    is_dynamic: false,
                },
                ImportRecord {
                    specifier: "../bar".to_string(),
                    line: 2,
                    is_type_only: false,
                    is_dynamic: false,
                },
            ]
        );
    }

    #[test]
    fn extracts_dynamic_imports_with_line_numbers() {
        assert_eq!(
            imports("export async function load() {\n  return import(\"./lazy\");\n}\n"),
            vec![ImportRecord {
                specifier: "./lazy".to_string(),
                line: 2,
                is_type_only: false,
                is_dynamic: true,
            }]
        );
    }

    #[test]
    fn marks_type_only_imports() {
        assert_eq!(
            imports("import type { Foo } from \"./types\";\nimport { foo } from \"./foo\";\n"),
            vec![
                ImportRecord {
                    specifier: "./types".to_string(),
                    line: 1,
                    is_type_only: true,
                    is_dynamic: false,
                },
                ImportRecord {
                    specifier: "./foo".to_string(),
                    line: 2,
                    is_type_only: false,
                    is_dynamic: false,
                },
            ]
        );
    }

    #[test]
    fn uses_tsx_parser_for_tsx_files() {
        let records = scan_imports(
            &PathBuf::from("page.tsx"),
            "import { Button } from './button';\nexport const page = <Button />;\n",
        )
        .expect("tsx source should parse");

        assert_eq!(
            records,
            vec![ImportRecord {
                specifier: "./button".to_string(),
                line: 1,
                is_type_only: false,
                is_dynamic: false,
            }]
        );
    }

    #[test]
    fn extracts_jsx_opening_and_self_closing_elements() {
        let records = scan_jsx_elements(
            &PathBuf::from("page.tsx"),
            "export const page = <main>\n  <button>Click</button>\n  <input />\n</main>;\n",
        )
        .expect("tsx source should parse");

        assert_eq!(
            records,
            vec![
                JsxElementRecord {
                    tag_name: "main".to_string(),
                    line: 1,
                },
                JsxElementRecord {
                    tag_name: "button".to_string(),
                    line: 2,
                },
                JsxElementRecord {
                    tag_name: "input".to_string(),
                    line: 3,
                },
            ]
        );
    }

    #[test]
    fn skips_jsx_scan_for_plain_typescript_files() {
        let records = scan_jsx_elements(
            &PathBuf::from("page.ts"),
            "export const page = '<button>Click</button>';\n",
        )
        .expect("non-jsx source should be skipped");

        assert!(records.is_empty());
    }

    #[test]
    fn extracts_non_global_string_match_calls() {
        let records = scan_string_match(
            &PathBuf::from("index.ts"),
            "const m = line.match(/foo/i);\nconst ok = line.match(/foo/g);\n",
        )
        .expect("source should parse");

        assert_eq!(
            records,
            vec![StringMatchRecord {
                line: 1,
                receiver: "line".to_string(),
                regex: "/foo/i".to_string(),
                start_row: 0,
                end_row: 0,
                start_byte: 10,
                end_byte: 28,
            }]
        );
    }

    #[test]
    fn ignores_string_match_with_variable_argument() {
        let records = scan_string_match(
            &PathBuf::from("index.ts"),
            "declare const re: RegExp;\nconst m = line.match(re);\n",
        )
        .expect("source should parse");

        assert!(records.is_empty());
    }

    #[test]
    fn extracts_nested_template_literals() {
        let records = scan_nested_template_literals(
            &PathBuf::from("index.ts"),
            "declare const b: boolean;\nconst value = `${b ? `yes` : `no`}`;\n",
        )
        .expect("source should parse");

        assert_eq!(
            records,
            vec![
                NestedTemplateLiteralRecord { line: 2 },
                NestedTemplateLiteralRecord { line: 2 },
            ]
        );
    }

    #[test]
    fn ignores_standalone_and_tagged_template_literals() {
        let records = scan_nested_template_literals(
            &PathBuf::from("index.ts"),
            "declare function tag(s: TemplateStringsArray): string;\nconst standalone = `hello`;\nconst value = `${tag`inner`}`;\n",
        )
        .expect("source should parse");

        assert!(records.is_empty());
    }

    #[test]
    fn extracts_consecutive_array_push_runs() {
        let records = scan_consecutive_array_push(
            &PathBuf::from("index.ts"),
            "const arr: number[] = [];\narr.push(1);\narr.push(2);\narr.push(3);\n",
        )
        .expect("source should parse");

        assert_eq!(
            records,
            vec![ConsecutiveArrayPushRecord {
                line: 2,
                count: 3,
                receiver: "arr".to_string(),
            }]
        );
    }

    #[test]
    fn ignores_array_push_runs_split_by_other_statements() {
        let records = scan_consecutive_array_push(
            &PathBuf::from("index.ts"),
            "const arr: number[] = [];\narr.push(1);\nconsole.log(arr);\narr.push(2);\n",
        )
        .expect("source should parse");

        assert!(records.is_empty());
    }

    #[test]
    fn extracts_unguarded_json_parse_calls() {
        let records = scan_unguarded_json_parse(
            &PathBuf::from("index.ts"),
            "const value = JSON.parse(raw);\ntry {\n  JSON.parse(raw);\n} catch {}\n",
        )
        .expect("source should parse");

        assert_eq!(records, vec![UnguardedJsonParseRecord { line: 1 }]);
    }

    #[test]
    fn ignores_json_parse_nested_inside_try_statement() {
        let records = scan_unguarded_json_parse(
            &PathBuf::from("index.ts"),
            "try {\n  function parse() {\n    return JSON.parse(raw);\n  }\n} catch {}\n",
        )
        .expect("source should parse");

        assert!(records.is_empty());
    }

    #[test]
    fn extracts_sync_calls_inside_async_functions() {
        let records = scan_sync_in_async(
            &PathBuf::from("index.ts"),
            "async function load() {\n  readFileSync(path);\n  fs.writeFileSync(path, value);\n}\n",
        )
        .expect("source should parse");

        assert_eq!(
            records,
            vec![
                SyncInAsyncRecord {
                    line: 2,
                    name: "readFileSync".to_string(),
                },
                SyncInAsyncRecord {
                    line: 3,
                    name: "writeFileSync".to_string(),
                },
            ]
        );
    }

    #[test]
    fn ignores_exists_sync_and_nested_sync_functions() {
        let records = scan_sync_in_async(
            &PathBuf::from("index.ts"),
            "async function load() {\n  existsSync(path);\n  function nested() {\n    readFileSync(path);\n  }\n}\n",
        )
        .expect("source should parse");

        assert!(records.is_empty());
    }

    #[test]
    fn extracts_single_char_classes() {
        let records =
            scan_single_char_classes(&PathBuf::from("index.ts"), "const r = /a[b]c[\\d]/;\n")
                .expect("source should parse");

        assert_eq!(
            records,
            vec![SingleCharClassRecord {
                line: 1,
                class: "b".to_string(),
                fixed_regex: "/abc\\d/".to_string(),
                start_row: 0,
                end_row: 0,
                start_byte: 10,
                end_byte: 21,
            }]
        );
    }

    #[test]
    fn ignores_non_single_and_exception_classes() {
        let records = scan_single_char_classes(
            &PathBuf::from("index.ts"),
            "const r = /[ab][a-z][^a][.][*]/;\n",
        )
        .expect("source should parse");

        assert!(records.is_empty());
    }

    #[test]
    fn extracts_prefer_at_negative_index_access() {
        let records = scan_prefer_at(
            &PathBuf::from("index.ts"),
            "const x = arr[arr.length - 1];\nconst y = obj.items[obj.items.length - 5];\n",
        )
        .expect("source should parse");

        assert_eq!(
            records,
            vec![
                PreferAtRecord {
                    line: 1,
                    base: "arr".to_string(),
                    offset: 1,
                    start_row: 0,
                    end_row: 0,
                    start_byte: 10,
                    end_byte: 29,
                },
                PreferAtRecord {
                    line: 2,
                    base: "obj.items".to_string(),
                    offset: 5,
                    start_row: 1,
                    end_row: 1,
                    start_byte: 41,
                    end_byte: 72,
                },
            ]
        );
    }

    #[test]
    fn ignores_prefer_at_non_matching_index_access() {
        let records = scan_prefer_at(
            &PathBuf::from("index.ts"),
            "const a = arr[other.length - 1];\nconst b = arr[arr.length - 0];\nconst c = arr[arr.length + 1];\nconst d = arr[i];\n",
        )
        .expect("source should parse");

        assert!(records.is_empty());
    }
}
