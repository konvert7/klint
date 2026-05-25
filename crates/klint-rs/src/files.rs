use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn resolve_files(root: &Path, include: &[String]) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for pattern in include.iter().filter(|pattern| !pattern.starts_with('!')) {
        let base = include_base(root, pattern);
        collect_ts_files(&base, &mut files)?;
    }
    files.sort();
    files.dedup();
    Ok(files)
}

pub(crate) fn read_files(files: &[PathBuf]) -> Result<BTreeMap<PathBuf, String>, String> {
    let mut contents = BTreeMap::new();
    for file in files {
        let content = fs::read_to_string(file)
            .map_err(|err| format!("klint-rs: failed to read {}: {err}", file.display()))?;
        contents.insert(file.clone(), content);
    }
    Ok(contents)
}

fn include_base(root: &Path, pattern: &str) -> PathBuf {
    let prefix = pattern
        .split("/**")
        .next()
        .unwrap_or(pattern)
        .split("/*")
        .next()
        .unwrap_or(pattern);
    normalize_path(&root.join(prefix))
}

fn collect_ts_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(());
    };

    for entry in entries {
        let entry = entry.map_err(|err| format!("klint-rs: failed to read dir entry: {err}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_ts_files(&path, files)?;
        } else if is_supported_source(&path) {
            files.push(normalize_path(&path));
        }
    }
    Ok(())
}

fn is_supported_source(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("ts" | "tsx" | "js" | "jsx" | "mts" | "cts")
    )
}

pub(crate) fn normalize_path(path: &Path) -> PathBuf {
    let text = path.to_string_lossy().replace('\\', "/");
    PathBuf::from(text)
}

pub(crate) fn relative_path(root: &Path, file: &Path) -> String {
    file.strip_prefix(root)
        .unwrap_or(file)
        .to_string_lossy()
        .replace('\\', "/")
}
