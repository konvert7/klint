use super::*;
use std::fs::{create_dir_all, write};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_root(name: &str) -> PathBuf {
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic enough for tests")
        .as_nanos();
    std::env::temp_dir().join(format!("klint-rs-files-{name}-{id}"))
}

#[test]
fn the_walk_prunes_an_excluded_directory_but_never_a_file() {
    let root = Path::new("/repo");
    let excludes = vec!["src/gen/**".to_string()];
    let keeps =
        |path: &str, is_dir: bool| keeps_entry(&root.join(path), is_dir, root, root, &excludes);

    assert!(!keeps("src/gen", true));
    assert!(keeps("src/gen", false));
    assert!(keeps("src", true));
    assert!(!keeps_entry(
        &root.join("src/gen"),
        true,
        root,
        root,
        &["src/gen/".to_string()]
    ));
}

#[test]
fn the_walk_keeps_only_paths_on_the_way_to_or_under_its_base() {
    let root = Path::new("/repo");
    let base = root.join("src/app");
    let keeps = |path: &str| keeps_entry(&root.join(path), true, &base, root, &[]);

    assert!(keeps("src"));
    assert!(keeps("src/app/ui"));
    assert!(!keeps("docs"));
}

#[test]
fn a_trailing_star_matches_nothing_more() {
    assert!(match_component("a*", "a"));
    assert!(match_component("a**", "a"));
    assert!(!match_component("ab*", "a"));
}

#[test]
fn resolve_files_prunes_nested_excluded_directories() {
    let root = temp_root("exclude-node-modules");
    create_dir_all(root.join("src")).expect("create src");
    create_dir_all(root.join("assets/skill/node_modules/pkg")).expect("create nested node_modules");
    write(root.join("src/index.ts"), "export const value = 1;\n").expect("write source");
    write(
        root.join("assets/skill/node_modules/pkg/index.ts"),
        "export const dependency = 1;\n",
    )
    .expect("write dependency source");

    let files = resolve_files(
        &root,
        &[
            "src".to_string(),
            "assets".to_string(),
            "!**/node_modules/**".to_string(),
        ],
        true,
        &[],
    )
    .expect("resolve files");

    let rel_files = files
        .iter()
        .map(|file| relative_path(&root, file))
        .collect::<Vec<_>>();
    assert_eq!(rel_files, vec!["src/index.ts"]);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_files_includes_multilanguage_sources_for_architecture_rules() {
    let root = temp_root("multilanguage-source");
    create_dir_all(root.join("src/app")).expect("create src dirs");
    write(root.join("src/app/main.py"), "print('x')\n").expect("write python source");
    write(root.join("src/app/main.swift"), "print(\"x\")\n").expect("write swift source");
    write(root.join("src/app/main.ts"), "console.log('x');\n").expect("write ts source");
    write(root.join("src/app/readme.md"), "# ignored\n").expect("write markdown");

    let files = resolve_files(&root, &["src".to_string()], true, &[]).expect("resolve files");

    let rel_files = files
        .iter()
        .map(|file| relative_path(&root, file))
        .collect::<Vec<_>>();
    assert_eq!(
        rel_files,
        vec!["src/app/main.py", "src/app/main.swift", "src/app/main.ts"]
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn match_pattern_treats_a_leading_dot_slash_as_the_root() {
    for pattern in ["./src", "./src/**", "./src/*.ts", "./**/*.ts"] {
        assert!(match_pattern("src/app.ts", pattern), "{pattern}");
    }

    assert!(!match_pattern("lib/app.ts", "./src/**"));
}

#[test]
fn resolve_files_excludes_dot_slash_patterns() {
    let root = temp_root("dot-slash-exclude");
    create_dir_all(root.join("src/gen")).expect("create src/gen");
    write(root.join("src/app.ts"), "export const x = 1;\n").expect("write app");
    write(root.join("src/gen/out.ts"), "export const y = 2;\n").expect("write gen");

    let files = resolve_files(
        &root,
        &["./src/**/*.ts".to_string(), "!./src/gen".to_string()],
        true,
        &[],
    )
    .expect("resolve files");

    let rel_files = files
        .iter()
        .map(|file| relative_path(&root, file))
        .collect::<Vec<_>>();
    assert_eq!(rel_files, vec!["src/app.ts"]);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_files_matches_root_level_files_under_double_star_glob() {
    // `**` means zero-or-more path segments, so `**/*.ts` must match a root-level
    // file. Regression test for include_base collapsing it to a literal directory.
    let root = temp_root("double-star-root");
    create_dir_all(root.join("src")).expect("create src");
    write(root.join("cli.ts"), "export const x = 1;\n").expect("write root source");
    write(root.join("src/app.ts"), "export const y = 2;\n").expect("write nested source");

    let files = resolve_files(&root, &["**/*.ts".to_string()], true, &[]).expect("resolve files");

    let rel_files = files
        .iter()
        .map(|file| relative_path(&root, file))
        .collect::<Vec<_>>();
    assert_eq!(rel_files, vec!["cli.ts", "src/app.ts"]);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_files_honors_glob_extension_not_just_base_dir() {
    // `**/*.ts` selects .ts only — the walker must match each file against the
    // glob, not collect every supported source type under the base directory.
    let root = temp_root("glob-extension");
    create_dir_all(root.join("src")).expect("create src");
    write(root.join("src/a.ts"), "export const x = 1;\n").expect("write ts");
    write(root.join("src/a.js"), "export const y = 2;\n").expect("write js");
    write(root.join("src/a.py"), "y = 2\n").expect("write py");

    let files = resolve_files(&root, &["**/*.ts".to_string()], true, &[]).expect("resolve files");

    let rel_files = files
        .iter()
        .map(|file| relative_path(&root, file))
        .collect::<Vec<_>>();
    assert_eq!(rel_files, vec!["src/a.ts"]);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_files_excludes_individual_files_not_only_directories() {
    let root = temp_root("exclude-file-glob");
    create_dir_all(root.join("src/jobs")).expect("create src dirs");
    write(root.join("src/jobs/worker.py"), "y = 1\n").expect("write worker");
    write(root.join("src/jobs/worker.test.py"), "y = 2\n").expect("write worker test");
    write(root.join("src/jobs/keep.py"), "y = 3\n").expect("write keep");

    let files = resolve_files(
        &root,
        &[
            "src".to_string(),
            "!**/*.test.py".to_string(),
            "!src/jobs/worker.py".to_string(),
        ],
        true,
        &[],
    )
    .expect("resolve files");

    let rel_files = files
        .iter()
        .map(|file| relative_path(&root, file))
        .collect::<Vec<_>>();
    assert_eq!(rel_files, vec!["src/jobs/keep.py"]);

    let _ = fs::remove_dir_all(root);
}
