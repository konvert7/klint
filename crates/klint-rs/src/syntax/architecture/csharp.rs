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

/// Every fully-qualified namespace a file declares. Block and file-scoped
/// declarations both expose the namespace under the `name` field; a block
/// namespace nested inside another contributes the ancestors too, so
/// `namespace A { namespace B {} }` declares both `A` and `A.B` — which is what
/// a `using A.B` must resolve against.
pub(super) fn walk_csharp_namespaces(node: Node<'_>, source: &[u8], out: &mut Vec<String>) {
    walk_namespaces_with_prefix(node, source, "", out);
}

fn walk_namespaces_with_prefix(node: Node<'_>, source: &[u8], prefix: &str, out: &mut Vec<String>) {
    let mut child_prefix = prefix.to_string();
    if matches!(
        node.kind(),
        "namespace_declaration" | "file_scoped_namespace_declaration"
    ) && let Some(name) = node
        .child_by_field_name("name")
        .and_then(|name| raw_node_text(name, source))
    {
        child_prefix = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}.{name}")
        };
        out.push(child_prefix.clone());
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_namespaces_with_prefix(child, source, &child_prefix, out);
    }
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
    fn collects_file_scoped_and_nested_namespaces() {
        assert_eq!(
            crate::syntax::scan_csharp_namespaces("namespace App.Web;\nclass C {}\n"),
            vec!["App.Web".to_string()]
        );
        assert_eq!(
            crate::syntax::scan_csharp_namespaces(
                "namespace App\n{\n    namespace Core\n    {\n        class C {}\n    }\n}\n"
            ),
            vec!["App".to_string(), "App.Core".to_string()]
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
