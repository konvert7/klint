mod arch;
mod config;
mod engine;
mod files;
mod fix;
mod output;
mod plugins;
mod rules;
mod schema;
mod skill;
mod stats;
pub mod syntax;
mod unsupported;
mod version;

use std::path::PathBuf;

use arch::ArchPlan;
use config::{find_config, read_config, resolve_root};
use engine::run_engine;
use files::{read_files, resolve_files};
pub use fix::MAX_FIX_PASSES;
use fix::{Originals, apply_fixes, fix_until_stable, has_fixes};
pub use output::{JsonOutput, Summary, Violation};
use plugins::effective_rules;
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
    let config_path = find_config(&options.config_dir)?;
    let raw = read_config(&config_path)?;
    let root = resolve_root(&options.config_dir, raw.root.as_deref());
    let include = raw.include.unwrap_or_else(|| vec![".".to_string()]);

    let files = resolve_files(&root, &include)?;
    let contents = read_files(&files)?;
    let rules = effective_rules(
        &raw.plugins.unwrap_or_default(),
        raw.rules.unwrap_or_default(),
    )?;

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
        &config_path,
        &options.config_dir,
    ));
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
