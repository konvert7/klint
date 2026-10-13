use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::output::Violation;

const STATS_DIR: &str = ".klint";
const STATS_FILE: &str = "stats.json";
const SELF_IGNORE: &str = "# Created by klint automatically.\n*\n";
const STATS_VERSION: u32 = 1;
const KLINT_ADVISORY_PREFIX: &str = "klint/";
const LABEL_WIDTH: usize = "violations reported".len();
const RULE_HEADING: &str = "by rule";
const REPORTED_HEADING: &str = "reported";
const PREVENTED_HEADING: &str = "prevented";

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
struct RuleCounts {
    by_rule: BTreeMap<String, u64>,
    by_source: SourceCounts,
}

impl RuleCounts {
    fn bump(&mut self, rule: &str, source: RunSource) {
        *self.by_rule.entry(rule.to_string()).or_insert(0) += 1;
        self.by_source.bump(source);
    }
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
    #[serde(default)]
    reported: RuleCounts,
    prevented: RuleCounts,
    open: Vec<OpenError>,
}

impl KlintStats {
    fn empty() -> Self {
        Self {
            version: STATS_VERSION,
            runs: SourceCounts::default(),
            reported: RuleCounts::default(),
            prevented: RuleCounts::default(),
            open: Vec::new(),
        }
    }
}

fn is_code_violation(violation: &Violation) -> bool {
    !violation.rule.starts_with(KLINT_ADVISORY_PREFIX)
}

fn next_stats(previous: &KlintStats, violations: &[Violation], source: RunSource) -> KlintStats {
    let mut stats = previous.clone();
    stats.runs.bump(source);
    for violation in violations
        .iter()
        .filter(|violation| is_code_violation(violation))
    {
        stats.reported.bump(&violation.rule, source);
    }

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
            None => stats.prevented.bump(&error.rule, error.caught_by),
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

struct RuleRow<'a> {
    rule: &'a str,
    reported: u64,
    prevented: u64,
}

fn rule_rows(stats: &KlintStats) -> Vec<RuleRow<'_>> {
    let rules: BTreeSet<&String> = stats
        .reported
        .by_rule
        .keys()
        .chain(stats.prevented.by_rule.keys())
        .collect();
    let mut rows: Vec<RuleRow> = rules
        .into_iter()
        .map(|rule| RuleRow {
            rule,
            reported: stats.reported.by_rule.get(rule).copied().unwrap_or(0),
            prevented: stats.prevented.by_rule.get(rule).copied().unwrap_or(0),
        })
        .collect();
    rows.sort_by(|a, b| {
        b.reported
            .cmp(&a.reported)
            .then_with(|| b.prevented.cmp(&a.prevented))
            .then_with(|| a.rule.cmp(b.rule))
    });
    rows
}

fn summary_lines(summaries: &[(&str, u64, String)]) -> Vec<String> {
    let width = summaries
        .iter()
        .map(|(_, total, _)| total.to_string().len())
        .max()
        .unwrap_or(0);
    summaries
        .iter()
        .map(|(label, total, detail)| {
            format!("  {label:<LABEL_WIDTH$}  {total:>width$}  ({detail})")
        })
        .collect()
}

fn rule_table(rows: &[RuleRow]) -> Vec<String> {
    let width = rows
        .iter()
        .map(|row| row.rule.chars().count())
        .chain([RULE_HEADING.len() - 2])
        .max()
        .unwrap_or(0);
    let heading_width = width + 2;
    let reported_width = REPORTED_HEADING.len();
    let prevented_width = PREVENTED_HEADING.len();
    let mut lines = vec![format!(
        "  {RULE_HEADING:<heading_width$}  {REPORTED_HEADING}  {PREVENTED_HEADING}"
    )];
    lines.extend(rows.iter().map(|row| {
        format!(
            "    {:<width$}  {:>reported_width$}  {:>prevented_width$}",
            row.rule, row.reported, row.prevented
        )
    }));
    lines
}

fn format_stats(stats: &KlintStats) -> String {
    let runs = &stats.runs;
    let reported = &stats.reported.by_source;
    let prevented = &stats.prevented.by_source;
    let mut lines = vec![
        format!("klint stats ({STATS_DIR}/{STATS_FILE})"),
        String::new(),
    ];
    lines.extend(summary_lines(&[
        (
            "runs",
            runs.total(),
            format!("hook {}, direct {}", runs.hook, runs.direct),
        ),
        (
            "violations reported",
            reported.total(),
            format!("hook {}, direct {}", reported.hook, reported.direct),
        ),
        (
            "errors prevented",
            prevented.total(),
            format!(
                "caught by hook {}, direct {}",
                prevented.hook, prevented.direct
            ),
        ),
    ]));
    let rows = rule_rows(stats);
    if !rows.is_empty() {
        lines.push(String::new());
        lines.extend(rule_table(&rows));
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
    fn every_reported_violation_counts_warnings_and_repeats_included() {
        let mut warning = error("w", "a.ts", "m");
        warning.severity = "warn".to_string();
        let first = next_stats(
            &KlintStats::empty(),
            &[error("r", "a.ts", "m"), warning],
            RunSource::Hook,
        );
        let second = next_stats(&first, &[error("r", "a.ts", "m")], RunSource::Direct);

        assert_eq!(second.reported.by_rule.get("r"), Some(&2));
        assert_eq!(second.reported.by_rule.get("w"), Some(&1));
        assert_eq!(
            second.reported.by_source,
            SourceCounts { hook: 2, direct: 1 }
        );
    }

    #[test]
    fn klint_advisories_are_not_reported_violations() {
        let mut advisory = error("klint/skill-stale", "a.ts", "m");
        advisory.severity = "warn".to_string();
        let stats = next_stats(&KlintStats::empty(), &[advisory], RunSource::Hook);

        assert_eq!(stats.reported.by_source.total(), 0);
    }

    #[test]
    fn report_lists_rules_by_reported_then_prevented_then_name() {
        let mut stats = KlintStats::empty();
        stats.runs = SourceCounts { hook: 3, direct: 1 };
        for (rule, count) in [("arch/imports", 5), ("c-rule", 3), ("a-rule", 3)] {
            stats.reported.by_rule.insert(rule.to_string(), count);
        }
        stats.reported.by_source = SourceCounts { hook: 9, direct: 2 };
        for (rule, count) in [("b-rule", 1), ("a-rule", 1), ("arch/imports", 2)] {
            stats.prevented.by_rule.insert(rule.to_string(), count);
        }
        stats.prevented.by_source = SourceCounts { hook: 3, direct: 1 };

        assert_eq!(
            format_stats(&stats),
            concat!(
                "klint stats (.klint/stats.json)\n\n",
                "  runs                  4  (hook 3, direct 1)\n",
                "  violations reported  11  (hook 9, direct 2)\n",
                "  errors prevented      4  (caught by hook 3, direct 1)\n\n",
                "  by rule         reported  prevented\n",
                "    arch/imports         5          2\n",
                "    a-rule               3          1\n",
                "    c-rule               3          0\n",
                "    b-rule               0          1\n",
            )
        );
    }

    #[test]
    fn a_stats_file_from_before_reported_counts_keeps_its_history() {
        let root = temp_root("stats-legacy");
        fs::create_dir_all(root.join(STATS_DIR)).expect("create stats dir");
        let legacy = r#"{"version":1,"runs":{"hook":4,"direct":2},"prevented":{"byRule":{},"bySource":{"hook":0,"direct":0}},"open":[]}"#;
        fs::write(root.join(STATS_DIR).join(STATS_FILE), legacy).expect("write legacy file");

        let stats = read_stats(&root);
        assert_eq!(stats.runs, SourceCounts { hook: 4, direct: 2 });
        assert_eq!(stats.reported, RuleCounts::default());
        fs::remove_dir_all(&root).expect("clean up");
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
