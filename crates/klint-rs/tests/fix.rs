use std::fs::{create_dir_all, read_to_string, remove_dir_all, write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const FIXABLE_RULES_CONFIG: &str = r#"
include: ["src"]
rules:
  sonar/prefer-at: error
  sonar/prefer-nullish-coalescing-assign: error
  sonar/prefer-string-raw: error
  sonar/prefer-string-raw-regexp: error
  sonar/prefer-string-replaceall: error
  sonar/no-single-char-class: error
  no-string-match: error
arch:
  forbidden:
    - pattern: "console.log("
      in: "src/**"
      message: "Use logger"
"#;

const BEFORE: &str = r#"declare const arr: number[], text: string, repl: string, line: string, s: string;
let x: object | undefined;
const a = arr[arr.length - 1];
if (x == null) x = {};
const p = "C:\\Users\\Documents\\file.txt";
const r = new RegExp(`\\.foo`);
const t = text.replace(/foo/g, repl);
const c = /a[b]c/;
const m = line.match(/foo/i);
const both = s.match(/[b]/i);
console.log(a, p, r, t, c, m, both);
"#;

const AFTER_TS_ENGINE_FIX: &str = r#"declare const arr: number[], text: string, repl: string, line: string, s: string;
let x: object | undefined;
const a = arr.at(-1);
x ??= {};
const p = String.raw`C:\Users\Documents\file.txt`;
const r = new RegExp(String.raw`\.foo`);
const t = text.replaceAll("foo", repl);
const c = /abc/;
const m = new RegExp(/foo/i).exec(line);
const both = new RegExp(/b/i).exec(s);
console.log(a, p, r, t, c, m, both);
"#;

fn temp_root(name: &str) -> PathBuf {
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic enough for tests")
        .as_nanos();
    std::env::temp_dir().join(format!("klint-rs-fix-{name}-{id}"))
}

fn write_fixture(root: &Path, config: &str, source: &str) {
    create_dir_all(root.join("src")).expect("create fixture dirs");
    write(root.join("klint.yaml"), config).expect("write config");
    write(root.join("src/index.ts"), source).expect("write source");
}

fn run_fix(root: &Path) -> (i32, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_klint-rs"))
        .args(["--config", &root.to_string_lossy(), "--fix"])
        .env("KLINT_NO_STATS", "1")
        .output()
        .expect("klint-rs binary should execute");
    (
        output.status.code().expect("process should exit normally"),
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
    )
}

#[test]
fn fix_rewrites_every_fixable_rule_like_the_ts_engine() {
    let root = temp_root("parity");
    write_fixture(&root, FIXABLE_RULES_CONFIG, BEFORE);

    let (status, stdout) = run_fix(&root);

    assert_eq!(status, 0, "{stdout}");
    assert_eq!(
        read_to_string(root.join("src/index.ts")).expect("read fixed source"),
        AFTER_TS_ENGINE_FIX
    );
    assert!(
        stdout.starts_with(
            "klint: applied 9 fix(es). 1 violation(s) require manual attention. Finished in "
        ),
        "{stdout}"
    );
    let _ = remove_dir_all(root);
}

#[test]
fn fix_reports_no_remaining_violations_when_everything_was_fixable() {
    let root = temp_root("clean");
    write_fixture(
        &root,
        "include: [\"src\"]\nrules:\n  sonar/prefer-at: error\n",
        "const a = arr[arr.length - 1];\n",
    );

    let (status, stdout) = run_fix(&root);

    assert_eq!(status, 0, "{stdout}");
    assert_eq!(
        read_to_string(root.join("src/index.ts")).expect("read fixed source"),
        "const a = arr.at(-1);\n"
    );
    assert!(
        stdout.starts_with("klint: applied 1 fix(es). No remaining violations. Finished in "),
        "{stdout}"
    );
    let _ = remove_dir_all(root);
}

#[test]
fn fix_leaves_files_without_fixable_violations_untouched() {
    let root = temp_root("untouched");
    let source = "console.log(\"x\");\n";
    write_fixture(&root, FIXABLE_RULES_CONFIG, source);

    let (status, stdout) = run_fix(&root);

    assert_eq!(status, 0, "{stdout}");
    assert_eq!(
        read_to_string(root.join("src/index.ts")).expect("read source"),
        source
    );
    assert!(
        stdout.starts_with(
            "klint: applied 0 fix(es). 1 violation(s) require manual attention. Finished in "
        ),
        "{stdout}"
    );
    let _ = remove_dir_all(root);
}

const MULTI_LINE_BEFORE: &str = r#"declare const arr: number[], text: string, repl: string, line: string;
let x: object | undefined;
const a = arr[
  arr.length - 1
];
if (x == null) {
  x = {};
}
const r = new RegExp(
  `\\.foo`
);
const t = text.replace(
  /foo/g,
  repl
);
const m = line.match(
  /foo/i
);
console.log(a, r, t, m);
"#;

const MULTI_LINE_AFTER_TS_ENGINE_FIX: &str = r#"declare const arr: number[], text: string, repl: string, line: string;
let x: object | undefined;
const a = arr.at(-1);
x ??= {};
const r = new RegExp(
  String.raw`\.foo`
);
const t = text.replaceAll("foo", repl);
const m = line.match(
  /foo/i
);
console.log(a, r, t, m);
"#;

#[test]
fn fix_rewrites_multi_line_violations_like_the_ts_engine() {
    let root = temp_root("multi-line");
    write_fixture(&root, FIXABLE_RULES_CONFIG, MULTI_LINE_BEFORE);

    let (status, stdout) = run_fix(&root);

    assert_eq!(status, 0, "{stdout}");
    assert_eq!(
        read_to_string(root.join("src/index.ts")).expect("read fixed source"),
        MULTI_LINE_AFTER_TS_ENGINE_FIX
    );
    assert!(
        stdout.starts_with(
            "klint: applied 4 fix(es). 2 violation(s) require manual attention. Finished in "
        ),
        "{stdout}"
    );
    let _ = remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn fix_leaves_every_file_untouched_when_one_write_fails() {
    use std::os::unix::fs::PermissionsExt;

    let root = temp_root("all-or-nothing");
    let source = "const a = arr[arr.length - 1];\n";
    create_dir_all(root.join("src")).expect("create fixture dirs");
    write(
        root.join("klint.yaml"),
        "include: [\"src\"]\nrules:\n  sonar/prefer-at: error\n",
    )
    .expect("write config");
    write(root.join("src/a.ts"), source).expect("write a");
    write(root.join("src/b.ts"), source).expect("write b");
    let read_only = root.join("src/b.ts");
    std::fs::set_permissions(&read_only, std::fs::Permissions::from_mode(0o444))
        .expect("make b read-only");
    if std::fs::OpenOptions::new()
        .write(true)
        .open(&read_only)
        .is_ok()
    {
        let _ = remove_dir_all(root);
        return;
    }

    let (status, stdout) = run_fix(&root);

    assert_eq!(status, 1, "{stdout}");
    assert_eq!(
        read_to_string(root.join("src/a.ts")).expect("read a"),
        source
    );
    assert_eq!(read_to_string(&read_only).expect("read b"), source);
    std::fs::set_permissions(&read_only, std::fs::Permissions::from_mode(0o644))
        .expect("restore b");
    let _ = remove_dir_all(root);
}
