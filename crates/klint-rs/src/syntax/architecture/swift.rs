use tree_sitter::Node;

use super::ImportRecord;
use crate::syntax::raw_node_text;

pub(super) fn walk_swift_imports(node: Node<'_>, source: &[u8], imports: &mut Vec<ImportRecord>) {
    if node.kind() == "import_declaration" {
        imports.extend(swift_import_record(node, source));
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_swift_imports(child, source, imports);
    }
}

/// The module a Swift `import` names. `import_declaration` exposes no field
/// names, so the module is the first `simple_identifier` under its `identifier`
/// child — one shape for `import Beta.Sub` and `import struct Models.User`
/// alike, with `@testable`/`@_exported` parked in a sibling `modifiers` node.
fn swift_import_record(node: Node<'_>, source: &[u8]) -> Option<ImportRecord> {
    let identifier = first_named_child_of_kind(node, "identifier")?;
    let module = first_named_child_of_kind(identifier, "simple_identifier")?;
    Some(ImportRecord {
        specifier: raw_node_text(module, source)?,
        line: identifier.start_position().row + 1,
        is_type_only: false,
        is_dynamic: false,
    })
}

fn first_named_child_of_kind<'tree>(node: Node<'tree>, kind: &str) -> Option<Node<'tree>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find(|child| child.kind() == kind)
}

#[cfg(test)]
mod tests {
    use crate::syntax::ImportRecord;
    use crate::syntax::scan_imports;
    use std::path::PathBuf;

    #[test]
    fn extracts_swift_imports_with_line_numbers() {
        let records = scan_imports(
            &PathBuf::from("Sources/App/UI/ViewModel.swift"),
            "import Foundation\n@_exported import Core\nimport struct Models.User\n// import Ignored\n",
        )
        .expect("swift imports should scan");

        assert_eq!(
            records,
            vec![
                ImportRecord {
                    specifier: "Foundation".to_string(),
                    line: 1,
                    is_type_only: false,
                    is_dynamic: false,
                },
                ImportRecord {
                    specifier: "Core".to_string(),
                    line: 2,
                    is_type_only: false,
                    is_dynamic: false,
                },
                ImportRecord {
                    specifier: "Models".to_string(),
                    line: 3,
                    is_type_only: false,
                    is_dynamic: false,
                },
            ]
        );
    }

    #[test]
    fn skips_swift_imports_inside_block_comments() {
        let records = scan_imports(
            &PathBuf::from("Sources/App/UI/ViewModel.swift"),
            "/*\nimport Core\n*/\n/* outer /* inner import Nested */ still */\nimport Foundation\n",
        )
        .expect("swift imports should scan");

        assert_eq!(
            records,
            vec![ImportRecord {
                specifier: "Foundation".to_string(),
                line: 5,
                is_type_only: false,
                is_dynamic: false,
            }]
        );
    }
}
