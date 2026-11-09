mod common;

use common::temp_root;
use klint_rs::{RunOptions, run};
use std::fs::{create_dir_all, remove_dir_all, write};

fn comment_findings(name: &str, arch: &str, source: &str) -> Vec<(usize, String)> {
    let root = temp_root(name);
    create_dir_all(root.join("src")).expect("create fixture dirs");
    write(
        root.join("klint.yaml"),
        format!("include: [\"src\"]\nrules: {{}}\narch:\n{arch}"),
    )
    .expect("write config");
    write(root.join("src/index.ts"), source).expect("write source");
    let output = run(RunOptions {
        config_dir: root.clone(),
    })
    .expect("valid config should run");
    let _ = remove_dir_all(root);
    output
        .violations
        .into_iter()
        .filter(|violation| violation.rule.starts_with("arch/max-comment"))
        .map(|violation| (violation.line, violation.message))
        .collect()
}

#[test]
fn slashes_inside_regex_literals_are_not_comment_lines() {
    let findings = comment_findings(
        "regex-not-comment",
        "  maxCommentBlock:\n    - limit: 2\n      in: [\"src/**\"]\n",
        "const PROTECTED: RegExp[] = [\n  /^\\/etc\\//,\n  /^\\/boot\\//,\n  /^\\/System\\//,\n];\n",
    );

    assert!(findings.is_empty(), "{findings:?}");
}

#[test]
fn comments_after_a_regex_with_backticks_still_count_toward_density() {
    let findings = comment_findings(
        "regex-backticks",
        "  maxCommentDensity:\n    - limit: 10\n      in: [\"src/**\"]\n",
        "export const clean = (t: string) =>\n  t\n    // strip fenced code\n    .replace(/```[\\s\\S]*?```/g, \"\")\n    // strip inline code\n    .replace(/`[^`]+`/g, \"\")\n    // collapse whitespace\n    .trim();\n",
    );

    assert_eq!(
        findings,
        vec![(
            1,
            "Comment density 37.5% exceeds the maximum of 10%".to_string()
        )]
    );
}
