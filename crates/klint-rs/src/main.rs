use klint_rs::{RunOptions, run};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
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
                println!("klint-rs {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            _ => {
                i += 1;
            }
        }
    }

    Ok(CliOptions { config_dir, json })
}

fn print_help() {
    println!(
        "klint-rs — shadow Rust architecture engine\n\nUsage: klint-rs [--config <dir>] [--json] [--version]"
    );
}
