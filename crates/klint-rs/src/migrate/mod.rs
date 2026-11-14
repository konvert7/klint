mod json;
mod json_levels;
mod levels;
mod proof;
mod yaml_levels;
mod yaml_lines;
mod yaml_rules;

use std::fs;
use std::path::PathBuf;

use crate::LoadedConfig;
use crate::config::{ConfigFormat, find_config, parse_config_text};
use crate::config_shape::{ConfigShape, Group, config_shape};
use crate::plugins::SONAR_PREFIX;
use crate::rules::is_experimental_rule;
use levels::LevelPlan;
pub use proof::NewlyRunning;

pub(crate) fn v2_location(v1_name: &str) -> (Group, &str) {
    match v1_name.strip_prefix(SONAR_PREFIX) {
        Some(short) => (Group::Sonar, short),
        None if is_experimental_rule(v1_name) => (Group::Experimental, v1_name),
        None => (Group::Core, v1_name),
    }
}

pub struct MigrateOptions {
    pub config_dir: PathBuf,
    pub check_only: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum MigrateOutcome {
    AlreadyV2 {
        file: String,
    },
    NeedsMigration {
        file: String,
    },
    Migrated {
        file: String,
        newly_running: Vec<NewlyRunning>,
    },
    Refused {
        file: String,
        line: usize,
        reason: String,
    },
}

impl MigrateOutcome {
    pub fn refusal_message(file: &str, reason: &str) -> String {
        format!("klint: cannot migrate {file}: {reason}")
    }
}

fn reason_line(reason: &str) -> usize {
    reason
        .strip_prefix("line ")
        .and_then(|rest| rest.split_once(": "))
        .and_then(|(number, _)| number.parse().ok())
        .unwrap_or(1)
}

fn refused(file: &str, reason: String) -> MigrateOutcome {
    MigrateOutcome::Refused {
        file: file.to_string(),
        line: reason_line(&reason),
        reason,
    }
}

fn mixed_reason(keys: &[String]) -> String {
    format!(
        "This config mixes v1 and v2 keys ({}) — move them into rules.core / rules.sonar by hand.",
        keys.join(", ")
    )
}

fn rewritten(
    format: ConfigFormat,
    text: &str,
    needs_groups: bool,
    plans: &[LevelPlan],
) -> Result<String, String> {
    let grouped = match (needs_groups, format) {
        (false, _) => text.to_string(),
        (true, ConfigFormat::Yaml) => yaml_rules::migrate_yaml(text)?,
        (true, ConfigFormat::Json) => json::migrate_json(text)?,
    };

    if plans.is_empty() {
        return Ok(grouped);
    }

    match format {
        ConfigFormat::Yaml => yaml_levels::migrate_levels_yaml(&grouped, plans),
        ConfigFormat::Json => json_levels::migrate_levels_json(&grouped, plans),
    }
}

fn file_name(path: &std::path::Path) -> String {
    path.file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
}

pub fn migrate(options: &MigrateOptions) -> Result<MigrateOutcome, String> {
    let dir = &options.config_dir;
    let path = find_config(dir)?;
    let file = file_name(&path);
    let text = fs::read_to_string(&path)
        .map_err(|err| format!("klint: failed to read {}: {err}", path.display()))?;
    let format = ConfigFormat::of(&path);
    let raw = match parse_config_text(&text, format) {
        Ok(raw) => raw,
        Err(_) => {
            return match format {
                ConfigFormat::Yaml => yaml_rules::refusal(&text)
                    .map(|reason| refused(&file, reason))
                    .ok_or_else(|| format!("klint: failed to parse {}", path.display())),
                ConfigFormat::Json => Err(format!("klint: failed to parse {}", path.display())),
            };
        }
    };
    let shape = config_shape(&raw);
    let needs_groups = shape != ConfigShape::V2;
    let needs_levels = levels::needs_levels(&raw);
    if !needs_groups && !needs_levels {
        return Ok(MigrateOutcome::AlreadyV2 { file });
    }

    if options.check_only {
        return Ok(MigrateOutcome::NeedsMigration { file });
    }

    if let ConfigShape::Mixed { keys } = &shape {
        return Ok(refused(&file, mixed_reason(keys)));
    }

    let plans = match levels::level_plans(&raw, &text) {
        Ok(plans) => plans,
        Err(reason) => return Ok(refused(&file, reason)),
    };
    let new_text = match rewritten(format, &text, needs_groups, &plans) {
        Ok(new_text) => new_text,
        Err(reason) => return Ok(refused(&file, reason)),
    };
    let old = LoadedConfig {
        path: path.clone(),
        raw,
    };
    let newly_running = match proof::prove_same_findings(dir, old, &new_text) {
        Ok(newly_running) => newly_running,
        Err(reason) => return Ok(refused(&file, reason)),
    };

    match fs::write(&path, new_text) {
        Ok(()) => Ok(MigrateOutcome::Migrated {
            file,
            newly_running,
        }),
        Err(err) => Ok(refused(
            &file,
            format!("failed to write {}: {err}", path.display()),
        )),
    }
}
