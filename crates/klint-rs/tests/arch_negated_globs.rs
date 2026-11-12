mod common;

use common::temp_root;
use klint_rs::{RunOptions, run};
use std::fs::{create_dir_all, remove_dir_all, write};

const LONG_COMMENT: &str = "// one\n// two\n// three\nexport const a = 1;\n";
const FILES: &[&str] = &[
    "skills/report/template/page.ts",
    "skills/report/tools/build.ts",
    "skills/slides/tools/render.ts",
];

fn files_flagged_by_comment_block_in(name: &str, scope: &str) -> Vec<String> {
    let root = temp_root(name);

    for file in FILES {
        let path = root.join(file);
        create_dir_all(path.parent().expect("fixture file has a parent")).expect("create dirs");
        write(path, LONG_COMMENT).expect("write source");
    }

    write(
        root.join("klint.yaml"),
        format!(
            "include: [\"skills\"]\nrules: {{}}\narch:\n  maxCommentBlock:\n    - limit: 2\n      in: {scope}\n"
        ),
    )
    .expect("write config");
    let output = run(RunOptions {
        config_dir: root.clone(),
    })
    .expect("valid config should run");
    let _ = remove_dir_all(root);
    let mut files: Vec<String> = output
        .violations
        .into_iter()
        .filter(|violation| violation.rule == "arch/max-comment-block")
        .map(|violation| violation.file)
        .collect();
    files.sort();
    files
}

#[test]
fn excluding_a_wildcard_folder_keeps_its_siblings_in_scope() {
    assert_eq!(
        files_flagged_by_comment_block_in(
            "negated-star",
            "[\"skills/**\", \"!skills/*/template/**\"]"
        ),
        vec![
            "skills/report/tools/build.ts",
            "skills/slides/tools/render.ts"
        ]
    );
}

#[test]
fn excluding_a_double_star_folder_keeps_its_siblings_in_scope() {
    assert_eq!(
        files_flagged_by_comment_block_in(
            "negated-double-star",
            "[\"skills/**\", \"!skills/**/template/**\"]"
        ),
        vec![
            "skills/report/tools/build.ts",
            "skills/slides/tools/render.ts"
        ]
    );
}

#[test]
fn excluding_a_literal_folder_still_works() {
    assert_eq!(
        files_flagged_by_comment_block_in(
            "negated-literal",
            "[\"skills/**\", \"!skills/report/template/**\"]"
        ),
        vec![
            "skills/report/tools/build.ts",
            "skills/slides/tools/render.ts"
        ]
    );
}

#[test]
fn excluding_a_dot_slash_folder_works_like_the_plain_path() {
    assert_eq!(
        files_flagged_by_comment_block_in(
            "negated-dot-slash",
            "[\"./skills/**\", \"!./skills/*/template/**\"]"
        ),
        vec![
            "skills/report/tools/build.ts",
            "skills/slides/tools/render.ts"
        ]
    );
}
