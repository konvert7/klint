use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::output::Violation;

const STATS_DIR: &str = ".klint";
const STATS_FILE: &str = "stats.json";
const SELF_IGNORE: &str = "# Created by klint automatically.\n*\n";
const STATS_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunSource {
    Hook,
    Direct,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct SourceCounts {
    hook: u64,
    direct: u64,
}

impl SourceCounts {
    fn bump(&mut self, source: RunSource) {
        match source {
            RunSource::Hook => self.hook += 1,
            RunSource::Direct => self.direct += 1,
        }
    }

    fn total(&self) -> u64 {
        self.hook + self.direct
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Prevented {
    by_rule: BTreeMap<String, u64>,
    by_source: SourceCounts,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenError {
    rule: String,
    file: String,
    message: String,
    caught_by: RunSource,
}

impl OpenError {
    fn same_error(&self, violation: &Violation) -> bool {
        self.rule == violation.rule
            && self.file == violation.file
            && self.message == violation.message
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct KlintStats {
    version: u32,
    runs: SourceCounts,
    prevented: Prevented,
    open: Vec<OpenError>,
}

impl KlintStats {
    fn empty() -> Self {
        Self {
            version: STATS_VERSION,
            runs: SourceCounts::default(),
            prevented: Prevented::default(),
            open: Vec::new(),
        }
    }
}

fn next_stats(previous: &KlintStats, violations: &[Violation], source: RunSource) -> KlintStats {
    let mut stats = previous.clone();
    stats.runs.bump(source);

    let mut unmatched: Vec<&Violation> = violations
        .iter()
        .filter(|violation| violation.severity == "error")
        .collect();
    let mut still_open = Vec::new();
    for error in &previous.open {
        match unmatched
            .iter()
            .position(|current| error.same_error(current))
        {
            Some(index) => {
                unmatched.remove(index);
                still_open.push(error.clone());
            }
            None => {
                *stats
                    .prevented
                    .by_rule
                    .entry(error.rule.clone())
                    .or_insert(0) += 1;
                stats.prevented.by_source.bump(error.caught_by);
            }
        }
    }
    still_open.extend(unmatched.into_iter().map(|violation| OpenError {
        rule: violation.rule.clone(),
        file: violation.file.clone(),
        message: violation.message.clone(),
        caught_by: source,
    }));
    stats.open = still_open;
    stats
}

fn read_stats(config_dir: &Path) -> KlintStats {
    fs::read_to_string(config_dir.join(STATS_DIR).join(STATS_FILE))
        .ok()
        .and_then(|text| serde_json::from_str::<KlintStats>(&text).ok())
        .filter(|stats| stats.version == STATS_VERSION)
        .unwrap_or_else(KlintStats::empty)
}

fn write_stats(config_dir: &Path, stats: &KlintStats) -> std::io::Result<()> {
    let dir = config_dir.join(STATS_DIR);
    fs::create_dir_all(&dir)?;
    let ignore_file = dir.join(".gitignore");
    if !ignore_file.exists() {
        fs::write(&ignore_file, SELF_IGNORE)?;
    }
    let target = dir.join(STATS_FILE);
    let staging = dir.join(format!("{STATS_FILE}.{}.tmp", std::process::id()));
    let json = serde_json::to_string_pretty(stats).map_err(std::io::Error::other)?;
    fs::write(&staging, format!("{json}\n"))?;
    fs::rename(&staging, &target)
}

fn stats_disabled() -> bool {
    std::env::var_os("KLINT_NO_STATS").is_some_and(|value| !value.is_empty())
}

pub fn record_run(config_dir: &Path, violations: &[Violation], source: RunSource) {
    if stats_disabled() {
        return;
    }
    let _best_effort = write_stats(
        config_dir,
        &next_stats(&read_stats(config_dir), violations, source),
    );
}

fn rules_by_count(by_rule: &BTreeMap<String, u64>) -> Vec<(&String, u64)> {
    let mut rules: Vec<(&String, u64)> =
        by_rule.iter().map(|(rule, count)| (rule, *count)).collect();
    rules.sort_by(|(a, x), (b, y)| y.cmp(x).then_with(|| a.cmp(b)));
    rules
}

fn format_stats(stats: &KlintStats) -> String {
    let rules = rules_by_count(&stats.prevented.by_rule);
    let width = rules
        .iter()
        .map(|(rule, _)| rule.chars().count())
        .max()
        .unwrap_or(0);
    let mut lines = vec![
        format!("klint stats ({STATS_DIR}/{STATS_FILE})"),
        String::new(),
        format!(
            "  runs              {}  (hook {}, direct {})",
            stats.runs.total(),
            stats.runs.hook,
            stats.runs.direct
        ),
        format!(
            "  errors prevented  {}  (caught by hook {}, direct {})",
            stats.prevented.by_source.total(),
            stats.prevented.by_source.hook,
            stats.prevented.by_source.direct
        ),
    ];
    if !rules.is_empty() {
        lines.push(String::new());
        lines.push("  prevented by rule".to_string());
        for (rule, count) in rules {
            lines.push(format!("    {rule:<width$}  {count}"));
        }
    }
    format!("{}\n", lines.join("\n"))
}

pub fn stats_report(config_dir: &Path) -> String {
    format_stats(&read_stats(config_dir))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::tests::temp_root;

    fn error(rule: &str, file: &str, message: &str) -> Violation {
        Violation {
            file: file.to_string(),
            line: 1,
            rule: rule.to_string(),
            message: message.to_string(),
            severity: "error".to_string(),
            fix: None,
        }
    }

    #[test]
    fn an_error_gone_on_the_next_run_counts_as_prevented_by_its_catcher() {
        let flagged = next_stats(
            &KlintStats::empty(),
            &[error("arch/imports", "a.ts", "m")],
            RunSource::Hook,
        );
        let fixed = next_stats(&flagged, &[], RunSource::Direct);

        assert_eq!(fixed.runs, SourceCounts { hook: 1, direct: 1 });
        assert_eq!(fixed.prevented.by_rule.get("arch/imports"), Some(&1));
        assert_eq!(
            fixed.prevented.by_source,
            SourceCounts { hook: 1, direct: 0 }
        );
        assert!(fixed.open.is_empty());
    }

    #[test]
    fn an_error_still_present_is_not_counted_again() {
        let violations = [error("r", "a.ts", "m")];
        let first = next_stats(&KlintStats::empty(), &violations, RunSource::Hook);
        let second = next_stats(&first, &violations, RunSource::Hook);

        assert_eq!(second.prevented.by_source.total(), 0);
        assert_eq!(second.open.len(), 1);
    }

    #[test]
    fn duplicate_errors_are_matched_one_for_one() {
        let twice = [error("r", "a.ts", "m"), error("r", "a.ts", "m")];
        let first = next_stats(&KlintStats::empty(), &twice, RunSource::Direct);
        let second = next_stats(&first, &twice[..1], RunSource::Direct);

        assert_eq!(second.prevented.by_rule.get("r"), Some(&1));
        assert_eq!(second.open.len(), 1);
    }

    #[test]
    fn warnings_are_not_tracked() {
        let mut warning = error("r", "a.ts", "m");
        warning.severity = "warn".to_string();
        let stats = next_stats(&KlintStats::empty(), &[warning], RunSource::Direct);

        assert!(stats.open.is_empty());
    }

    #[test]
    fn report_lists_rules_by_count_then_name() {
        let mut stats = KlintStats::empty();
        stats.runs = SourceCounts { hook: 3, direct: 1 };
        stats.prevented.by_rule.insert("b-rule".to_string(), 1);
        stats.prevented.by_rule.insert("a-rule".to_string(), 1);
        stats
            .prevented
            .by_rule
            .insert("arch/imports".to_string(), 2);
        stats.prevented.by_source = SourceCounts { hook: 3, direct: 1 };

        assert_eq!(
            format_stats(&stats),
            "klint stats (.klint/stats.json)\n\n  runs              4  (hook 3, direct 1)\n  errors prevented  4  (caught by hook 3, direct 1)\n\n  prevented by rule\n    arch/imports  2\n    a-rule        1\n    b-rule        1\n"
        );
    }

    #[test]
    fn recording_creates_a_self_ignoring_stats_dir() {
        let root = temp_root("stats");
        fs::create_dir_all(&root).expect("create root");
        record_run(&root, &[error("r", "a.ts", "m")], RunSource::Hook);

        let ignore = fs::read_to_string(root.join(".klint/.gitignore")).expect("read .gitignore");
        assert!(ignore.lines().any(|line| line == "*"));
        assert_eq!(read_stats(&root).runs, SourceCounts { hook: 1, direct: 0 });
        fs::remove_dir_all(&root).expect("clean up");
    }

    #[test]
    fn a_corrupt_stats_file_starts_over() {
        let root = temp_root("stats-corrupt");
        fs::create_dir_all(root.join(STATS_DIR)).expect("create stats dir");
        fs::write(root.join(STATS_DIR).join(STATS_FILE), "{not json").expect("write corrupt file");

        assert_eq!(read_stats(&root), KlintStats::empty());
        fs::remove_dir_all(&root).expect("clean up");
    }
}
