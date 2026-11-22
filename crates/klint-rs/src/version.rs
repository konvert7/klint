use crate::output::Violation;
use std::fs;
use std::path::Path;

const UNRELEASED: &str = "0.0.0";
const TAG_MARKER: &str = "/refs/tags/v";
const RULE: &str = "klint/schema-version";

pub fn reported_version() -> String {
    version_from(
        compiled_version(),
        sidecar_version,
        env!("CARGO_PKG_VERSION"),
    )
}

fn version_from(
    compiled: Option<&str>,
    sidecar: impl FnOnce() -> Option<String>,
    fallback: &str,
) -> String {
    compiled
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(sidecar)
        .unwrap_or_else(|| fallback.to_string())
}

pub(crate) fn released_version() -> Option<String> {
    let version = reported_version();
    (version != UNRELEASED).then_some(version)
}

pub(crate) fn schema_url_for_version(version: &str) -> String {
    format!(
        "https://raw.githubusercontent.com/konvert7/klint/refs/tags/v{version}/klint.schema.json"
    )
}

pub(crate) fn declared_schema_version(schema: &str) -> Option<&str> {
    let start = schema.find(TAG_MARKER)? + TAG_MARKER.len();
    let rest = &schema[start..];
    let version = &rest[..rest.find('/')?];
    is_semver(version).then_some(version)
}

pub(crate) fn schema_version_advisory(
    schema: Option<&str>,
    config_path: &Path,
) -> Option<Violation> {
    advisory_for(schema?, &released_version()?, config_path)
}

fn advisory_for(schema: &str, installed: &str, config_path: &Path) -> Option<Violation> {
    let declared = declared_schema_version(schema)?;
    if declared == installed {
        return None;
    }

    Some(Violation {
        file: config_file_name(config_path),
        line: schema_declaration_line(config_path),
        rule: RULE.to_string(),
        message: format!(
            "config declares schema v{declared} but klint {installed} is installed — update $schema to {}",
            schema_url_for_version(installed)
        ),
        severity: "warn".to_string(),
        fix: None,
        span: None,
    })
}

fn compiled_version() -> Option<&'static str> {
    option_env!("KLINT_VERSION")
}

fn sidecar_version() -> Option<String> {
    let executable = std::env::current_exe().ok()?;
    let text = fs::read_to_string(executable.parent()?.join("VERSION")).ok()?;
    let trimmed = text.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn is_semver(value: &str) -> bool {
    let core = value.split('-').next().unwrap_or_default();
    let parts: Vec<&str> = core.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

fn config_file_name(config_path: &Path) -> String {
    config_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("klint.yaml")
        .to_string()
}

fn schema_declaration_line(config_path: &Path) -> usize {
    let Ok(text) = fs::read_to_string(config_path) else {
        return 1;
    };
    text.lines()
        .position(|line| line.contains("$schema"))
        .map_or(1, |index| index + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declared_schema_version_reads_the_tag_segment() {
        assert_eq!(
            declared_schema_version(
                "https://raw.githubusercontent.com/konvert7/klint/refs/tags/v0.29.0/klint.schema.json"
            ),
            Some("0.29.0")
        );
    }

    #[test]
    fn declared_schema_version_accepts_prerelease_tags() {
        assert_eq!(
            declared_schema_version(
                "https://raw.githubusercontent.com/konvert7/klint/refs/tags/v1.0.0-rc.1/klint.schema.json"
            ),
            Some("1.0.0-rc.1")
        );
    }

    #[test]
    fn declared_schema_version_ignores_unversioned_and_local_paths() {
        assert_eq!(declared_schema_version("./klint.schema.json"), None);
        assert_eq!(
            declared_schema_version("https://klint.dev/schema.json"),
            None
        );
        assert_eq!(
            declared_schema_version(
                "https://raw.githubusercontent.com/konvert7/klint/refs/tags/vmain/klint.schema.json"
            ),
            None
        );
    }

    #[test]
    fn declared_schema_version_needs_three_numeric_parts() {
        for version in ["1.2", "1..3", "1.x.3"] {
            assert_eq!(
                declared_schema_version(&schema_url_for_version(version)),
                None,
                "{version}"
            );
        }
    }

    #[test]
    fn the_compiled_version_wins_unless_it_is_empty() {
        let sidecar = || Some("2.0.0".to_string());

        assert_eq!(version_from(Some("1.0.0"), sidecar, "0.0.0"), "1.0.0");
        assert_eq!(version_from(Some(""), sidecar, "0.0.0"), "2.0.0");
        assert_eq!(version_from(None, || None, "0.0.0"), "0.0.0");
    }

    #[test]
    fn schema_url_for_version_round_trips() {
        let url = schema_url_for_version("2.5.1");
        assert_eq!(declared_schema_version(&url), Some("2.5.1"));
    }

    #[test]
    fn advisory_is_absent_without_a_versioned_schema() {
        assert!(schema_version_advisory(None, Path::new("klint.yaml")).is_none());
        assert!(
            schema_version_advisory(Some("./klint.schema.json"), Path::new("klint.yaml")).is_none()
        );
    }

    #[test]
    fn advisory_warns_when_the_declared_version_is_stale() {
        let advisory = advisory_for(
            &schema_url_for_version("0.11.2"),
            "0.29.0",
            Path::new("klint.yaml"),
        )
        .expect("stale schema version should produce an advisory");

        assert_eq!(advisory.severity, "warn");
        assert_eq!(advisory.rule, RULE);
        assert_eq!(advisory.file, "klint.yaml");
        assert!(advisory.message.contains("declares schema v0.11.2"));
        assert!(advisory.message.contains("klint 0.29.0 is installed"));
        assert!(advisory.message.contains(&schema_url_for_version("0.29.0")));
    }

    #[test]
    fn advisory_is_absent_when_the_versions_match() {
        assert!(
            advisory_for(
                &schema_url_for_version("0.29.0"),
                "0.29.0",
                Path::new("klint.yaml")
            )
            .is_none()
        );
    }

    #[test]
    fn advisory_falls_back_to_line_one_for_an_unreadable_config() {
        let advisory = advisory_for(
            &schema_url_for_version("0.11.2"),
            "0.29.0",
            Path::new("no-such-klint.yaml"),
        )
        .expect("stale schema version should produce an advisory");

        assert_eq!(advisory.line, 1);
    }
}
