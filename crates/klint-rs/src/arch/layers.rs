use super::*;
use crate::files::normalize_path;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub(super) fn resolve_layer_prefixes(
    scope: &StringOrVec,
    layers: Option<&BTreeMap<String, Vec<String>>>,
    root: &Path,
) -> Vec<PathBuf> {
    resolve_globs(scope, layers)
        .iter()
        .filter(|glob| !glob.starts_with('!'))
        .map(|glob| glob_to_prefix(glob, root))
        .collect()
}

pub(super) fn resolve_layer_files(
    scope: &StringOrVec,
    layers: Option<&BTreeMap<String, Vec<String>>>,
    root: &Path,
    all_files: &[PathBuf],
) -> Vec<PathBuf> {
    let globs = resolve_globs(scope, layers);
    let include_prefixes: Vec<PathBuf> = globs
        .iter()
        .filter(|glob| !glob.starts_with('!'))
        .map(|glob| glob_to_prefix(glob, root))
        .collect();
    let exclude_prefixes: Vec<PathBuf> = globs
        .iter()
        .filter(|glob| glob.starts_with('!'))
        .map(|glob| glob_to_prefix(&glob[1..], root))
        .collect();

    all_files
        .iter()
        .filter(|file| {
            include_prefixes
                .iter()
                .any(|prefix| path_in_prefix(file, prefix))
                && !exclude_prefixes
                    .iter()
                    .any(|prefix| path_in_prefix(file, prefix))
        })
        .cloned()
        .collect()
}

fn resolve_globs(
    scope: &StringOrVec,
    layers: Option<&BTreeMap<String, Vec<String>>>,
) -> Vec<String> {
    scope
        .items()
        .iter()
        .flat_map(|item| {
            layers
                .and_then(|known| known.get(item))
                .cloned()
                .unwrap_or_else(|| vec![item.clone()])
        })
        .collect()
}

fn glob_to_prefix(glob: &str, root: &Path) -> PathBuf {
    let prefix = glob
        .split("/**")
        .next()
        .unwrap_or(glob)
        .split("/*")
        .next()
        .unwrap_or(glob)
        .split('*')
        .next()
        .unwrap_or(glob);
    normalize_path(&root.join(prefix))
}

fn path_in_prefix(path: &Path, prefix: &Path) -> bool {
    path == prefix || path.starts_with(prefix)
}

pub(super) fn in_prefixes(path: &Path, prefixes: &[PathBuf]) -> bool {
    prefixes.iter().any(|prefix| {
        if path_in_prefix(path, prefix) {
            return true;
        }
        let Some(prefix_text) = prefix.to_str() else {
            return false;
        };
        let bare_prefix = prefix_text
            .strip_suffix(".ts")
            .or_else(|| prefix_text.strip_suffix(".tsx"))
            .or_else(|| prefix_text.strip_suffix(".js"))
            .or_else(|| prefix_text.strip_suffix(".jsx"))
            .or_else(|| prefix_text.strip_suffix(".mts"))
            .or_else(|| prefix_text.strip_suffix(".cts"));
        bare_prefix.is_some_and(|bare| {
            let bare_path = PathBuf::from(bare);
            path_in_prefix(path, &bare_path)
        })
    })
}
