use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

pub(crate) fn resolve_files(root: &Path, include: &[String]) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    let excludes: Vec<&str> = include
        .iter()
        .filter_map(|pattern| pattern.strip_prefix('!'))
        .collect();
    for pattern in include.iter().filter(|pattern| !pattern.starts_with('!')) {
        let base = include_base(root, pattern);
        collect_source_files(&base, root, pattern, &excludes, &mut files)?;
    }
    files.sort();
    files.dedup();
    Ok(files)
}

/// Reads every file into a `Vec` positionally aligned with `files`, so the
/// engine indexes contents rather than looking them up by path. Reads run
/// across threads because they are independent and I/O-bound.
pub(crate) fn read_files(files: &[PathBuf]) -> Result<Vec<String>, String> {
    let next = AtomicUsize::new(0);
    let workers = std::thread::available_parallelism()
        .map_or(1, |count| count.get())
        .min(files.len().max(1));

    let claimed = std::thread::scope(|scope| {
        (0..workers)
            .map(|_| {
                scope.spawn(|| {
                    let mut read = Vec::new();
                    loop {
                        let index = next.fetch_add(1, AtomicOrdering::Relaxed);
                        let Some(file) = files.get(index) else {
                            return read;
                        };
                        read.push((
                            index,
                            fs::read_to_string(file).map_err(|err| {
                                format!("klint-rs: failed to read {}: {err}", file.display())
                            }),
                        ));
                    }
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .flat_map(|handle| handle.join().expect("file read thread panicked"))
            .collect::<Vec<_>>()
    });

    let mut contents = vec![String::new(); files.len()];
    for (index, result) in claimed {
        contents[index] = result?;
    }
    Ok(contents)
}

fn include_base(root: &Path, pattern: &str) -> PathBuf {
    let mut base = PathBuf::new();
    for segment in pattern.split('/') {
        if segment.contains('*') {
            break;
        }
        base.push(segment);
    }
    normalize_path(&root.join(base))
}

fn collect_source_files(
    dir: &Path,
    root: &Path,
    include_pattern: &str,
    excludes: &[&str],
    files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(());
    };

    for entry in entries {
        let entry = entry.map_err(|err| format!("klint-rs: failed to read dir entry: {err}"))?;
        let path = entry.path();
        if path.is_dir() {
            let rel = relative_path(root, &path);
            if is_excluded(&rel, excludes) || is_excluded(&format!("{rel}/"), excludes) {
                continue;
            }
            collect_source_files(&path, root, include_pattern, excludes, files)?;
        } else if is_supported_source(&path) {
            let rel = relative_path(root, &path);
            if match_pattern(&rel, include_pattern) && !is_excluded(&rel, excludes) {
                files.push(normalize_path(&path));
            }
        }
    }
    Ok(())
}

fn is_excluded(rel: &str, excludes: &[&str]) -> bool {
    excludes.iter().any(|pattern| match_pattern(rel, pattern))
}

fn is_supported_source(path: &Path) -> bool {
    supports_import_scan(path)
}

pub(crate) fn is_javascript_like_source(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("ts" | "tsx" | "js" | "jsx" | "mts" | "cts")
    )
}

pub(crate) fn is_python_source(path: &Path) -> bool {
    matches!(path.extension().and_then(|ext| ext.to_str()), Some("py"))
}

pub(crate) fn is_swift_source(path: &Path) -> bool {
    matches!(path.extension().and_then(|ext| ext.to_str()), Some("swift"))
}

pub(crate) fn is_rust_source(path: &Path) -> bool {
    matches!(path.extension().and_then(|ext| ext.to_str()), Some("rs"))
}

pub(crate) fn supports_import_scan(path: &Path) -> bool {
    is_javascript_like_source(path)
        || is_python_source(path)
        || is_swift_source(path)
        || is_rust_source(path)
}

pub(crate) fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
            Component::RootDir | Component::Prefix(_) => normalized.push(component.as_os_str()),
        }
    }

    let text = normalized.to_string_lossy().replace('\\', "/");
    PathBuf::from(text)
}

pub(crate) fn relative_path(root: &Path, file: &Path) -> String {
    file.strip_prefix(root)
        .unwrap_or(file)
        .to_string_lossy()
        .replace('\\', "/")
}

pub(crate) fn match_pattern(rel_path: &str, pattern: &str) -> bool {
    let norm = rel_path.replace('\\', "/");
    let pattern = pattern.replace('\\', "/");
    if pattern == "." || pattern == "**" {
        return true;
    }
    if !pattern.contains('*') {
        return norm == pattern || norm.starts_with(&format!("{pattern}/"));
    }

    glob_match(&pattern, &norm)
}

fn glob_match(pattern: &str, path: &str) -> bool {
    let pattern_parts = pattern
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let path_parts = path
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();

    match_segments(&pattern_parts, &path_parts)
}

fn match_segments(pattern: &[&str], path: &[&str]) -> bool {
    if pattern.is_empty() {
        return path.is_empty();
    }

    if pattern[0] == "**" {
        return match_segments(&pattern[1..], path)
            || (!path.is_empty() && match_segments(pattern, &path[1..]));
    }

    !path.is_empty()
        && match_component(pattern[0], path[0])
        && match_segments(&pattern[1..], &path[1..])
}

fn match_component(pattern: &str, text: &str) -> bool {
    let pattern = pattern.as_bytes();
    let text = text.as_bytes();
    let mut pattern_index = 0;
    let mut text_index = 0;
    let mut star_index = None;
    let mut match_index = 0;

    while text_index < text.len() {
        if pattern_index < pattern.len()
            && (pattern[pattern_index] == b'?' || pattern[pattern_index] == text[text_index])
        {
            pattern_index += 1;
            text_index += 1;
        } else if pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
            star_index = Some(pattern_index);
            match_index = text_index;
            pattern_index += 1;
        } else if let Some(star) = star_index {
            pattern_index = star + 1;
            match_index += 1;
            text_index = match_index;
        } else {
            return false;
        }
    }

    while pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
        pattern_index += 1;
    }

    pattern_index == pattern.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir_all, write};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(name: &str) -> PathBuf {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be monotonic enough for tests")
            .as_nanos();
        std::env::temp_dir().join(format!("klint-rs-files-{name}-{id}"))
    }

    #[test]
    fn resolve_files_prunes_nested_excluded_directories() {
        let root = temp_root("exclude-node-modules");
        create_dir_all(root.join("src")).expect("create src");
        create_dir_all(root.join("assets/skill/node_modules/pkg"))
            .expect("create nested node_modules");
        write(root.join("src/index.ts"), "export const value = 1;\n").expect("write source");
        write(
            root.join("assets/skill/node_modules/pkg/index.ts"),
            "export const dependency = 1;\n",
        )
        .expect("write dependency source");

        let files = resolve_files(
            &root,
            &[
                "src".to_string(),
                "assets".to_string(),
                "!**/node_modules/**".to_string(),
            ],
        )
        .expect("resolve files");

        let rel_files = files
            .iter()
            .map(|file| relative_path(&root, file))
            .collect::<Vec<_>>();
        assert_eq!(rel_files, vec!["src/index.ts"]);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn resolve_files_includes_multilanguage_sources_for_architecture_rules() {
        let root = temp_root("multilanguage-source");
        create_dir_all(root.join("src/app")).expect("create src dirs");
        write(root.join("src/app/main.py"), "print('x')\n").expect("write python source");
        write(root.join("src/app/main.swift"), "print(\"x\")\n").expect("write swift source");
        write(root.join("src/app/main.ts"), "console.log('x');\n").expect("write ts source");
        write(root.join("src/app/readme.md"), "# ignored\n").expect("write markdown");

        let files = resolve_files(&root, &["src".to_string()]).expect("resolve files");

        let rel_files = files
            .iter()
            .map(|file| relative_path(&root, file))
            .collect::<Vec<_>>();
        assert_eq!(
            rel_files,
            vec!["src/app/main.py", "src/app/main.swift", "src/app/main.ts"]
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn resolve_files_matches_root_level_files_under_double_star_glob() {
        // `**` means zero-or-more path segments, so `**/*.ts` must match a root-level
        // file. Regression test for include_base collapsing it to a literal directory.
        let root = temp_root("double-star-root");
        create_dir_all(root.join("src")).expect("create src");
        write(root.join("cli.ts"), "export const x = 1;\n").expect("write root source");
        write(root.join("src/app.ts"), "export const y = 2;\n").expect("write nested source");

        let files = resolve_files(&root, &["**/*.ts".to_string()]).expect("resolve files");

        let rel_files = files
            .iter()
            .map(|file| relative_path(&root, file))
            .collect::<Vec<_>>();
        assert_eq!(rel_files, vec!["cli.ts", "src/app.ts"]);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn resolve_files_honors_glob_extension_not_just_base_dir() {
        // `**/*.ts` selects .ts only — the walker must match each file against the
        // glob, not collect every supported source type under the base directory.
        let root = temp_root("glob-extension");
        create_dir_all(root.join("src")).expect("create src");
        write(root.join("src/a.ts"), "export const x = 1;\n").expect("write ts");
        write(root.join("src/a.js"), "export const y = 2;\n").expect("write js");
        write(root.join("src/a.py"), "y = 2\n").expect("write py");

        let files = resolve_files(&root, &["**/*.ts".to_string()]).expect("resolve files");

        let rel_files = files
            .iter()
            .map(|file| relative_path(&root, file))
            .collect::<Vec<_>>();
        assert_eq!(rel_files, vec!["src/a.ts"]);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn resolve_files_excludes_individual_files_not_only_directories() {
        let root = temp_root("exclude-file-glob");
        create_dir_all(root.join("src/jobs")).expect("create src dirs");
        write(root.join("src/jobs/worker.py"), "y = 1\n").expect("write worker");
        write(root.join("src/jobs/worker.test.py"), "y = 2\n").expect("write worker test");
        write(root.join("src/jobs/keep.py"), "y = 3\n").expect("write keep");

        let files = resolve_files(
            &root,
            &[
                "src".to_string(),
                "!**/*.test.py".to_string(),
                "!src/jobs/worker.py".to_string(),
            ],
        )
        .expect("resolve files");

        let rel_files = files
            .iter()
            .map(|file| relative_path(&root, file))
            .collect::<Vec<_>>();
        assert_eq!(rel_files, vec!["src/jobs/keep.py"]);

        let _ = fs::remove_dir_all(root);
    }
}
