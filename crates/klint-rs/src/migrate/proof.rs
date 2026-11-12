use std::path::Path;

use crate::config::{ConfigFormat, parse_config_text};
use crate::{LoadedConfig, Violation, lint_loaded};

fn forced_mismatch() -> bool {
    std::env::var("KLINT_MIGRATE_FORCE_MISMATCH").as_deref() == Ok("1")
}

fn first_difference(before: &[String], after: &[String]) -> Option<String> {
    let shown =
        |items: &[String], i: usize| items.get(i).map_or("(none)", String::as_str).to_string();
    (0..before.len().max(after.len()))
        .find(|&i| before.get(i) != after.get(i))
        .map(|i| {
            format!(
                "findings would change — first difference: {} → {}",
                shown(before, i),
                shown(after, i)
            )
        })
}

fn serialized(findings: &[Violation]) -> Vec<String> {
    findings
        .iter()
        .map(|finding| serde_json::to_string(finding).expect("violation serializes"))
        .collect()
}

pub(super) fn prove_same_findings(
    config_dir: &Path,
    old: LoadedConfig,
    new_text: &str,
    format: ConfigFormat,
) -> Result<(), String> {
    let new = LoadedConfig {
        path: old.path.clone(),
        raw: parse_config_text(new_text, format)
            .map_err(|_| "the migrated config does not parse".to_string())?,
    };
    let before = serialized(&lint_loaded(config_dir, old)?.findings);
    let after = serialized(&lint_loaded(config_dir, new)?.findings);
    if forced_mismatch() {
        return Err(
            "findings would change — first difference: forced by KLINT_MIGRATE_FORCE_MISMATCH"
                .to_string(),
        );
    }
    first_difference(&before, &after).map_or(Ok(()), Err)
}
