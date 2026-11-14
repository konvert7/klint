mod common;

use std::fs::{create_dir_all, remove_dir_all, write};
use std::path::PathBuf;
use std::process::Command;

const CONFIG: &str = "include: [\"src\"]\nrules:\n  core:\n    no-string-match: error\n    no-unguarded-json-parse: warn\n";
const SOURCE: &str = "const m = s.match(/x/);\nconst v = JSON.parse(raw);\n";

const JSON_BEFORE_COLOUR: &str = concat!(
    r#"{"violations":[{"file":"src/a.ts","line":1,"rule":"no-string-match","message":"Use RegExp.exec() instead of String.match() for non-global regexes — use /x/.exec(s) instead.","severity":"error","fix":{"endLine":1,"replacement":"const m = /x/.exec(s);","startLine":1}},"#,
    r#"{"file":"src/a.ts","line":2,"rule":"no-unguarded-json-parse","message":"JSON.parse() called without a surrounding try/catch — a malformed payload will throw an unhandled exception.","severity":"warn","fix":null}],"#,
    r#""summary":{"errors":1,"warnings":1,"fixable":1},"hint":"1 violation(s) can be fixed automatically: run `klint --fix`."}"#,
    "\n"
);

const HOOK_BEFORE_COLOUR: &str = "klint: 1 warning(s)

src/a.ts:2  [no-unguarded-json-parse] ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  ⚠ JSON.parse() called without a surrounding try/catch — a malformed payload will throw an unhandled exception.
klint: 1 error(s)

src/a.ts:1  [no-string-match] ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  × Use RegExp.exec() instead of String.match() for non-global regexes — use /x/.exec(s) instead.

klint: 1 violation(s) can be fixed automatically: run `klint --fix`.
";

fn fixture(name: &str) -> PathBuf {
    let root = common::temp_root(name);
    create_dir_all(root.join("src")).expect("create fixture dirs");
    write(root.join("klint.yaml"), CONFIG).expect("write config");
    write(root.join("src/a.ts"), SOURCE).expect("write source");
    root
}

fn klint(fixture_name: &str, flags: &[&str]) -> (String, String) {
    let root = fixture(fixture_name);
    let output = Command::new(env!("CARGO_BIN_EXE_klint-rs"))
        .args(["--config", &root.to_string_lossy()])
        .args(flags)
        .env("KLINT_NO_STATS", "1")
        .env_remove("NO_COLOR")
        .output()
        .expect("klint-rs binary should execute");
    let _ = remove_dir_all(&root);
    (
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        String::from_utf8(output.stderr).expect("stderr should be utf8"),
    )
}

fn without_timing(stderr: &str) -> String {
    stderr
        .split_inclusive('\n')
        .filter(|line| !line.starts_with("klint: finished in"))
        .collect::<String>()
        .trim_end_matches('\n')
        .to_string()
        + "\n"
}

#[test]
fn piped_text_output_has_no_escape_codes() {
    let (_, stderr) = klint("text-piped", &[]);
    assert!(!stderr.contains('\x1b'), "{stderr:?}");
}

#[test]
fn piped_text_output_underlines_the_offending_call() {
    let (_, stderr) = klint("text-carets", &[]);
    assert!(
        stderr.contains("src/a.ts:1:11  [no-string-match] "),
        "{stderr}"
    );
    assert!(
        stderr.contains("  > 1 │ const m = s.match(/x/);\n      │           ^^^^^^^^^^^^\n"),
        "{stderr}"
    );
    assert!(
        stderr.contains("  > 2 │ const v = JSON.parse(raw);\n      │           ^^^^^^^^^^^^^^^\n"),
        "{stderr}"
    );
}

#[test]
fn json_output_is_unchanged_byte_for_byte() {
    let (stdout, _) = klint("text-json", &["--json"]);
    assert_eq!(stdout, JSON_BEFORE_COLOUR);
}

#[test]
fn hook_text_output_is_unchanged_byte_for_byte() {
    let (_, stderr) = klint("text-hook", &["--hook"]);
    assert_eq!(without_timing(&stderr), HOOK_BEFORE_COLOUR);
}
