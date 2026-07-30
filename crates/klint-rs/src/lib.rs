mod arch;
mod config;
mod engine;
mod files;
mod output;
mod rules;
pub mod syntax;
mod version;

use std::path::PathBuf;

use arch::ArchPlan;
use config::{find_config, read_config, resolve_root};
use engine::run_engine;
use files::{read_files, resolve_files};
pub use output::{JsonOutput, Summary, Violation};
use rules::plan_rule_passes;
pub use version::reported_version;
use version::schema_version_advisory;

#[derive(Debug)]
pub struct RunOptions {
    pub config_dir: PathBuf,
}

pub fn run(options: RunOptions) -> Result<JsonOutput, String> {
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

    let mut violations = run_engine(&rule_passes, arch_plan.as_ref(), &files, &contents, &root);
    violations.extend(schema_version_advisory(raw.schema.as_deref(), &config_path));

    Ok(output::output_from_violations(violations))
}

pub fn empty_output() -> JsonOutput {
    output::output_from_violations(Vec::new())
}
