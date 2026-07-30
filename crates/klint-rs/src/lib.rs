mod arch;
mod config;
mod files;
mod output;
mod rules;
pub mod syntax;

use std::path::PathBuf;

use arch::run_arch_rules;
use config::{find_config, read_config, resolve_root};
use files::{read_files, resolve_files};
pub use output::{JsonOutput, Summary, Violation};
use rules::run_supported_rules;
use syntax::TreeCache;

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
    let file_contents = read_files(&files)?;
    let _plugins = raw.plugins.unwrap_or_default();
    let rules = raw.rules.unwrap_or_default();

    // Each file is parsed at most once, on demand, the first time some rule
    // or arch scan actually needs its AST — see `TreeCache`.
    let tree_cache = TreeCache::new();

    let mut violations = run_supported_rules(&rules, &files, &file_contents, &tree_cache, &root);
    if let Some(arch) = raw.arch {
        violations.extend(run_arch_rules(
            &arch,
            &files,
            &file_contents,
            &tree_cache,
            &root,
        ));
    }

    Ok(output::output_from_violations(violations))
}

pub fn empty_output() -> JsonOutput {
    output::output_from_violations(Vec::new())
}
