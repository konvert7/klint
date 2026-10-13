mod command;
mod prompt;

use command::{Dispatch, Group, Leaf, OptionSpec, Values, dispatch};
use klint_rs::{
    InstallRequest, RunOptions, RunSource, install_skill, known_agent_names, known_agents,
    record_run, run, stats_report,
};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match dispatch(&klint_command(), &args, "klint-rs") {
        Dispatch::Exit(code) => code,
        Dispatch::Run {
            command: Some("stats"),
            values,
        } => stats_command(&values),
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
        summary: "Shadow Rust architecture engine",
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
                OptionSpec::flag("hook", "Count this run as an agent hook run in klint stats"),
                OptionSpec::flag("version", "Print the installed klint-rs version").short('V'),
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
        ],
        details: Vec::new(),
    }
}

fn config_dir(values: &Values) -> Result<PathBuf, String> {
    match values.text("config") {
        Some(dir) => Ok(PathBuf::from(dir)),
        None => {
            std::env::current_dir().map_err(|err| format!("klint-rs: failed to resolve cwd: {err}"))
        }
    }
}

fn lint_command(values: &Values) -> i32 {
    if values.flag("version") {
        println!("klint-rs {}", klint_rs::reported_version());
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
            } else if output.violations.is_empty() {
                println!("{}", serde_json::json!({ "output": "klint: 0 violations" }));
            }
            if output.summary.errors > 0 { 2 } else { 0 }
        }
        Err(err) => {
            eprintln!("{err}");
            1
        }
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
        std::env::current_dir().map_err(|err| format!("klint-rs: failed to resolve cwd: {err}"))?;
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
