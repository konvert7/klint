mod arch;
mod config;
mod engine;
mod files;
mod fix;
mod output;
mod rules;
mod skill;
mod stats;
pub mod syntax;
mod version;

use std::path::PathBuf;

use arch::ArchPlan;
use config::{find_config, read_config, resolve_root};
use engine::run_engine;
use files::{read_files, resolve_files};
use fix::{apply_fixes, has_fixes};
pub use output::{JsonOutput, Summary, Violation};
use rules::plan_rule_passes;
pub use skill::install::{InstallRequest, install_skill};
use skill::skill_advisories;
pub use skill::{known_agent_names, known_agents};
pub use stats::{RunSource, record_run, stats_report};
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
}

struct Linted {
    root: PathBuf,
    findings: Vec<Violation>,
    advisories: Vec<Violation>,
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
    let mut linted = lint(&options)?;
    let mut applied = 0;
    loop {
        let applied_this_pass = apply_fixes(&linted.findings, &linted.root)?;
        applied += applied_this_pass;
        if applied_this_pass == 0 {
            break;
        }
        linted = lint(&options)?;
        if !has_fixes(&linted.findings) {
            break;
        }
    }
    Ok(FixOutcome {
        applied,
        remaining: output::output_from_violations(linted.findings),
    })
}

fn lint(options: &RunOptions) -> Result<Linted, String> {
    let config_path = find_config(&options.config_dir)?;
    let raw = read_config(&config_path)?;
    let root = resolve_root(&options.config_dir, raw.root.as_deref());
    let include = raw.include.unwrap_or_else(|| vec![".".to_string()]);

    let files = resolve_files(&root, &include)?;
    let contents = read_files(&files)?;
    let _plugins = raw.plugins.unwrap_or_default();
    let rules = raw.rules.unwrap_or_default();

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
    advisories.extend(skill_advisories(&options.config_dir));

    Ok(Linted {
        root,
        findings,
        advisories,
    })
}

pub fn empty_output() -> JsonOutput {
    output::output_from_violations(Vec::new())
}
