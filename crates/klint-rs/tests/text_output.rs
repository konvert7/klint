use std::fs::{create_dir_all, remove_dir_all, write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const WARN_AND_ERROR_CONFIG: &str = r#"
include: ["src"]
rules:
  sonar/prefer-at: warn
arch:
  forbidden:
    - pattern: "console.log("
      in: "src/**"
      message: "Use logger"
"#;

const TS_ENGINE_STDERR_BEFORE_DURATION: &str = "klint: 1 warning(s)

src/index.ts:1  [sonar/prefer-at] ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  ⚠ Prefer arr.at(-1) over arr[arr.length - 1] for cleaner negative indexing.
klint: 1 error(s)

src/index.ts:2  [arch/forbidden] ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  × Use logger

klint: finished in ";

fn temp_root(name: &str) -> PathBuf {
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic enough for tests")
        .as_nanos();
    std::env::temp_dir().join(format!("klint-rs-text-{name}-{id}"))
}

fn write_fixture(root: &Path, config: &str, source: &str) {
    create_dir_all(root.join("src")).expect("create fixture dirs");
    write(root.join("klint.yaml"), config).expect("write config");
    write(root.join("src/index.ts"), source).expect("write source");
}

fn run_text(root: &Path) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_klint-rs"))
        .args(["--config", &root.to_string_lossy()])
        .env("KLINT_NO_STATS", "1")
        .output()
        .expect("klint-rs binary should execute");
    (
        output.status.code().expect("process should exit normally"),
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        String::from_utf8(output.stderr).expect("stderr should be utf8"),
    )
}

#[test]
fn text_mode_prints_warnings_then_errors_like_the_ts_engine() {
    let root = temp_root("mixed");
    write_fixture(
        &root,
        WARN_AND_ERROR_CONFIG,
        "const a = arr[arr.length - 1];\nconsole.log(a);\n",
    );

    let (status, stdout, stderr) = run_text(&root);

    assert_eq!(status, 2, "{stderr}");
    assert!(stdout.is_empty(), "{stdout}");
    assert!(
        stderr.starts_with(TS_ENGINE_STDERR_BEFORE_DURATION),
        "{stderr}"
    );
    assert!(
        stderr.ends_with("ms\n") || stderr.ends_with("s\n"),
        "{stderr}"
    );
    let _ = remove_dir_all(root);
}

#[test]
fn text_mode_with_only_warnings_exits_zero() {
    let root = temp_root("warn");
    write_fixture(
        &root,
        WARN_AND_ERROR_CONFIG,
        "const a = arr[arr.length - 1];\n",
    );

    let (status, _stdout, stderr) = run_text(&root);

    assert_eq!(status, 0, "{stderr}");
    assert!(stderr.starts_with("klint: 1 warning(s)\n\n"), "{stderr}");
    assert!(stderr.contains("\nklint: finished in "), "{stderr}");
    assert!(!stderr.contains("error(s)"), "{stderr}");
    let _ = remove_dir_all(root);
}

#[test]
fn text_mode_clean_run_reports_zero_violations_on_stdout() {
    let root = temp_root("clean");
    write_fixture(&root, WARN_AND_ERROR_CONFIG, "const a = 1;\n");

    let (status, stdout, stderr) = run_text(&root);

    assert_eq!(status, 0, "{stderr}");
    assert!(stdout.starts_with("klint: 0 violations in "), "{stdout}");
    assert!(stderr.is_empty(), "{stderr}");
    let _ = remove_dir_all(root);
}
