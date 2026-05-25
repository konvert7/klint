use crate::arch::ArchConfig;
use crate::files::normalize_path;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub(crate) struct RawConfig {
    pub(crate) root: Option<String>,
    pub(crate) include: Option<Vec<String>>,
    pub(crate) plugins: Option<Vec<String>>,
    pub(crate) rules: Option<BTreeMap<String, serde_yaml::Value>>,
    pub(crate) arch: Option<ArchConfig>,
}

pub(crate) fn find_config(config_dir: &Path) -> Result<PathBuf, String> {
    let yaml = config_dir.join("klint.yaml");
    if yaml.exists() {
        return Ok(yaml);
    }

    let json = config_dir.join("klint.config.json");
    if json.exists() {
        return Ok(json);
    }

    Err(format!(
        "klint-rs: no config file found — create klint.yaml (or klint.config.json) at {}",
        config_dir.display()
    ))
}

pub(crate) fn read_config(path: &Path) -> Result<RawConfig, String> {
    let text = fs::read_to_string(path)
        .map_err(|err| format!("klint-rs: failed to read {}: {err}", path.display()))?;

    if path.file_name().and_then(|name| name.to_str()) == Some("klint.yaml") {
        serde_yaml::from_str(&text)
            .map_err(|_| format!("klint-rs: failed to parse {}", path.display()))
    } else {
        serde_json::from_str(&text)
            .map_err(|_| format!("klint-rs: failed to parse {}", path.display()))
    }
}

pub(crate) fn resolve_root(config_dir: &Path, root: Option<&str>) -> PathBuf {
    let configured = root.unwrap_or(".");
    let path = Path::new(configured);
    if path.is_absolute() {
        normalize_path(path)
    } else {
        normalize_path(&config_dir.join(path))
    }
}
