mod dynamic;

use tree_sitter::Node;

use super::{ImportRecord, first_string_child};
use crate::syntax::node_text;
use dynamic::{ImportlibBindings, python_dynamic_import_record, python_importlib_bindings};

pub(super) fn scan_python_imports(root: Node<'_>, source: &[u8]) -> Vec<ImportRecord> {
    let mut imports = Vec::new();
    let importlib = python_importlib_bindings(root, source);
    walk_python_imports(root, source, &importlib, false, &mut imports);
    imports
}

fn walk_python_imports(
    node: Node<'_>,
    source: &[u8],
    importlib: &ImportlibBindings,
    type_only: bool,
    imports: &mut Vec<ImportRecord>,
) {
    match node.kind() {
        "import_statement" => {
            imports.extend(with_type_only(
                python_import_records(node, source),
                type_only,
            ));
        }
        "import_from_statement" => {
            imports.extend(with_type_only(
                python_from_import_records(node, source),
                type_only,
            ));
        }
        "call" => {
            imports.extend(with_type_only(
                python_dynamic_import_record(node, source, importlib),
                type_only,
            ));
        }
        _ => {}
    }

    let guarded_branch = guards_type_checking(node, source)
        .then(|| node.child_by_field_name("consequence"))
        .flatten();

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let child_type_only =
            type_only || guarded_branch.is_some_and(|branch| branch.id() == child.id());
        walk_python_imports(child, source, importlib, child_type_only, imports);
    }
}

fn with_type_only(
    records: impl IntoIterator<Item = ImportRecord>,
    type_only: bool,
) -> impl Iterator<Item = ImportRecord> {
    records.into_iter().map(move |record| ImportRecord {
        is_type_only: type_only,
        ..record
    })
}

/// An `if TYPE_CHECKING:` statement, whose consequence block runs only under a
/// type checker. `typing.TYPE_CHECKING` and `TYPE_CHECKING and <version test>`
/// count; `not TYPE_CHECKING` does not, so its else-branch stays a runtime
/// import rather than being silently exempted.
fn guards_type_checking(node: Node<'_>, source: &[u8]) -> bool {
    node.kind() == "if_statement"
        && node
            .child_by_field_name("condition")
            .is_some_and(|condition| is_type_checking_test(condition, source))
}

fn is_type_checking_test(node: Node<'_>, source: &[u8]) -> bool {
    match node.kind() {
        "identifier" => node_text(node, source).as_deref() == Some("TYPE_CHECKING"),
        "attribute" => {
            node.child_by_field_name("attribute")
                .and_then(|attribute| node_text(attribute, source))
                .as_deref()
                == Some("TYPE_CHECKING")
        }
        "parenthesized_expression" => node
            .named_child(0)
            .is_some_and(|inner| is_type_checking_test(inner, source)),
        "boolean_operator" => {
            has_child_kind(node, "and")
                && ["left", "right"].iter().any(|field| {
                    node.child_by_field_name(field)
                        .is_some_and(|operand| is_type_checking_test(operand, source))
                })
        }
        _ => false,
    }
}

fn has_child_kind(node: Node<'_>, kind: &str) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor).any(|child| child.kind() == kind)
}

/// Names that `importlib` and `importlib.import_module` are bound to in this
/// file, so `il.import_module("x")` and a renamed `from importlib import
/// import_module as load` are both recognised while an unrelated
/// `self.import_module(...)` is not.
fn python_import_records(node: Node<'_>, source: &[u8]) -> Vec<ImportRecord> {
    python_import_names(node)
        .into_iter()
        .filter_map(|name| {
            Some(python_record(
                python_import_target_text(name, source)?,
                name,
            ))
        })
        .collect()
}

fn python_from_import_records(node: Node<'_>, source: &[u8]) -> Vec<ImportRecord> {
    let Some(module) = node.child_by_field_name("module_name") else {
        return Vec::new();
    };

    if module.kind() != "relative_import" {
        return node_text(module, source)
            .map(|specifier| vec![python_record(specifier, module)])
            .unwrap_or_default();
    }

    let Some(prefix) = python_relative_prefix(module, source) else {
        return Vec::new();
    };

    if let Some(module_path) = python_relative_module_path(module, source) {
        return vec![python_record(format!("{prefix}{module_path}"), module)];
    }

    python_import_names(node)
        .into_iter()
        .filter_map(|name| {
            let target = python_import_target_text(name, source)?;
            Some(python_record(format!("{prefix}{target}"), name))
        })
        .collect()
}

pub(super) fn python_import_names<'tree>(node: Node<'tree>) -> Vec<Node<'tree>> {
    let mut cursor = node.walk();
    node.children_by_field_name("name", &mut cursor).collect()
}

pub(super) fn python_import_target_text(name: Node<'_>, source: &[u8]) -> Option<String> {
    let target = if name.kind() == "aliased_import" {
        name.child_by_field_name("name")?
    } else {
        name
    };
    node_text(target, source)
}

fn python_relative_prefix(module: Node<'_>, source: &[u8]) -> Option<String> {
    let mut cursor = module.walk();
    let prefix = module
        .children(&mut cursor)
        .find(|child| child.kind() == "import_prefix")?;
    let dots = node_text(prefix, source)?
        .chars()
        .filter(|char| *char == '.')
        .count();
    Some(python_dot_prefix(dots))
}

pub(super) fn python_dot_prefix(dots: usize) -> String {
    if dots <= 1 {
        "./".to_string()
    } else {
        "../".repeat(dots - 1)
    }
}

fn python_relative_module_path(module: Node<'_>, source: &[u8]) -> Option<String> {
    let mut cursor = module.walk();
    let dotted = module
        .children(&mut cursor)
        .find(|child| child.kind() == "dotted_name")?;
    Some(node_text(dotted, source)?.replace('.', "/"))
}

pub(super) fn python_record(specifier: String, node: Node<'_>) -> ImportRecord {
    ImportRecord {
        specifier,
        line: node.start_position().row + 1,
        is_type_only: false,
        is_dynamic: false,
    }
}

#[cfg(test)]
mod tests {
    use crate::syntax::ImportRecord;
    use crate::syntax::scan_imports;
    use std::path::PathBuf;

    #[test]
    fn extracts_python_relative_imports_with_line_numbers() {
        let records = scan_imports(
            &PathBuf::from("src/jobs/worker.py"),
            "import requests\nfrom ..lib.auth import load_key\nfrom . import local\n",
        )
        .expect("python source should parse");

        assert_eq!(
            records,
            vec![
                ImportRecord {
                    specifier: "requests".to_string(),
                    line: 1,
                    is_type_only: false,
                    is_dynamic: false,
                },
                ImportRecord {
                    specifier: "../lib/auth".to_string(),
                    line: 2,
                    is_type_only: false,
                    is_dynamic: false,
                },
                ImportRecord {
                    specifier: "./local".to_string(),
                    line: 3,
                    is_type_only: false,
                    is_dynamic: false,
                },
            ]
        );
    }

    #[test]
    fn extracts_every_target_of_a_multi_target_python_import() {
        let records = scan_imports(
            &PathBuf::from("src/jobs/worker.py"),
            "import os, sys\nimport os.path as p, json\n",
        )
        .expect("python source should parse");

        assert_eq!(
            records,
            vec![
                ImportRecord {
                    specifier: "os".to_string(),
                    line: 1,
                    is_type_only: false,
                    is_dynamic: false,
                },
                ImportRecord {
                    specifier: "sys".to_string(),
                    line: 1,
                    is_type_only: false,
                    is_dynamic: false,
                },
                ImportRecord {
                    specifier: "os.path".to_string(),
                    line: 2,
                    is_type_only: false,
                    is_dynamic: false,
                },
                ImportRecord {
                    specifier: "json".to_string(),
                    line: 2,
                    is_type_only: false,
                    is_dynamic: false,
                },
            ]
        );
    }

    #[test]
    fn extracts_every_target_of_a_bare_relative_python_import() {
        let records = scan_imports(
            &PathBuf::from("src/jobs/worker.py"),
            "from . import first, second as alias\nfrom ..lib import (\n    auth,\n    other,\n)\n",
        )
        .expect("python source should parse");

        assert_eq!(
            records,
            vec![
                ImportRecord {
                    specifier: "./first".to_string(),
                    line: 1,
                    is_type_only: false,
                    is_dynamic: false,
                },
                ImportRecord {
                    specifier: "./second".to_string(),
                    line: 1,
                    is_type_only: false,
                    is_dynamic: false,
                },
                ImportRecord {
                    specifier: "../lib".to_string(),
                    line: 2,
                    is_type_only: false,
                    is_dynamic: false,
                },
            ]
        );
    }

    #[test]
    fn ignores_python_wildcard_imports_without_a_module_path() {
        let records = scan_imports(
            &PathBuf::from("src/jobs/worker.py"),
            "from . import *\nfrom app.lib import *\n",
        )
        .expect("python source should parse");

        assert_eq!(
            records,
            vec![ImportRecord {
                specifier: "app.lib".to_string(),
                line: 2,
                is_type_only: false,
                is_dynamic: false,
            }]
        );
    }

    #[test]
    fn marks_python_imports_guarded_by_type_checking() {
        let records = scan_imports(
            &PathBuf::from("src/jobs/worker.py"),
            "from typing import TYPE_CHECKING\n\nif TYPE_CHECKING:\n    from app.lib import shapes\n\nif typing.TYPE_CHECKING:\n    from app.lib import aliases\n\nif TYPE_CHECKING and sys.version_info >= (3, 11):\n    from app.lib import modern\n\nif TYPE_CHECKING:\n    if sys.platform == \"linux\":\n        from app.lib import nested\n",
        )
        .expect("python source should parse");

        assert_eq!(
            records
                .iter()
                .map(|record| (record.specifier.as_str(), record.is_type_only))
                .collect::<Vec<_>>(),
            vec![
                ("typing", false),
                ("app.lib", true),
                ("app.lib", true),
                ("app.lib", true),
                ("app.lib", true),
            ]
        );
    }

    #[test]
    fn keeps_runtime_branches_of_a_type_checking_guard_as_value_imports() {
        let records = scan_imports(
            &PathBuf::from("src/jobs/worker.py"),
            "if TYPE_CHECKING:\n    from app.lib import shapes\nelse:\n    from app.lib import fallback\n\nif TYPE_CHECKING:\n    from app.lib import other\nelif legacy:\n    from app.lib import old\n\nif not TYPE_CHECKING:\n    from app.lib import runtime\n\nif TYPE_CHECKING or debug:\n    from app.lib import loose\n",
        )
        .expect("python source should parse");

        assert_eq!(
            records
                .iter()
                .map(|record| record.is_type_only)
                .collect::<Vec<_>>(),
            vec![true, false, true, false, false, false]
        );
    }
}
