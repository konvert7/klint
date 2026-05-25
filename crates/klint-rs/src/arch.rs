use crate::files::{normalize_path, relative_path};
use crate::output::Violation;
use crate::syntax::{scan_imports, scan_jsx_elements};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub(crate) struct ArchConfig {
    layers: Option<BTreeMap<String, Vec<String>>>,
    imports: Option<Vec<ArchImportRule>>,
    forbidden: Option<Vec<ArchForbiddenRule>>,
    singleton: Option<Vec<ArchSingletonRule>>,
}

#[derive(Debug, Deserialize)]
struct ArchImportRule {
    from: StringOrVec,
    deny: Option<StringOrVec>,
    allow: Option<StringOrVec>,
    #[serde(rename = "type-only")]
    type_only: Option<String>,
    message: Option<String>,
    severity: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ArchForbiddenRule {
    pattern: Option<String>,
    #[serde(rename = "jsx-element")]
    jsx_element: Option<StringOrVec>,
    #[serde(rename = "in")]
    in_scope: StringOrVec,
    message: String,
    severity: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ArchSingletonRule {
    pattern: Option<String>,
    #[serde(rename = "jsx-element")]
    jsx_element: Option<StringOrVec>,
    only: String,
    #[serde(rename = "in")]
    in_scope: Option<StringOrVec>,
    message: String,
    severity: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum StringOrVec {
    One(String),
    Many(Vec<String>),
}

struct PatternScan<'a> {
    rule_name: &'a str,
    pattern: &'a str,
    message: &'a str,
    severity: &'a str,
}

struct ElementScan<'a> {
    rule_name: &'a str,
    message: &'a str,
    severity: &'a str,
}

#[derive(Debug)]
struct AliasEntry {
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

pub(crate) fn run_arch_rules(
    arch: &ArchConfig,
    files: &[PathBuf],
    file_contents: &BTreeMap<PathBuf, String>,
    root: &Path,
    violations: &mut Vec<Violation>,
) {
    run_arch_import_rules(arch, files, file_contents, root, violations);
    run_arch_forbidden_rules(arch, files, file_contents, root, violations);
    run_arch_singleton_rules(arch, files, file_contents, root, violations);
}

fn run_arch_import_rules(
    arch: &ArchConfig,
    files: &[PathBuf],
    file_contents: &BTreeMap<PathBuf, String>,
    root: &Path,
    violations: &mut Vec<Violation>,
) {
    let Some(rules) = &arch.imports else {
        return;
    };
    let aliases = load_path_aliases(root);

    for rule in rules {
        if rule.deny.is_none() && rule.allow.is_none() {
            continue;
        }
        let allow_type_only = rule.type_only.as_deref() == Some("allow");

        let severity = rule.severity.as_deref().unwrap_or("error");
        let from_files = resolve_layer_files(&rule.from, arch.layers.as_ref(), root, files);
        let deny_prefixes = rule
            .deny
            .as_ref()
            .map(|deny| resolve_layer_prefixes(deny, arch.layers.as_ref(), root));
        let allow_prefixes = rule
            .allow
            .as_ref()
            .map(|allow| resolve_layer_prefixes(allow, arch.layers.as_ref(), root));

        for file in from_files {
            let Some(content) = file_contents.get(&file) else {
                continue;
            };
            let Ok(imports) = scan_imports(&file, content) else {
                continue;
            };

            for import in imports {
                if allow_type_only && import.is_type_only {
                    continue;
                }
                let Some(resolved) = resolve_import(&file, root, &import.specifier, &aliases)
                else {
                    continue;
                };

                let message = if let Some(prefixes) = &deny_prefixes {
                    if !in_prefixes(&resolved, prefixes) {
                        continue;
                    }
                    rule.message
                        .as_deref()
                        .unwrap_or("Import crosses a denied boundary")
                } else if let Some(prefixes) = &allow_prefixes {
                    if in_prefixes(&resolved, prefixes) {
                        continue;
                    }
                    rule.message
                        .as_deref()
                        .unwrap_or("Import is not in the allowed list")
                } else {
                    continue;
                };

                violations.push(Violation {
                    file: relative_path(root, &file),
                    line: import.line,
                    rule: "arch/imports".to_string(),
                    message: message.to_string(),
                    severity: severity.to_string(),
                    fix: None,
                });
            }
        }
    }
}

fn load_path_aliases(root: &Path) -> Vec<AliasEntry> {
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

fn resolve_import(
    file: &Path,
    root: &Path,
    specifier: &str,
    aliases: &[AliasEntry],
) -> Option<PathBuf> {
    if !is_bare_specifier(specifier) {
        return Some(normalize_path(
            &file.parent().unwrap_or(root).join(specifier),
        ));
    }

    resolve_alias(specifier, aliases)
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

fn run_arch_forbidden_rules(
    arch: &ArchConfig,
    files: &[PathBuf],
    file_contents: &BTreeMap<PathBuf, String>,
    root: &Path,
    violations: &mut Vec<Violation>,
) {
    let Some(rules) = &arch.forbidden else {
        return;
    };

    for rule in rules {
        let scoped_files = resolve_layer_files(&rule.in_scope, arch.layers.as_ref(), root, files);
        if let Some(tags) = &rule.jsx_element {
            scan_jsx_elements_for_targets(
                &scoped_files,
                tags,
                file_contents,
                root,
                ElementScan {
                    rule_name: "arch/forbidden",
                    message: &rule.message,
                    severity: rule.severity.as_deref().unwrap_or("error"),
                },
                violations,
            );
            continue;
        }

        let Some(pattern) = &rule.pattern else {
            continue;
        };
        scan_lines_for_pattern(
            &scoped_files,
            file_contents,
            root,
            PatternScan {
                rule_name: "arch/forbidden",
                pattern,
                message: &rule.message,
                severity: rule.severity.as_deref().unwrap_or("error"),
            },
            violations,
        );
    }
}

fn resolve_layer_prefixes(
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

fn run_arch_singleton_rules(
    arch: &ArchConfig,
    files: &[PathBuf],
    file_contents: &BTreeMap<PathBuf, String>,
    root: &Path,
    violations: &mut Vec<Violation>,
) {
    let Some(rules) = &arch.singleton else {
        return;
    };

    for rule in rules {
        let only_file = normalize_path(&root.join(&rule.only));
        let scoped_files = match &rule.in_scope {
            Some(scope) => resolve_layer_files(scope, arch.layers.as_ref(), root, files),
            None => files.to_vec(),
        };
        let checked_files = scoped_files
            .into_iter()
            .filter(|file| file != &only_file)
            .collect::<Vec<_>>();

        if let Some(tags) = &rule.jsx_element {
            scan_jsx_elements_for_targets(
                &checked_files,
                tags,
                file_contents,
                root,
                ElementScan {
                    rule_name: "arch/singleton",
                    message: &rule.message,
                    severity: rule.severity.as_deref().unwrap_or("error"),
                },
                violations,
            );
            continue;
        }

        let Some(pattern) = &rule.pattern else {
            continue;
        };
        scan_lines_for_pattern(
            &checked_files,
            file_contents,
            root,
            PatternScan {
                rule_name: "arch/singleton",
                pattern,
                message: &rule.message,
                severity: rule.severity.as_deref().unwrap_or("error"),
            },
            violations,
        );
    }
}

fn resolve_layer_files(
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

impl StringOrVec {
    fn items(&self) -> Vec<String> {
        match self {
            Self::One(item) => vec![item.clone()],
            Self::Many(items) => items.clone(),
        }
    }
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

fn in_prefixes(path: &Path, prefixes: &[PathBuf]) -> bool {
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

fn is_bare_specifier(specifier: &str) -> bool {
    !specifier.starts_with('.') && !Path::new(specifier).is_absolute()
}

fn scan_jsx_elements_for_targets(
    files: &[PathBuf],
    targets: &StringOrVec,
    file_contents: &BTreeMap<PathBuf, String>,
    root: &Path,
    scan: ElementScan<'_>,
    violations: &mut Vec<Violation>,
) {
    let target_names = targets.items();
    for file in files {
        let Some(content) = file_contents.get(file) else {
            continue;
        };
        let Ok(elements) = scan_jsx_elements(file, content) else {
            continue;
        };

        for element in elements {
            if !target_names.contains(&element.tag_name) {
                continue;
            }
            violations.push(Violation {
                file: relative_path(root, file),
                line: element.line,
                rule: scan.rule_name.to_string(),
                message: scan.message.to_string(),
                severity: scan.severity.to_string(),
                fix: None,
            });
        }
    }
}

fn scan_lines_for_pattern(
    files: &[PathBuf],
    file_contents: &BTreeMap<PathBuf, String>,
    root: &Path,
    scan: PatternScan<'_>,
    violations: &mut Vec<Violation>,
) {
    for file in files {
        let Some(content) = file_contents.get(file) else {
            continue;
        };

        for (index, line) in content.lines().enumerate() {
            if line.contains(scan.pattern) {
                violations.push(Violation {
                    file: relative_path(root, file),
                    line: index + 1,
                    rule: scan.rule_name.to_string(),
                    message: scan.message.to_string(),
                    severity: scan.severity.to_string(),
                    fix: None,
                });
            }
        }
    }
}
