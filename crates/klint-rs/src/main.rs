mod command;
mod prompt;
mod text;

use command::{Dispatch, Group, Leaf, OptionSpec, Values, dispatch};
use klint_rs::{
    InstallRequest, MAX_FIX_PASSES, MigrateOptions, MigrateOutcome, RunOptions, RunSource,
    Violation, fix, install_skill, known_agent_names, known_agents, migrate, record_run, run,
    stats_report,
};
use std::path::PathBuf;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match dispatch(&klint_command(), &args, "klint") {
        Dispatch::Exit(code) => code,
        Dispatch::Run {
            command: Some("stats"),
            values,
        } => stats_command(&values),
        Dispatch::Run {
            command: Some("migrate"),
            values,
        } => migrate_command(&values),
        Dispatch::Run {
            command: Some(_),
            values,
        } => install_skill_command(&values),
        Dispatch::Run {
            command: None,
            values,
        } => lint_command(&values),
    };
    std::process::exit(code);
}

fn klint_command() -> Group {
    Group {
        summary: "Architecture-as-Code linter: layer boundaries, import rules and forbidden patterns",
        aliases: &[("h", "help")],
        default_command: Some(Leaf {
            summary: "Lints the project",
            options: vec![
                OptionSpec::valued(
                    "config",
                    "<dir>",
                    "Directory containing klint.yaml or klint.config.json (default: cwd)",
                ),
                OptionSpec::flag("json", "Emit structured JSON to stdout"),
                OptionSpec::flag("fix", "Apply autofixes, then report what is left"),
                OptionSpec::flag("hook", "Count this run as an agent hook run in klint stats"),
                OptionSpec::flag("version", "Print the installed klint version").short('V'),
            ],
            details: Vec::new(),
        }),
        commands: vec![
            (
                "stats",
                Leaf {
                    summary: "Print runs, violations reported and errors prevented, in total and per rule",
                    options: vec![OptionSpec::valued(
                        "config",
                        "<dir>",
                        "Directory holding .klint/stats.json (default: cwd)",
                    )],
                    details: vec!["Set KLINT_NO_STATS=1 to stop recording.".to_string()],
                },
            ),
            (
                "install-skill",
                Leaf {
                    summary: "Install the embedded klint-rules skill into agent config directories",
                    options: vec![
                        OptionSpec::valued(
                            "agents",
                            "<list>",
                            format!("Comma-separated: {} (default: all)", known_agent_names()),
                        ),
                        OptionSpec::flag(
                            "symlink",
                            "Keep one skill in .agents/skills and symlink the others to it (default)",
                        )
                        .aliases(&["--shared"]),
                        OptionSpec::flag("copy", "Give every agent directory its own copy"),
                        OptionSpec::flag("force", "Replace a skill directory klint did not install"),
                    ],
                    details: vec![
                        "Asks which agents and how to store the skill when run on a terminal without".to_string(),
                        "--agents, --symlink or --copy.".to_string(),
                    ],
                },
            ),
            (
                "migrate",
                Leaf {
                    summary: "Upgrade a v1 klint.yaml or klint.config.json to the v2 rule groups in place",
                    options: vec![
                        OptionSpec::valued(
                            "config",
                            "<dir>",
                            "Directory containing klint.yaml or klint.config.json (default: cwd)",
                        ),
                        OptionSpec::flag(
                            "check",
                            "Exit 1 when the config is v1, 0 when it is v2; never writes",
                        ),
                    ],
                    details: vec![
                        "Comments, blank lines and key order are kept. The config is written only when the findings stay identical.".to_string(),
                    ],
                },
            ),
        ],
        details: Vec::new(),
    }
}

fn config_dir(values: &Values) -> Result<PathBuf, String> {
    match values.text("config") {
        Some(dir) => Ok(PathBuf::from(dir)),
        None => {
            std::env::current_dir().map_err(|err| format!("klint: failed to resolve cwd: {err}"))
        }
    }
}

fn lint_command(values: &Values) -> i32 {
    let started_at = Instant::now();
    if values.flag("version") {
        println!("klint {}", klint_rs::reported_version());
        return 0;
    }
    let config_dir = match config_dir(values) {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let source = if values.flag("hook") {
        RunSource::Hook
    } else {
        RunSource::Direct
    };
    if values.flag("fix") {
        return fix_command(config_dir, source, started_at);
    }
    match run(RunOptions {
        config_dir: config_dir.clone(),
    }) {
        Ok(output) => {
            record_run(&config_dir, &output.violations, source);
            if values.flag("json") {
                println!(
                    "{}",
                    serde_json::to_string(&output).expect("klint-rs output should be serializable")
                );
            } else {
                let report = text::render(&output.violations, started_at.elapsed());
                print!("{}", report.stdout);
                eprint!("{}", report.stderr);
            }
            if output.summary.errors > 0 { 2 } else { 0 }
        }
        Err(err) => {
            eprintln!("{err}");
            1
        }
    }
}

fn migrate_command(values: &Values) -> i32 {
    let config_dir = match config_dir(values) {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let options = MigrateOptions {
        config_dir,
        check_only: values.flag("check"),
    };
    match migrate(&options) {
        Ok(MigrateOutcome::AlreadyV2 { file }) => {
            println!("klint: {file} is already v2 — nothing to migrate.");
            0
        }
        Ok(MigrateOutcome::Migrated { file }) => {
            println!("klint: migrated {file} to v2.");
            0
        }
        Ok(MigrateOutcome::NeedsMigration { file }) => {
            println!("klint: {file} uses the v1 format — run `klint migrate`.");
            1
        }
        Ok(MigrateOutcome::Refused { file, reason, .. }) => {
            eprintln!("{}", MigrateOutcome::refusal_message(&file, &reason));
            1
        }
        Err(err) => {
            eprintln!("{err}");
            1
        }
    }
}

fn refusal_warning(file: String, line: usize, reason: &str) -> Violation {
    Violation {
        file,
        line,
        rule: "klint/config".to_string(),
        message: format!("Cannot migrate this config to v2: {reason}"),
        severity: "warn".to_string(),
        fix: None,
    }
}

fn migrate_before_fixing(config_dir: &std::path::Path) -> Option<Violation> {
    let outcome = migrate(&MigrateOptions {
        config_dir: config_dir.to_path_buf(),
        check_only: false,
    });
    match outcome {
        Ok(MigrateOutcome::Migrated { file }) => {
            eprintln!("klint: migrated {file} to v2.");
            None
        }
        Ok(MigrateOutcome::Refused { file, line, reason }) => {
            Some(refusal_warning(file, line, &reason))
        }
        Ok(_) => None,
        Err(message) => Some(refusal_warning(
            config_dir.display().to_string(),
            1,
            &message,
        )),
    }
}

fn fix_command(config_dir: PathBuf, source: RunSource, started_at: Instant) -> i32 {
    let refusal = migrate_before_fixing(&config_dir);
    match fix(RunOptions {
        config_dir: config_dir.clone(),
    }) {
        Ok(mut outcome) => {
            if let Some(warning) = refusal {
                eprint!("{}", text::warnings_section(std::slice::from_ref(&warning)));
                outcome.remaining.violations.push(warning);
                outcome.remaining.summary.warnings += 1;
            }
            record_run(&config_dir, &outcome.remaining.violations, source);
            if outcome.stopped_at_pass_limit {
                eprintln!(
                    "klint: stopped after {MAX_FIX_PASSES} fix passes — some fixes keep producing new fixable violations."
                );
            }
            println!(
                "{} Finished in {}.",
                fix_summary(outcome.applied, outcome.remaining.violations.len()),
                text::format_duration(started_at.elapsed())
            );
            0
        }
        Err(err) => {
            eprintln!("{err}");
            1
        }
    }
}

fn fix_summary(applied: usize, remaining: usize) -> String {
    if remaining > 0 {
        format!(
            "klint: applied {applied} fix(es). {remaining} violation(s) require manual attention."
        )
    } else {
        format!("klint: applied {applied} fix(es). No remaining violations.")
    }
}

fn stats_command(values: &Values) -> i32 {
    match config_dir(values) {
        Ok(dir) => {
            print!("{}", stats_report(&dir));
            0
        }
        Err(err) => {
            eprintln!("{err}");
            1
        }
    }
}

fn install_skill_command(values: &Values) -> i32 {
    let (mut request, decided) = match install_request(values) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    if !decided && prompt::stdin_is_a_terminal() {
        match prompt::ask_install_choices(&known_agents()) {
            Ok(Some(choices)) if choices.agents.is_empty() => {
                println!("klint: no agents selected — nothing to install");
                return 0;
            }
            Ok(Some(choices)) => {
                request.agents = choices.agents;
                request.shared = choices.shared;
            }
            Ok(None) => {
                println!("klint: cancelled");
                return 0;
            }
            Err(err) => {
                eprintln!("{err}");
                return 1;
            }
        }
    }

    match install_skill(&request) {
        Ok(installed) => {
            for line in installed {
                println!("klint: {line}");
            }
            0
        }
        Err(err) => {
            eprintln!("{err}");
            1
        }
    }
}

fn install_request(values: &Values) -> Result<(InstallRequest, bool), String> {
    let root =
        std::env::current_dir().map_err(|err| format!("klint: failed to resolve cwd: {err}"))?;
    let agents: Vec<String> = values
        .text("agents")
        .unwrap_or_default()
        .split(',')
        .map(|agent| agent.trim().to_string())
        .filter(|agent| !agent.is_empty())
        .collect();
    let decided = values.text("agents").is_some() || values.flag("symlink") || values.flag("copy");
    Ok((
        InstallRequest {
            root,
            agents,
            shared: !values.flag("copy"),
            force: values.flag("force"),
        },
        decided,
    ))
}
