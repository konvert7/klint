use tree_sitter::Node;

use super::ImportRecord;
use crate::syntax::raw_node_text;

pub(super) fn walk_csharp_imports(node: Node<'_>, source: &[u8], imports: &mut Vec<ImportRecord>) {
    if node.kind() == "using_directive" {
        imports.extend(csharp_import_record(node, source));
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_csharp_imports(child, source, imports);
    }
}

/// The namespace a `using` directive names. `using`, `static`, and `global` are
/// anonymous tokens, so a plain, static, or global directive exposes only its
/// target type — `identifier` for `using System`, `qualified_name` for
/// `using System.Collections.Generic`. An alias directive
/// (`using Json = Newtonsoft.Json`) parks the alias under the `name` field and
/// keeps the target as a separate child, so the target is the type node that is
/// not the alias.
fn csharp_import_record(node: Node<'_>, source: &[u8]) -> Option<ImportRecord> {
    let alias = node.child_by_field_name("name");
    let mut cursor = node.walk();
    let target = node.named_children(&mut cursor).find(|child| {
        Some(*child) != alias && matches!(child.kind(), "identifier" | "qualified_name")
    })?;

    Some(ImportRecord {
        specifier: raw_node_text(target, source)?,
        line: node.start_position().row + 1,
        is_type_only: false,
        is_dynamic: false,
    })
}

#[cfg(test)]
mod tests {
    use crate::syntax::ImportRecord;
    use crate::syntax::scan_imports;
    use std::path::PathBuf;

    #[test]
    fn extracts_csharp_usings_with_line_numbers() {
        let records = scan_imports(
            &PathBuf::from("src/Web/HomeController.cs"),
            "using System;\nusing System.Collections.Generic;\nusing static System.Math;\nusing Json = Newtonsoft.Json;\nglobal using System.Linq;\n// using Ignored;\n",
        )
        .expect("csharp imports should scan");

        assert_eq!(
            records,
            vec![
                ImportRecord {
                    specifier: "System".to_string(),
                    line: 1,
                    is_type_only: false,
                    is_dynamic: false,
                },
                ImportRecord {
                    specifier: "System.Collections.Generic".to_string(),
                    line: 2,
                    is_type_only: false,
                    is_dynamic: false,
                },
                ImportRecord {
                    specifier: "System.Math".to_string(),
                    line: 3,
                    is_type_only: false,
                    is_dynamic: false,
                },
                ImportRecord {
                    specifier: "Newtonsoft.Json".to_string(),
                    line: 4,
                    is_type_only: false,
                    is_dynamic: false,
                },
                ImportRecord {
                    specifier: "System.Linq".to_string(),
                    line: 5,
                    is_type_only: false,
                    is_dynamic: false,
                },
            ]
        );
    }

    #[test]
    fn finds_namespace_scoped_usings() {
        let records = scan_imports(
            &PathBuf::from("src/Web/Startup.cs"),
            "namespace App.Web\n{\n    using App.Core;\n    class Startup {}\n}\n",
        )
        .expect("csharp imports should scan");

        assert_eq!(
            records,
            vec![ImportRecord {
                specifier: "App.Core".to_string(),
                line: 3,
                is_type_only: false,
                is_dynamic: false,
            }]
        );
    }
}
