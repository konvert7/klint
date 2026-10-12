mod prompt;

use klint_rs::{
    InstallRequest, RunOptions, RunSource, install_skill, known_agent_names, known_agents,
    record_run, run, stats_report,
};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.first().is_some_and(|arg| arg == "install-skill") {
        std::process::exit(install_skill_command(&args[1..]));
    }
    if args.first().is_some_and(|arg| arg == "stats") {
        std::process::exit(stats_command(&args[1..]));
    }

    let options = parse_args(&args).unwrap_or_else(|err| {
        eprintln!("{err}");
        std::process::exit(1);
    });

    match run(RunOptions {
        config_dir: options.config_dir.clone(),
    }) {
        Ok(output) => {
            record_run(&options.config_dir, &output.violations, options.source);
            if options.json {
                println!(
                    "{}",
                    serde_json::to_string(&output).expect("klint-rs output should be serializable")
                );
            } else if output.violations.is_empty() {
                println!("{}", serde_json::json!({ "output": "klint: 0 violations" }));
            }
            std::process::exit(if output.summary.errors > 0 { 2 } else { 0 });
        }
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}

struct CliOptions {
    config_dir: PathBuf,
    json: bool,
    source: RunSource,
}

fn parse_args(args: &[String]) -> Result<CliOptions, String> {
    let mut config_dir =
        std::env::current_dir().map_err(|err| format!("klint-rs: failed to resolve cwd: {err}"))?;
    let mut json = false;
    let mut source = RunSource::Direct;
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--config" => {
                let Some(value) = args.get(i + 1) else {
                    return Err("klint-rs: --config requires a directory".to_string());
                };
                config_dir = PathBuf::from(value);
                i += 2;
            }
            "--json" => {
                json = true;
                i += 1;
            }
            "--hook" => {
                source = RunSource::Hook;
                i += 1;
            }
            "--help" | "-h" | "help" | "h" => {
                print_help();
                std::process::exit(0);
            }
            "--version" | "-V" | "version" => {
                println!("klint-rs {}", klint_rs::reported_version());
                std::process::exit(0);
            }
            other => {
                return Err(format!("klint-rs: unknown argument \"{other}\""));
            }
        }
    }

    Ok(CliOptions {
        config_dir,
        json,
        source,
    })
}

fn stats_command(args: &[String]) -> i32 {
    let config_dir = match args {
        [] => std::env::current_dir(),
        [flag, dir] if flag == "--config" => Ok(PathBuf::from(dir)),
        _ => {
            eprintln!("klint-rs: usage: stats [--config <dir>]");
            return 1;
        }
    };
    match config_dir {
        Ok(dir) => {
            print!("{}", stats_report(&dir));
            0
        }
        Err(err) => {
            eprintln!("klint-rs: failed to resolve cwd: {err}");
            1
        }
    }
}

fn install_skill_command(args: &[String]) -> i32 {
    let (mut request, decided) = match parse_install_args(args) {
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

fn parse_install_args(args: &[String]) -> Result<(InstallRequest, bool), String> {
    let root =
        std::env::current_dir().map_err(|err| format!("klint-rs: failed to resolve cwd: {err}"))?;
    let mut agents: Vec<String> = Vec::new();
    let mut shared = true;
    let mut force = false;
    let mut decided = false;
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--agents" => {
                let Some(value) = args.get(i + 1) else {
                    return Err(format!(
                        "klint-rs: --agents requires a comma-separated list of: {}",
                        known_agent_names()
                    ));
                };
                agents.extend(
                    value
                        .split(',')
                        .map(|agent| agent.trim().to_string())
                        .filter(|agent| !agent.is_empty()),
                );
                decided = true;
                i += 2;
            }
            "--symlink" | "--shared" => {
                shared = true;
                decided = true;
                i += 1;
            }
            "--copy" => {
                shared = false;
                decided = true;
                i += 1;
            }
            "--force" => {
                force = true;
                i += 1;
            }
            other => {
                return Err(format!(
                    "klint-rs: unknown install-skill argument \"{other}\""
                ));
            }
        }
    }

    Ok((
        InstallRequest {
            root,
            agents,
            shared,
            force,
        },
        decided,
    ))
}

fn print_help() {
    println!(
        "klint-rs — shadow Rust architecture engine\n\nUsage: klint-rs [--config <dir>] [--json] [--hook] [--version]\n       klint-rs stats [--config <dir>]\n       klint-rs install-skill [--agents <list>] [--symlink | --copy] [--force]\n\n  --hook           count this run as an agent hook run in klint stats\n  stats            print runs and errors prevented, in total and per rule, from\n                   .klint/stats.json (set KLINT_NO_STATS=1 to stop recording)\n\n  install-skill    install the embedded klint-rules skill into agent config directories\n                   Asks which agents and how to store the skill when run on a\n                   terminal without --agents/--symlink/--copy.\n                   --agents <list>  comma-separated: {} (default: all)\n                   --symlink        one skill in .agents/skills, others symlink to it (default)\n                   --copy           an independent copy in every agent directory\n                   --force          replace a skill directory klint did not install",
        known_agent_names()
    );
}
