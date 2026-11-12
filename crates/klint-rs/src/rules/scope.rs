use std::path::Path;

use crate::files::{is_javascript_like_source, match_pattern, relative_path, supports_import_scan};

#[derive(Clone, Copy)]
pub(super) enum FileScope {
    JavaScriptLike,
    AnySource,
}

pub(super) fn covering_scope(
    scope: FileScope,
    includes: &[Option<&[String]>],
    root: &Path,
    file: &Path,
) -> Option<usize> {
    if !scope_covers_file(scope, file) {
        return None;
    }

    includes
        .iter()
        .rposition(|include| include_matches(*include, root, file))
}

fn include_matches(include: Option<&[String]>, root: &Path, file: &Path) -> bool {
    let Some(include) = include else {
        return true;
    };

    let rel = relative_path(root, file);
    let includes = include
        .iter()
        .filter(|pattern| !pattern.starts_with('!'))
        .collect::<Vec<_>>();
    let excluded = include
        .iter()
        .filter_map(|pattern| pattern.strip_prefix('!'))
        .any(|pattern| match_pattern(&rel, pattern));

    !excluded
        && (includes.is_empty() || includes.iter().any(|pattern| match_pattern(&rel, pattern)))
}

fn scope_covers_file(scope: FileScope, file: &Path) -> bool {
    match scope {
        FileScope::JavaScriptLike => is_javascript_like_source(file),
        FileScope::AnySource => supports_import_scan(file),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project_root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn javascript_scope_skips_a_python_file() {
        let root = project_root();
        let python = root.join("src/a.py");
        assert_eq!(
            covering_scope(FileScope::JavaScriptLike, &[None], root, &python),
            None
        );
    }

    #[test]
    fn any_source_scope_accepts_a_python_file() {
        let root = project_root();
        let python = root.join("src/a.py");
        assert_eq!(
            covering_scope(FileScope::AnySource, &[None], root, &python),
            Some(0)
        );
    }

    #[test]
    fn exclude_pattern_removes_a_file_matched_by_an_include_glob() {
        let root = project_root();
        let include = ["src/keep/**".to_string(), "!src/keep/skip/**".to_string()];
        let file = root.join("src/keep/skip/a.ts");
        assert!(!include_matches(Some(&include[..]), root, &file));
    }

    #[test]
    fn include_glob_accepts_a_matching_rust_file() {
        let root = project_root();
        let include = ["src/**".to_string()];
        assert!(include_matches(
            Some(&include[..]),
            root,
            &root.join("src/a.rs")
        ));
    }

    #[test]
    fn a_later_matching_entry_is_the_one_that_covers_the_file() {
        let root = project_root();
        let scripts = ["scripts/**".to_string()];
        let src = ["src/**".to_string()];
        type Row<'a> = (&'a [Option<&'a [String]>], &'a str, Option<usize>);
        let rows: [Row<'_>; 4] = [
            (&[None, Some(&scripts[..])], "scripts/a.ts", Some(1)),
            (&[Some(&scripts[..]), None], "scripts/a.ts", Some(1)),
            (&[Some(&src[..])], "tools/x.ts", None),
            (&[], "src/a.ts", None),
        ];

        for (includes, file, expected) in rows {
            let covered = covering_scope(FileScope::AnySource, includes, root, &root.join(file));
            assert_eq!(covered, expected, "{file}");
        }
    }
}
