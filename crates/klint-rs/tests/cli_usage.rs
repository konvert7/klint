mod common;

use common::temp_root;
use std::fs::{self, create_dir_all, remove_dir_all, write};
use std::path::{Path, PathBuf};
use std::process::Command;

struct Outcome {
    exit: i32,
    stdout: String,
    stderr: String,
}

fn project(name: &str) -> PathBuf {
    let root = temp_root(name);
    create_dir_all(&root).expect("create project");
    write(root.join("klint.yaml"), "include: [\".\"]\n").expect("write config");
    root
}

fn klint(cwd: &Path, args: &[&str], env: &[(&str, &str)]) -> Outcome {
    let output = Command::new(env!("CARGO_BIN_EXE_klint-rs"))
        .args(args)
        .current_dir(cwd)
        .env_remove("KLINT_NO_STATS")
        .envs(env.iter().copied())
        .output()
        .expect("klint-rs runs");
    Outcome {
        exit: output.status.code().expect("exits normally"),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

#[test]
fn every_help_form_prints_usage_and_runs_nothing() {
    let root = project("cli-help");
    let commands =
        regex::Regex::new(r"Commands:\n {2}stats +\S.*\n {2}install-skill +\S").expect("regex");

    for flag in ["help", "-h", "--help"] {
        let out = klint(&root, &[flag], &[]);
        assert_eq!((out.exit, out.stderr.as_str()), (0, ""), "{flag}");
        assert!(
            out.stdout.starts_with("Usage: klint [<command>] [options]"),
            "{flag}: {}",
            out.stdout
        );
        assert!(commands.is_match(&out.stdout), "{flag}: {}", out.stdout);
    }

    for command in ["stats", "install-skill"] {
        for flag in ["help", "-h", "--help"] {
            let out = klint(&root, &[command, flag], &[]);
            assert_eq!(out.exit, 0, "{command} {flag}");
            assert!(
                out.stdout
                    .starts_with(&format!("Usage: klint {command} [options]")),
                "{command} {flag}: {}",
                out.stdout
            );
            assert!(out.stdout.contains("-h, --help"), "{command} {flag}");
        }
    }

    assert!(!root.join(".klint").exists());
    assert!(!root.join(".agents").exists());
    let _ = remove_dir_all(root);
}

#[test]
fn usage_errors_exit_one_with_the_usage_of_the_command() {
    let root = project("cli-usage-errors");
    let cases: &[(&[&str], &str)] = &[
        (
            &["--json", "--bogus"],
            "error: unknown option '--bogus'\n\nUsage: klint [<command>]",
        ),
        (
            &["stats", "--bogus"],
            "error: unknown option '--bogus'\n\nUsage: klint stats [options]",
        ),
        (
            &["install-skill", "--agents", "claude", "--bogus"],
            "error: unknown option '--bogus'\n\nUsage: klint install-skill [options]",
        ),
        (
            &["nope"],
            "error: unknown command 'nope'\n\nUsage: klint [<command>]",
        ),
        (
            &["stats", "--config"],
            "error: option '--config' needs <dir>\n\nUsage:",
        ),
    ];

    for (args, stderr_start) in cases {
        let out = klint(&root, args, &[]);
        assert_eq!((out.exit, out.stdout.as_str()), (1, ""), "{args:?}");
        assert!(
            out.stderr.starts_with(stderr_start),
            "{args:?}: {}",
            out.stderr
        );
    }

    assert!(!root.join(".klint").exists());
    assert!(!root.join(".agents").exists());
    assert!(!root.join(".claude").exists());
    let _ = remove_dir_all(root);
}

#[test]
fn klint_no_stats_leaves_no_stats_file() {
    let root = project("cli-no-stats");
    write(root.join("a.ts"), "debugger;\n").expect("write source");

    klint(&root, &[], &[("KLINT_NO_STATS", "1")]);
    assert!(!root.join(".klint/stats.json").exists());

    klint(&root, &[], &[]);
    assert!(root.join(".klint/stats.json").exists());
    let _ = remove_dir_all(root);
}

fn shipped_skill() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../skill/klint-rules/SKILL.md");
    fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

#[test]
fn install_skill_without_a_terminal_shares_one_copy_across_agents() {
    let root = temp_root("cli-install-skill");
    create_dir_all(&root).expect("create project");

    let out = klint(&root, &["install-skill"], &[]);

    assert_eq!(out.exit, 0, "{}", out.stderr);
    let canonical = root.join(".agents/skills/klint-rules");
    assert!(
        fs::symlink_metadata(&canonical)
            .expect("canonical skill")
            .is_dir()
    );
    assert!(canonical.join(".klint-skill.json").is_file());

    for agent in [".claude", ".cursor"] {
        let dir = root.join(agent).join("skills/klint-rules");
        assert_eq!(
            fs::read_to_string(dir.join("SKILL.md")).expect("agent SKILL.md"),
            shipped_skill(),
            "{agent}"
        );

        match fs::read_link(&dir) {
            Ok(target) => assert!(
                !target.to_string_lossy().contains("node_modules"),
                "{agent}"
            ),
            Err(_) => assert!(dir.join(".klint-skill.json").is_file(), "{agent}"),
        }
    }

    write(root.join("klint.yaml"), "include: [\".\"]\n").expect("write config");
    let lint = klint(&root, &["--json"], &[("KLINT_NO_STATS", "1")]);
    assert!(!lint.stdout.contains("klint/skill-"), "{}", lint.stdout);
    let _ = remove_dir_all(root);
}
