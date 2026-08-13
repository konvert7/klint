use klint_rs::{InstallRequest, RunOptions, install_skill, known_agent_names, run};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.first().is_some_and(|arg| arg == "install-skill") {
        std::process::exit(install_skill_command(&args[1..]));
    }

    let options = parse_args(&args).unwrap_or_else(|err| {
        eprintln!("{err}");
        std::process::exit(1);
    });

    match run(RunOptions {
        config_dir: options.config_dir,
    }) {
        Ok(output) => {
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
}

fn parse_args(args: &[String]) -> Result<CliOptions, String> {
    let mut config_dir =
        std::env::current_dir().map_err(|err| format!("klint-rs: failed to resolve cwd: {err}"))?;
    let mut json = false;
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

    Ok(CliOptions { config_dir, json })
}

fn install_skill_command(args: &[String]) -> i32 {
    let request = match parse_install_args(args) {
        Ok(request) => request,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };

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

fn parse_install_args(args: &[String]) -> Result<InstallRequest, String> {
    let root =
        std::env::current_dir().map_err(|err| format!("klint-rs: failed to resolve cwd: {err}"))?;
    let mut agents: Vec<String> = Vec::new();
    let mut shared = true;
    let mut force = false;
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
                i += 2;
            }
            "--symlink" | "--shared" => {
                shared = true;
                i += 1;
            }
            "--copy" => {
                shared = false;
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

    Ok(InstallRequest {
        root,
        agents,
        shared,
        force,
    })
}

fn print_help() {
    println!(
        "klint-rs — shadow Rust architecture engine\n\nUsage: klint-rs [--config <dir>] [--json] [--version]\n       klint-rs install-skill [--agents <list>] [--symlink | --copy] [--force]\n\n  install-skill    install the embedded klint-rules skill into agent config directories\n                   --agents <list>  comma-separated: {} (default: all)\n                   --symlink        one skill in .agents/skills, others symlink to it (default)\n                   --copy           an independent copy in every agent directory\n                   --force          replace a skill directory klint did not install",
        known_agent_names()
    );
}
