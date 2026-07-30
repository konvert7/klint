use std::collections::BTreeSet;
use tree_sitter::Node;

use super::super::ImportRecord;
use super::{
    first_string_child, python_dot_prefix, python_import_names, python_import_target_text,
    python_record,
};
use crate::syntax::node_text;

#[derive(Default)]
pub(super) struct ImportlibBindings {
    modules: BTreeSet<String>,
    functions: BTreeSet<String>,
}

pub(super) fn python_importlib_bindings(root: Node<'_>, source: &[u8]) -> ImportlibBindings {
    let mut bindings = ImportlibBindings::default();
    collect_importlib_bindings(root, source, &mut bindings);
    bindings
}

fn collect_importlib_bindings(node: Node<'_>, source: &[u8], bindings: &mut ImportlibBindings) {
    match node.kind() {
        "import_statement" => {
            for name in python_import_names(node) {
                if let Some(bound) = importlib_module_binding(name, source) {
                    bindings.modules.insert(bound);
                }
            }
        }
        "import_from_statement"
            if python_from_import_module(node, source).as_deref() == Some("importlib") =>
        {
            for name in python_import_names(node) {
                if python_import_target_text(name, source).as_deref() == Some("import_module") {
                    bindings.functions.insert(
                        python_import_alias(name, source).unwrap_or("import_module".to_string()),
                    );
                }
            }
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_importlib_bindings(child, source, bindings);
    }
}

fn importlib_module_binding(name: Node<'_>, source: &[u8]) -> Option<String> {
    let target = python_import_target_text(name, source)?;
    if target == "importlib" {
        return Some(python_import_alias(name, source).unwrap_or(target));
    }
    // `import importlib.util` binds the top-level package name, not the submodule.
    if target.starts_with("importlib.") && python_import_alias(name, source).is_none() {
        return Some("importlib".to_string());
    }
    None
}

fn python_import_alias(name: Node<'_>, source: &[u8]) -> Option<String> {
    if name.kind() != "aliased_import" {
        return None;
    }
    node_text(name.child_by_field_name("alias")?, source)
}

fn python_from_import_module(node: Node<'_>, source: &[u8]) -> Option<String> {
    let module = node.child_by_field_name("module_name")?;
    if module.kind() == "relative_import" {
        return None;
    }
    node_text(module, source)
}

pub(super) fn python_dynamic_import_record(
    node: Node<'_>,
    source: &[u8],
    importlib: &ImportlibBindings,
) -> Option<ImportRecord> {
    if !is_python_dynamic_import_call(node.child_by_field_name("function")?, source, importlib) {
        return None;
    }

    let arguments = node.child_by_field_name("arguments")?;
    let specifier = first_string_child(arguments)?;
    let record = python_record(
        python_dynamic_specifier(&node_text(specifier, source)?)?,
        specifier,
    );
    Some(ImportRecord {
        is_dynamic: true,
        ..record
    })
}

fn is_python_dynamic_import_call(
    function: Node<'_>,
    source: &[u8],
    importlib: &ImportlibBindings,
) -> bool {
    let Some(text) = node_text(function, source) else {
        return false;
    };
    match function.kind() {
        "identifier" => text == "__import__" || importlib.functions.contains(&text),
        "attribute" => function
            .child_by_field_name("object")
            .and_then(|object| node_text(object, source))
            .is_some_and(|object| {
                importlib.modules.contains(&object)
                    && function
                        .child_by_field_name("attribute")
                        .and_then(|attribute| node_text(attribute, source))
                        .as_deref()
                        == Some("import_module")
            }),
        _ => false,
    }
}

fn python_dynamic_specifier(raw: &str) -> Option<String> {
    let dots = raw.chars().take_while(|char| *char == '.').count();
    if dots == 0 {
        return (!raw.is_empty()).then(|| raw.to_string());
    }
    let module_path = raw[dots..].replace('.', "/");
    if module_path.is_empty() {
        return None;
    }
    Some(format!("{}{module_path}", python_dot_prefix(dots)))
}

#[cfg(test)]
mod tests {
    use crate::syntax::scan_imports;
    use std::path::PathBuf;

    #[test]
    fn extracts_python_dynamic_imports() {
        let records = scan_imports(
            &PathBuf::from("src/jobs/worker.py"),
            "import importlib\nfrom importlib import import_module as load\nfirst = importlib.import_module(\"requests\")\nsecond = load(\"app.lib.auth\")\nthird = __import__(\"json\")\nfourth = importlib.import_module(\".sibling\")\n",
        )
        .expect("python source should parse");

        assert_eq!(
            records
                .iter()
                .filter(|record| record.is_dynamic)
                .map(|record| (record.specifier.as_str(), record.line))
                .collect::<Vec<_>>(),
            vec![
                ("requests", 3),
                ("app.lib.auth", 4),
                ("json", 5),
                ("./sibling", 6),
            ]
        );
    }

    #[test]
    fn ignores_python_calls_that_are_not_bound_to_importlib() {
        let records = scan_imports(
            &PathBuf::from("src/jobs/worker.py"),
            "import_module(\"requests\")\nself.import_module(\"requests\")\nregistry.import_module(\"requests\")\n",
        )
        .expect("python source should parse");

        assert_eq!(records, vec![]);
    }
}
