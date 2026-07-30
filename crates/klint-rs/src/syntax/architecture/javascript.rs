use tree_sitter::Node;

use super::{ImportRecord, first_string_child};
use crate::syntax::node_text;

pub(super) fn walk_imports(node: Node<'_>, source: &[u8], imports: &mut Vec<ImportRecord>) {
    match node.kind() {
        "import_statement" => imports.extend(module_source_record(node, source, "import type ")),
        "export_statement" => imports.extend(module_source_record(node, source, "export type ")),
        "call_expression" => imports.extend(dynamic_import_record(node, source)),
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_imports(child, source, imports);
    }
}

/// The specifier of a statement that names another module — `import … from
/// "x"` and re-exporting `export … from "x"`. A local `export const` carries no
/// `source` field, so it yields nothing.
fn module_source_record(
    node: Node<'_>,
    source: &[u8],
    type_only_prefix: &str,
) -> Option<ImportRecord> {
    let source_node = node.child_by_field_name("source")?;
    Some(ImportRecord {
        specifier: node_text(source_node, source)?,
        line: source_node.start_position().row + 1,
        is_type_only: node_starts_with(node, source, type_only_prefix),
        is_dynamic: false,
    })
}

fn node_starts_with(node: Node<'_>, source: &[u8], prefix: &str) -> bool {
    let Ok(text) = node.utf8_text(source) else {
        return false;
    };
    text.trim_start().starts_with(prefix)
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

#[cfg(test)]
mod tests {
    use crate::syntax::ImportRecord;
    use crate::syntax::scan_imports;
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
}
