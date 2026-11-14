use std::path::Path;

use crate::config::read_config;
use crate::rules::is_experimental_rule;
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

#[derive(Debug, PartialEq, Eq)]
pub struct NewlyRunning {
    pub rule: String,
    pub findings: usize,
}

fn newly_running_experimental(before: &[Violation], after: &[Violation]) -> Vec<NewlyRunning> {
    let mut newly: Vec<NewlyRunning> = Vec::new();

    for finding in after.iter().filter(|f| is_experimental_rule(&f.rule)) {
        if before.iter().any(|b| b.rule == finding.rule) {
            continue;
        }

        match newly.iter_mut().find(|n| n.rule == finding.rule) {
            Some(entry) => entry.findings += 1,
            None => newly.push(NewlyRunning {
                rule: finding.rule.clone(),
                findings: 1,
            }),
        }
    }

    newly
}

fn without_rules(findings: Vec<Violation>, newly: &[NewlyRunning]) -> Vec<Violation> {
    findings
        .into_iter()
        .filter(|f| !newly.iter().any(|n| n.rule == f.rule))
        .collect()
}

pub(super) fn prove_same_findings(
    config_dir: &Path,
    old: LoadedConfig,
    new_text: &str,
) -> Result<Vec<NewlyRunning>, String> {
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
    let before = lint_loaded(config_dir, old)?.findings;
    let after = lint_loaded(config_dir, new)?.findings;
    let newly = newly_running_experimental(&before, &after);
    let after = without_rules(after, &newly);
    match first_difference(&serialized(&before), &serialized(&after)) {
        Some(difference) => Err(difference),
        None => Ok(newly),
    }
}

#[cfg(test)]
mod tests {
    use super::first_difference;

    fn items(values: &[&str]) -> Vec<String> {
        values.iter().map(|v| v.to_string()).collect()
    }

    #[test]
    fn identical_findings_have_no_difference() {
        assert_eq!(
            first_difference(&items(&["a", "b"]), &items(&["a", "b"])),
            None
        );
    }

    #[test]
    fn changed_finding_is_reported() {
        let message = first_difference(&items(&["a", "b"]), &items(&["a", "c"]))
            .expect("a difference is reported");
        assert_eq!(message, "findings would change — first difference: b → c");
    }

    #[test]
    fn dropped_finding_is_reported_as_none() {
        let message = first_difference(&items(&["a", "b"]), &items(&["a"]))
            .expect("a difference is reported");
        assert_eq!(
            message,
            "findings would change — first difference: b → (none)"
        );
    }
}
