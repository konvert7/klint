mod common;

use common::temp_root;
use std::fs::{create_dir_all, read_to_string, remove_dir_all, write};
use std::path::{Path, PathBuf};
use std::process::Command;

const CONFIG: &str = "include: [\"src\"]\nrules:\n  core:\n    naming-convention: error\n";

struct Project(PathBuf);

impl Drop for Project {
    fn drop(&mut self) {
        let _ = remove_dir_all(&self.0);
    }
}

fn project(name: &str, files: &[(&str, &str)]) -> Project {
    let root = temp_root(name);
    create_dir_all(root.join("src")).expect("create src");
    write(root.join("klint.yaml"), CONFIG).expect("write config");

    for (file, text) in files {
        write(root.join("src").join(file), text).expect("write source");
    }

    Project(root)
}

fn klint(root: &Path, args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_klint-rs"))
        .current_dir(root)
        .args(args)
        .env("KLINT_NO_STATS", "1")
        .output()
        .expect("klint-rs runs");

    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn messages(root: &Path) -> Vec<String> {
    let stdout = klint(root, &["--json"]);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("json output");
    let mut found: Vec<String> = json["violations"]
        .as_array()
        .expect("violations")
        .iter()
        .filter(|violation| violation["rule"] == "naming-convention")
        .map(|violation| {
            format!(
                "{}:{} {}",
                violation["file"].as_str().unwrap_or_default(),
                violation["line"],
                violation["message"].as_str().unwrap_or_default()
            )
        })
        .collect();
    found.sort();
    found
}

fn source(root: &Path, file: &str) -> String {
    read_to_string(root.join("src").join(file)).expect("read source")
}

#[test]
fn typescript_functions_and_classes_follow_biome_defaults() {
    let project = project(
        "naming-ts",
        &[(
            "a.ts",
            "function get_user() {}\nfunction FetchAll() {}\nfunction URL() {}\nfunction _private_helper() {}\nclass user_service {}\nclass HTTPClient {}\nclass Store {}\n",
        )],
    );

    assert_eq!(
        messages(&project.0),
        vec![
            "src/a.ts:1 Function name `get_user` should be camelCase, PascalCase or UPPERCASE — rename it to `getUser`.",
            "src/a.ts:5 Class name `user_service` should be PascalCase — rename it to `UserService`.",
            "src/a.ts:6 Class name `HTTPClient` should be PascalCase — rename it to `HttpClient`.",
        ]
    );
}

#[test]
fn nested_functions_may_not_be_uppercase() {
    let project = project(
        "naming-nested",
        &[(
            "a.ts",
            "function outer() {\n  function INNER() {}\n  return INNER;\n}\n",
        )],
    );

    assert_eq!(messages(&project.0).len(), 1);
    assert!(messages(&project.0)[0].starts_with("src/a.ts:2 Function name `INNER`"));
}

#[test]
fn python_rust_csharp_and_swift_use_their_own_conventions() {
    let project = project(
        "naming-langs",
        &[
            (
                "a.py",
                "def getUser():\n    pass\n\nclass TestCase:\n    def setUp(self):\n        pass\n\nclass user_store:\n    pass\n",
            ),
            (
                "b.rs",
                "fn getUser() {}\nstruct user_store;\nfn r#match() {}\nenum Color { Red }\n",
            ),
            (
                "c.cs",
                "class user_store { void getUser() {} void Save() {} }\ninterface IStore {}\n",
            ),
            (
                "d.swift",
                "func GetUser() {}\nclass user_store {}\nfunc load() {}\n",
            ),
        ],
    );
    let found = messages(&project.0);

    assert_eq!(
        found
            .iter()
            .map(|line| line.split(" should").next().unwrap_or_default())
            .collect::<Vec<_>>(),
        vec![
            "src/a.py:1 Function name `getUser`",
            "src/a.py:8 Class name `user_store`",
            "src/b.rs:1 Function name `getUser`",
            "src/b.rs:2 Type name `user_store`",
            "src/c.cs:1 Method name `getUser`",
            "src/c.cs:1 Type name `user_store`",
            "src/d.swift:1 Function name `GetUser`",
            "src/d.swift:2 Type name `user_store`",
        ]
    );
}

#[test]
fn fix_renames_a_local_function_and_its_references() {
    let project = project(
        "naming-fix",
        &[(
            "a.ts",
            "function get_user(id: string) {\n  return id;\n}\nconst obj = { get_user };\nconsole.log(get_user(\"a\"), obj);\n",
        )],
    );
    klint(&project.0, &["--fix"]);

    assert_eq!(
        source(&project.0, "a.ts"),
        "function getUser(id: string) {\n  return id;\n}\nconst obj = { get_user: getUser };\nconsole.log(getUser(\"a\"), obj);\n"
    );
}

#[test]
fn exported_names_and_name_clashes_get_no_fix() {
    let exported = "export function get_user() {}\n";
    let clash = "function get_user() {}\nconst getUser = 1;\nconsole.log(get_user, getUser);\n";
    let listed = "function get_user() {}\nexport { get_user };\n";
    let project = project(
        "naming-no-fix",
        &[("a.ts", exported), ("b.ts", clash), ("c.ts", listed)],
    );
    klint(&project.0, &["--fix"]);

    assert_eq!(source(&project.0, "a.ts"), exported);
    assert_eq!(source(&project.0, "b.ts"), clash);
    assert_eq!(source(&project.0, "c.ts"), listed);
}

#[test]
fn fix_safe_leaves_renames_alone() {
    let original = "function get_user() {}\nget_user();\n";
    let project = project("naming-fix-safe", &[("a.ts", original)]);
    klint(&project.0, &["--fix=safe"]);

    assert_eq!(source(&project.0, "a.ts"), original);
}
