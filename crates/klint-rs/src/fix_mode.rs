use std::path::PathBuf;

use klint_rs::FixScope;

use crate::command::Values;

pub fn fix_scope(values: &Values) -> Result<Option<FixScope>, String> {
    let paths = values.positionals();

    if !values.flag("fix") {
        return match paths.first() {
            Some(path) => Err(format!(
                "klint: '{path}' is not a command, and file paths work only with --fix"
            )),
            None => Ok(None),
        };
    }

    Ok(Some(FixScope {
        safe_only: safe_only(values.text("fix"))?,
        paths: chosen_paths(paths),
    }))
}

fn safe_only(mode: Option<&str>) -> Result<bool, String> {
    match mode {
        None => Ok(false),
        Some("safe") => Ok(true),
        Some(other) => Err(format!(
            "klint: unknown --fix mode '{other}', expected: safe"
        )),
    }
}

fn chosen_paths(paths: &[String]) -> Option<Vec<PathBuf>> {
    if paths.is_empty() {
        return None;
    }

    Some(paths.iter().filter_map(|path| existing(path)).collect())
}

fn existing(path: &str) -> Option<PathBuf> {
    match std::fs::canonicalize(path) {
        Ok(found) => Some(found),
        Err(err) => {
            eprintln!("klint: skipping {path}: {err}");
            None
        }
    }
}
