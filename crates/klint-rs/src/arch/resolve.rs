use crate::files::{
    is_csharp_source, is_python_source, is_rust_source, is_swift_source, normalize_path,
};
use crate::syntax::scan_csharp_namespaces;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub(super) struct AliasEntry {
    prefix: String,
    base: PathBuf,
    is_wildcard: bool,
}

#[derive(Debug, Deserialize)]
struct TsConfig {
    #[serde(rename = "compilerOptions")]
    compiler_options: Option<TsCompilerOptions>,
}

#[derive(Debug, Deserialize)]
struct TsCompilerOptions {
    #[serde(rename = "baseUrl")]
    base_url: Option<String>,
    paths: Option<BTreeMap<String, Vec<String>>>,
}

pub(super) fn load_path_aliases(root: &Path) -> Vec<AliasEntry> {
    let tsconfig_path = root.join("tsconfig.json");
    let Ok(text) = fs::read_to_string(&tsconfig_path) else {
        return Vec::new();
    };
    let Ok(tsconfig) = serde_json::from_str::<TsConfig>(&text) else {
        return Vec::new();
    };
    let Some(options) = tsconfig.compiler_options else {
        return Vec::new();
    };
    let Some(paths) = options.paths else {
        return Vec::new();
    };

    let base_url = options.base_url.unwrap_or_else(|| ".".to_string());
    let base_root = normalize_path(&root.join(base_url));
    paths
        .into_iter()
        .filter_map(|(pattern, targets)| {
            let target = targets.first()?;
            let is_wildcard = pattern.ends_with("/*");
            let prefix = if is_wildcard {
                pattern.trim_end_matches("/*").to_string()
            } else {
                pattern
            };
            let target_base = target.trim_end_matches("/*");
            Some(AliasEntry {
                prefix,
                base: normalize_path(&base_root.join(target_base)),
                is_wildcard,
            })
        })
        .collect()
}

/// Project-wide module indexes, built once per run rather than once per rule.
pub(super) struct ResolveContext {
    aliases: Vec<AliasEntry>,
    python: PythonContext,
    swift_modules: BTreeMap<String, PathBuf>,
    rust_crate_roots: BTreeSet<PathBuf>,
    csharp: CSharpContext,
}

impl ResolveContext {
    pub(super) fn build(root: &Path, files: &[PathBuf]) -> Self {
        Self {
            aliases: load_path_aliases(root),
            python: PythonContext::index(root, files),
            swift_modules: index_swift_modules(root, files),
            rust_crate_roots: index_rust_crate_roots(files),
            csharp: CSharpContext::index(files),
        }
    }
}

pub(super) fn resolve_import(
    file: &Path,
    root: &Path,
    specifier: &str,
    context: &ResolveContext,
) -> Option<PathBuf> {
    if !is_bare_specifier(specifier) {
        return Some(normalize_path(
            &file.parent().unwrap_or(root).join(specifier),
        ));
    }

    resolve_alias(specifier, &context.aliases)
        .or_else(|| resolve_python_module(specifier, &context.python))
        .or_else(|| resolve_swift_module(specifier, &context.swift_modules))
        .or_else(|| resolve_rust_module(specifier, file, &context.rust_crate_roots))
        .or_else(|| resolve_csharp_namespace(specifier, file, &context.csharp))
}

/// The `src` directory of every crate the scan found, keyed off the crate root
/// files Cargo requires. A file belongs to the longest such directory that
/// prefixes it, which is what `crate::` resolves against.
pub(super) fn index_rust_crate_roots(files: &[PathBuf]) -> BTreeSet<PathBuf> {
    files
        .iter()
        .filter(|file| {
            matches!(
                file.file_name().and_then(|name| name.to_str()),
                Some("lib.rs" | "main.rs")
            )
        })
        .filter_map(|file| file.parent().map(normalize_path))
        .collect()
}

fn rust_crate_src_dir(file: &Path, crate_roots: &BTreeSet<PathBuf>) -> Option<PathBuf> {
    crate_roots
        .iter()
        .filter(|src| file.starts_with(src))
        .max_by_key(|src| src.components().count())
        .cloned()
}

/// The directory holding a file's child modules. `mod.rs`, `lib.rs`, and
/// `main.rs` are their directory's module, so their children sit beside them;
/// any other file owns a directory named after its stem.
fn rust_module_dir(file: &Path) -> Option<PathBuf> {
    let parent = file.parent()?;
    match file.file_name().and_then(|name| name.to_str()) {
        Some("mod.rs" | "lib.rs" | "main.rs") => Some(parent.to_path_buf()),
        _ => Some(parent.join(file.file_stem()?)),
    }
}

fn resolve_rust_module(
    specifier: &str,
    file: &Path,
    crate_roots: &BTreeSet<PathBuf>,
) -> Option<PathBuf> {
    if !is_rust_source(file) {
        return None;
    }

    let mut segments = specifier.split("::");
    let first = segments.next()?;
    let mut base = match first {
        "crate" => rust_crate_src_dir(file, crate_roots)?,
        "self" => rust_module_dir(file)?,
        "super" => rust_module_dir(file)?.parent()?.to_path_buf(),
        _ => return None,
    };
    let mut rest: Vec<&str> = segments.collect();
    while rest.first() == Some(&"super") {
        base = base.parent()?.to_path_buf();
        rest.remove(0);
    }

    rust_module_file(&base, &rest)
}

/// Probes the longest prefix first, because a `use` path usually ends in an
/// item rather than a module — `crate::syntax::architecture::scan_imports`
/// resolves to `syntax/architecture.rs`, not to a `scan_imports` module.
fn rust_module_file(base: &Path, segments: &[&str]) -> Option<PathBuf> {
    for depth in (0..=segments.len()).rev() {
        let candidate = segments[..depth]
            .iter()
            .fold(base.to_path_buf(), |path, segment| path.join(segment));
        for module in [candidate.with_extension("rs"), candidate.join("mod.rs")] {
            if module.is_file() {
                return Some(normalize_path(&module));
            }
        }
    }
    None
}

fn resolve_alias(specifier: &str, aliases: &[AliasEntry]) -> Option<PathBuf> {
    aliases.iter().find_map(|alias| {
        if alias.is_wildcard {
            let match_prefix = format!("{}/", alias.prefix);
            specifier
                .strip_prefix(&match_prefix)
                .map(|rest| normalize_path(&alias.base.join(rest)))
        } else if specifier == alias.prefix {
            Some(alias.base.clone())
        } else {
            None
        }
    })
}

/// Where absolute Python imports may resolve from, plus the directories that
/// hold discovered `.py` files — PEP 420 namespace packages carry no
/// `__init__.py`, so a directory is a package when the scan found sources in it.
pub(super) struct PythonContext {
    source_roots: Vec<PathBuf>,
    package_dirs: BTreeSet<PathBuf>,
}

impl PythonContext {
    pub(super) fn index(root: &Path, files: &[PathBuf]) -> Self {
        Self {
            source_roots: infer_python_source_roots(root, files),
            package_dirs: index_python_package_dirs(root, files),
        }
    }
}

fn infer_python_source_roots(root: &Path, files: &[PathBuf]) -> Vec<PathBuf> {
    let mut roots = vec![normalize_path(root)];
    for file in files.iter().filter(|file| is_python_source(file)) {
        let Ok(rel) = file.strip_prefix(root) else {
            continue;
        };
        let mut components = rel.components();
        let Some(first) = components.next() else {
            continue;
        };
        if components.next().is_none() {
            continue;
        }
        roots.push(normalize_path(&root.join(first.as_os_str())));
    }
    roots.sort();
    roots.dedup();
    roots
}

fn index_python_package_dirs(root: &Path, files: &[PathBuf]) -> BTreeSet<PathBuf> {
    let root = normalize_path(root);
    let mut dirs = BTreeSet::new();
    for file in files.iter().filter(|file| is_python_source(file)) {
        let mut current = normalize_path(file);
        while let Some(parent) = current.parent() {
            if parent == root || !parent.starts_with(&root) {
                break;
            }
            let parent = parent.to_path_buf();
            if !dirs.insert(parent.clone()) {
                break;
            }
            current = parent;
        }
    }
    dirs
}

fn resolve_python_module(specifier: &str, python: &PythonContext) -> Option<PathBuf> {
    let module_path = specifier.replace('.', "/");
    python.source_roots.iter().find_map(|source_root| {
        let file = normalize_path(&source_root.join(format!("{module_path}.py")));
        if file.exists() {
            return Some(file);
        }

        let directory = normalize_path(&source_root.join(&module_path));
        let package = directory.join("__init__.py");
        if package.exists() {
            return Some(package);
        }

        python
            .package_dirs
            .contains(&directory)
            .then_some(directory)
    })
}

pub(super) fn index_swift_modules(root: &Path, files: &[PathBuf]) -> BTreeMap<String, PathBuf> {
    let mut modules = BTreeMap::new();
    for file in files.iter().filter(|file| is_swift_source(file)) {
        if let Some(stem) = file.file_stem().and_then(|stem| stem.to_str()) {
            modules
                .entry(stem.to_string())
                .or_insert_with(|| normalize_path(file));
        }

        let Ok(rel) = file.strip_prefix(root) else {
            continue;
        };
        let mut current = normalize_path(root);
        let components = rel.components().collect::<Vec<_>>();
        for component in components.iter().take(components.len().saturating_sub(1)) {
            current = normalize_path(&current.join(component.as_os_str()));
            let Some(name) = component.as_os_str().to_str() else {
                continue;
            };
            modules
                .entry(name.to_string())
                .or_insert_with(|| current.clone());
        }
    }
    modules
}

fn resolve_swift_module(
    specifier: &str,
    swift_modules: &BTreeMap<String, PathBuf>,
) -> Option<PathBuf> {
    swift_modules.get(specifier).cloned()
}

/// Maps each declared C# namespace to a file that declares it, so a
/// `using Foo.Bar` resolves even though C# namespaces are decoupled from disk
/// layout. Building it requires reading the sources — unlike Python/Swift/Rust,
/// no path convention recovers the namespace — so this is the one index that
/// parses files rather than inferring from names.
pub(super) struct CSharpContext {
    namespaces: BTreeMap<String, PathBuf>,
}

impl CSharpContext {
    pub(super) fn index(files: &[PathBuf]) -> Self {
        let mut namespaces = BTreeMap::new();
        for file in files.iter().filter(|file| is_csharp_source(file)) {
            let Ok(content) = fs::read_to_string(file) else {
                continue;
            };
            for namespace in scan_csharp_namespaces(&content) {
                namespaces
                    .entry(namespace)
                    .or_insert_with(|| normalize_path(file));
            }
        }
        Self { namespaces }
    }
}

fn resolve_csharp_namespace(
    specifier: &str,
    file: &Path,
    csharp: &CSharpContext,
) -> Option<PathBuf> {
    if !is_csharp_source(file) {
        return None;
    }
    csharp.namespaces.get(specifier).cloned()
}

pub(super) fn is_bare_specifier(specifier: &str) -> bool {
    !specifier.starts_with('.') && !Path::new(specifier).is_absolute()
}
