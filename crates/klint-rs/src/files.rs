use ignore::WalkBuilder;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

pub(crate) fn resolve_files(
    root: &Path,
    include: &[String],
    respect_gitignore: bool,
    text_globs: &[String],
) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    let excludes: Vec<&str> = include
        .iter()
        .filter_map(|pattern| pattern.strip_prefix('!'))
        .collect();

    for pattern in include.iter().filter(|pattern| !pattern.starts_with('!')) {
        let base = include_base(root, pattern);
        let walk = Walk {
            root,
            include_pattern: pattern,
            excludes: &excludes,
            respect_gitignore,
            text_globs,
        };
        walk.collect_into(&base, &mut files);
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
                                format!("klint: failed to read {}: {err}", file.display())
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

struct Walk<'a> {
    root: &'a Path,
    include_pattern: &'a str,
    excludes: &'a [&'a str],
    respect_gitignore: bool,
    text_globs: &'a [String],
}

impl Walk<'_> {
    fn collect_into(&self, base: &Path, files: &mut Vec<PathBuf>) {
        if !walkable(base).exists() {
            return;
        }

        for entry in self.builder(base).build() {
            let Ok(entry) = entry else {
                continue;
            };
            let path = entry.path();
            let is_file = entry.file_type().is_some_and(|kind| !kind.is_dir());

            if is_file && self.wants(path) && self.selects(path) {
                files.push(normalize_path(path));
            }
        }
    }

    fn builder(&self, base: &Path) -> WalkBuilder {
        let root = normalize_path(self.root);
        let start = if base.starts_with(&root) { &root } else { base };
        let mut builder = WalkBuilder::new(walkable(start));
        builder
            .standard_filters(false)
            .follow_links(true)
            .parents(self.respect_gitignore)
            .git_ignore(self.respect_gitignore)
            .git_exclude(self.respect_gitignore)
            .require_git(true);
        let base = base.to_path_buf();
        let excludes: Vec<String> = self.excludes.iter().map(|e| (*e).to_string()).collect();
        builder.filter_entry(move |entry| {
            let is_dir = entry.file_type().is_some_and(|kind| kind.is_dir());
            keeps_entry(
                &normalize_path(entry.path()),
                is_dir,
                &base,
                &root,
                &excludes,
            )
        });
        builder
    }

    fn wants(&self, path: &Path) -> bool {
        let rel = || relative_path(&normalize_path(self.root), &normalize_path(path));

        is_supported_source(path)
            || self
                .text_globs
                .iter()
                .any(|glob| match_pattern(&rel(), glob))
    }

    fn selects(&self, path: &Path) -> bool {
        let rel = relative_path(&normalize_path(self.root), &normalize_path(path));

        match_pattern(&rel, self.include_pattern) && !is_excluded(&rel, self.excludes)
    }
}

fn walkable(path: &Path) -> &Path {
    if path.as_os_str().is_empty() {
        Path::new(".")
    } else {
        path
    }
}

fn keeps_entry(path: &Path, is_dir: bool, base: &Path, root: &Path, excludes: &[String]) -> bool {
    let leads_to_base = base.starts_with(path) || path.starts_with(base);

    leads_to_base && !(is_dir && is_excluded_dir(&relative_path(root, path), excludes))
}

fn is_excluded_dir(rel: &str, excludes: &[String]) -> bool {
    excludes
        .iter()
        .any(|pattern| match_pattern(rel, pattern) || match_pattern(&format!("{rel}/"), pattern))
}

fn is_excluded(rel: &str, excludes: &[&str]) -> bool {
    excludes.iter().any(|pattern| match_pattern(rel, pattern))
}

pub(crate) fn is_supported_source(path: &Path) -> bool {
    supports_import_scan(path)
}

pub(crate) fn is_javascript_like_source(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("ts" | "tsx" | "js" | "jsx" | "mts" | "cts" | "mjs" | "cjs")
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

pub(crate) fn is_csharp_source(path: &Path) -> bool {
    matches!(path.extension().and_then(|ext| ext.to_str()), Some("cs"))
}

pub(crate) fn supports_import_scan(path: &Path) -> bool {
    is_javascript_like_source(path)
        || is_python_source(path)
        || is_swift_source(path)
        || is_rust_source(path)
        || is_csharp_source(path)
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

fn without_leading_dot_slash(pattern: &str) -> String {
    let mut rest = pattern;

    while let Some(stripped) = rest.strip_prefix("./") {
        rest = stripped;
    }

    rest.to_string()
}

pub(crate) fn match_pattern(rel_path: &str, pattern: &str) -> bool {
    let norm = rel_path.replace('\\', "/");
    let pattern = without_leading_dot_slash(&pattern.replace('\\', "/"));
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

pub(crate) fn match_component(pattern: &str, text: &str) -> bool {
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
mod tests;
