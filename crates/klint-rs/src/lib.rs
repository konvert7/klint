mod arch;
mod config;
mod config_check;
mod config_keys;
mod config_shape;
mod engine;
mod files;
mod fix;
mod migrate;
mod output;
mod plugins;
mod rules;
mod schema;
mod skill;
mod stats;
pub mod syntax;
mod unsupported;
mod version;

use config::{RawConfig, RuleConfig};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use arch::ArchPlan;
use config::{find_config, read_config, resolve_root};
use config_check::{config_advisories, has_valid_severity};
#[doc(hidden)]
pub use config_keys::config_key_paths;
use config_shape::resolve_rules;
use engine::run_engine;
use files::{read_files, resolve_files};
pub use fix::MAX_FIX_PASSES;
use fix::{Originals, apply_fixes, fix_until_stable, has_fixes};
pub use migrate::{MigrateOptions, MigrateOutcome, migrate};
pub use output::{JsonOutput, Summary, Violation};
use rules::plan_rule_passes;
pub use schema::config_schema;
pub use skill::install::{InstallRequest, install_skill};
use skill::skill_advisories;
pub use skill::{known_agent_names, known_agents};
pub use stats::{RunSource, record_run, stats_report};
use unsupported::unsupported_rule_advisories;
pub use version::reported_version;
use version::schema_version_advisory;

#[derive(Debug)]
pub struct RunOptions {
    pub config_dir: PathBuf,
}

#[derive(Debug)]
pub struct FixOutcome {
    pub applied: usize,
    pub remaining: JsonOutput,
    pub stopped_at_pass_limit: bool,
}

pub(crate) struct Linted {
    pub(crate) root: PathBuf,
    pub(crate) findings: Vec<Violation>,
    pub(crate) advisories: Vec<Violation>,
}

pub(crate) struct LoadedConfig {
    pub(crate) path: PathBuf,
    pub(crate) raw: RawConfig,
}

pub fn run(options: RunOptions) -> Result<JsonOutput, String> {
    let Linted {
        mut findings,
        advisories,
        ..
    } = lint(&options)?;
    findings.extend(advisories);
    Ok(output::output_from_violations(findings))
}

pub fn fix(options: RunOptions) -> Result<FixOutcome, String> {
    let mut originals = Originals::default();
    let passes = match fix_until_stable(
        || lint(&options),
        |linted| apply_fixes(&linted.findings, &linted.root, &mut originals),
        |linted| has_fixes(&linted.findings),
    ) {
        Ok(passes) => passes,
        Err(error) => return Err(originals.restore_after(error)),
    };
    Ok(FixOutcome {
        applied: passes.applied,
        remaining: output::output_from_violations(passes.last.findings),
        stopped_at_pass_limit: !passes.converged,
    })
}

fn lint(options: &RunOptions) -> Result<Linted, String> {
    lint_loaded(&options.config_dir, load_config(&options.config_dir)?)
}

pub(crate) fn load_config(config_dir: &Path) -> Result<LoadedConfig, String> {
    let path = find_config(config_dir)?;
    let raw = read_config(&path)?;
    Ok(LoadedConfig { path, raw })
}

pub(crate) fn lint_loaded(config_dir: &Path, loaded: LoadedConfig) -> Result<Linted, String> {
    let LoadedConfig {
        path: config_path,
        raw,
    } = loaded;
    let root = resolve_root(config_dir, raw.root.as_deref());
    let include = raw.include.clone().unwrap_or_else(|| vec![".".to_string()]);

    let files = resolve_files(&root, &include)?;
    let contents = read_files(&files)?;
    let mut resolved = resolve_rules(&raw).map_err(|err| err.into_message(&config_path))?;
    let config_warnings = config_advisories(&resolved, &config_path);
    let rules: BTreeMap<String, RuleConfig> = std::mem::take(&mut resolved.rules)
        .into_iter()
        .filter(|(_, config)| has_valid_severity(config))
        .collect();

    let rule_passes = plan_rule_passes(&rules, &files, &root);
    let arch_plan = raw
        .arch
        .as_ref()
        .map(|arch| ArchPlan::build(arch, &files, &root));

    let findings = run_engine(&rule_passes, arch_plan.as_ref(), &files, &contents, &root);
    let mut advisories: Vec<Violation> =
        schema_version_advisory(raw.schema.as_deref(), &config_path)
            .into_iter()
            .collect();
    advisories.extend(unsupported_rule_advisories(
        &rules,
        &resolved,
        &config_path,
        config_dir,
    ));
    advisories.extend(config_warnings);
    advisories.extend(skill_advisories(config_dir));

    Ok(Linted {
        root,
        findings,
        advisories,
    })
}

pub fn empty_output() -> JsonOutput {
    output::output_from_violations(Vec::new())
}
