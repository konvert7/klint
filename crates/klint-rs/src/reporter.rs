use std::time::Duration;

use klint_rs::{FixOutcome, JsonOutput};
use serde::Serialize;

use crate::command::{OptionSpec, Values};
use crate::{github, text};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Reporter {
    Text,
    Json,
    Github,
}

const NAMES: [&str; 3] = ["text", "json", "github"];

pub fn options() -> [OptionSpec; 2] {
    [
        OptionSpec::valued(
            "reporter",
            "<name>",
            format!("Output format: {} (default: text)", NAMES.join(", ")),
        ),
        OptionSpec::flag("json", "Same as --reporter json").hidden(),
    ]
}

pub fn from_values(values: &Values) -> Result<Reporter, String> {
    let json_alias = values.flag("json").then_some(Reporter::Json);
    let named = values.text("reporter").map(named).transpose()?;

    match (json_alias, named) {
        (Some(_), Some(other)) if other != Reporter::Json => {
            Err("klint: --json is --reporter json; pass only one reporter".to_string())
        }
        (alias, named) => Ok(named.or(alias).unwrap_or(Reporter::Text)),
    }
}

fn named(name: &str) -> Result<Reporter, String> {
    match name {
        "text" => Ok(Reporter::Text),
        "json" => Ok(Reporter::Json),
        "github" => Ok(Reporter::Github),
        _ => Err(format!(
            "klint: unknown reporter '{name}', expected one of: {}",
            NAMES.join(", ")
        )),
    }
}

pub fn print(reporter: Reporter, output: &JsonOutput, elapsed: Duration, style: text::TextStyle) {
    match reporter {
        Reporter::Json => println!(
            "{}",
            serde_json::to_string(output).expect("klint-rs output should be serializable")
        ),
        Reporter::Github => print!("{}", github::render(&output.violations)),
        Reporter::Text => {
            let report = text::render(&output.violations, elapsed, style);
            print!("{}", report.stdout);
            eprint!("{}", report.stderr);
        }
    }
}

pub fn check_fix_support(reporter: Reporter) -> Result<(), String> {
    match reporter {
        Reporter::Github => Err(
            "klint: --fix does not support --reporter github; use --reporter text or json"
                .to_string(),
        ),
        Reporter::Text | Reporter::Json => Ok(()),
    }
}

#[derive(Serialize)]
struct FixJson<'a> {
    applied: usize,
    #[serde(flatten)]
    remaining: &'a JsonOutput,
}

pub fn print_fix_json(outcome: &FixOutcome) {
    let report = FixJson {
        applied: outcome.applied,
        remaining: &outcome.remaining,
    };
    println!(
        "{}",
        serde_json::to_string(&report).expect("klint-rs fix output should be serializable")
    );
}
