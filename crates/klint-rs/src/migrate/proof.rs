use std::path::Path;

use crate::config::read_config;
use crate::{LoadedConfig, Violation, lint_loaded};

fn read_candidate(old_path: &Path, new_text: &str) -> Result<crate::config::RawConfig, String> {
    let scratch = std::env::temp_dir().join(format!("klint-migrate-{}", std::process::id()));
    let candidate = scratch.join(old_path.file_name().unwrap_or_default());
    let written =
        std::fs::create_dir_all(&scratch).and_then(|()| std::fs::write(&candidate, new_text));
    let read = written
        .map_err(|_| "the migrated config could not be written for checking".to_string())
        .and_then(|()| {
            read_config(&candidate).map_err(|_| "the migrated config does not parse".to_string())
        });
    let _ = std::fs::remove_dir_all(&scratch);
    read
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

#[cfg(debug_assertions)]
fn forced_mismatch() -> bool {
    std::env::var("KLINT_MIGRATE_FORCE_MISMATCH").as_deref() == Ok("1")
}

#[cfg(not(debug_assertions))]
fn forced_mismatch() -> bool {
    false
}

pub(super) fn prove_same_findings(
    config_dir: &Path,
    old: LoadedConfig,
    new_text: &str,
) -> Result<(), String> {
    if forced_mismatch() {
        return Err(
            "findings would change — first difference: forced by KLINT_MIGRATE_FORCE_MISMATCH"
                .into(),
        );
    }
    let new = LoadedConfig {
        path: old.path.clone(),
        raw: read_candidate(&old.path, new_text)?,
    };
    let before = serialized(&lint_loaded(config_dir, old)?.findings);
    let after = serialized(&lint_loaded(config_dir, new)?.findings);
    first_difference(&before, &after).map_or(Ok(()), Err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ConfigFormat, parse_config_text};

    const OLD: &str = "include: [\"src\"]\nrules:\n  no-string-match: error\n";
    const DROPPED_RULE: &str = "include: [\"src\"]\nrules:\n  core:\n    no-date-equality: off\n";

    fn project() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("klint-proof-test-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("src")).expect("mkdir");
        std::fs::write(dir.join("src/a.ts"), "const m = \"x\".match(/x/);\n").expect("write");
        dir
    }

    #[test]
    fn refuses_when_the_new_config_changes_the_findings() {
        let dir = project();
        let old = LoadedConfig {
            path: dir.join("klint.yaml"),
            raw: parse_config_text(OLD, ConfigFormat::Yaml).expect("parse"),
        };
        let result = prove_same_findings(&dir, old, DROPPED_RULE);
        let _ = std::fs::remove_dir_all(&dir);
        let reason = result.expect_err("findings differ");
        assert!(reason.contains("findings would change"), "{reason}");
    }
}
