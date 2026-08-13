pub(crate) mod install;

use crate::output::Violation;
use crate::version::reported_version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

const SKILL_MARKDOWN: &str = include_str!("../../../../skill/klint-rules/SKILL.md");
const SKILL_DIR_NAME: &str = "klint-rules";
const SKILL_FILE_NAME: &str = "SKILL.md";
const RECEIPT_FILE_NAME: &str = ".klint-skill.json";
const CANONICAL_SKILL_DIR: &str = ".agents/skills";
const STALE_RULE: &str = "klint/skill-stale";
const LEGACY_RULE: &str = "klint/skill-legacy-link";
const LEGACY_TARGET: &str = "node_modules";

const AGENT_SKILL_DIRS: [(&str, &str); 4] = [
    ("claude", ".claude/skills"),
    ("codex", ".agents/skills"),
    ("cursor", ".cursor/skills"),
    ("opencode", ".agents/skills"),
];

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct SkillReceipt {
    version: String,
    sha256: String,
}

pub fn shipped_skill_hash() -> String {
    let mut hasher = Sha256::new();
    hasher.update(SKILL_MARKDOWN.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub(crate) fn skill_advisories(root: &Path) -> Vec<Violation> {
    let mut advisories = legacy_link_advisories(root);
    advisories.extend(stale_skill_advisories(root));
    advisories
}

fn legacy_link_advisories(root: &Path) -> Vec<Violation> {
    unique_agent_dirs()
        .into_iter()
        .filter_map(|dir| legacy_advisory_for(root, &dir))
        .collect()
}

fn legacy_advisory_for(root: &Path, dir: &str) -> Option<Violation> {
    let target = symlink_target(&root.join(dir).join(SKILL_DIR_NAME))?;
    if !target.contains(LEGACY_TARGET) {
        return None;
    }

    Some(Violation {
        file: format!("{dir}/{SKILL_DIR_NAME}"),
        line: 1,
        rule: LEGACY_RULE.to_string(),
        message: format!(
            "this skill is a symlink into {LEGACY_TARGET} ({target}), which breaks as soon as dependencies are reinstalled — run: klint install-skill"
        ),
        severity: "warn".to_string(),
        fix: None,
    })
}

fn stale_skill_advisories(root: &Path) -> Vec<Violation> {
    let shipped = shipped_skill_hash();
    let installed = reported_version();
    let mut reported: Vec<PathBuf> = Vec::new();

    real_skill_dirs_first(root)
        .into_iter()
        .filter_map(|dir| {
            let skill_dir = root.join(&dir).join(SKILL_DIR_NAME);
            let canonical = fs::canonicalize(&skill_dir).ok()?;
            if reported.contains(&canonical) {
                return None;
            }

            let receipt = read_receipt(&skill_dir.join(RECEIPT_FILE_NAME))?;
            if receipt.sha256 == shipped {
                return None;
            }

            reported.push(canonical);
            Some(stale_advisory(&dir, &receipt.version, &installed))
        })
        .collect()
}

fn stale_advisory(dir: &str, installed_from: &str, installed: &str) -> Violation {
    Violation {
        file: format!("{dir}/{SKILL_DIR_NAME}/{SKILL_FILE_NAME}"),
        line: 1,
        rule: STALE_RULE.to_string(),
        message: format!(
            "this klint-rules skill was installed from klint {installed_from} and no longer matches klint {installed} — reinstall it with: klint install-skill"
        ),
        severity: "warn".to_string(),
        fix: None,
    }
}

fn real_skill_dirs_first(root: &Path) -> Vec<String> {
    let (linked, real): (Vec<String>, Vec<String>) = unique_agent_dirs()
        .into_iter()
        .partition(|dir| symlink_target(&root.join(dir).join(SKILL_DIR_NAME)).is_some());
    real.into_iter().chain(linked).collect()
}

fn symlink_target(path: &Path) -> Option<String> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if !metadata.is_symlink() {
        return None;
    }
    Some(fs::read_link(path).ok()?.to_string_lossy().to_string())
}

fn read_receipt(path: &Path) -> Option<SkillReceipt> {
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
}

fn agent_dirs_for(agents: &[String]) -> Result<Vec<String>, String> {
    if agents.is_empty() {
        return Ok(unique_agent_dirs());
    }

    let selected = agents
        .iter()
        .map(|agent| dir_for_agent(agent))
        .collect::<Result<Vec<&str>, String>>()?;
    Ok(deduplicated(&selected))
}

fn dir_for_agent(agent: &str) -> Result<&'static str, String> {
    AGENT_SKILL_DIRS
        .iter()
        .find(|(name, _)| *name == agent)
        .map(|(_, dir)| *dir)
        .ok_or_else(|| {
            format!(
                "klint: unknown agent \"{agent}\" (expected one of: {})",
                known_agent_names()
            )
        })
}

pub fn known_agent_names() -> String {
    AGENT_SKILL_DIRS
        .iter()
        .map(|(name, _)| *name)
        .collect::<Vec<&str>>()
        .join(", ")
}

fn unique_agent_dirs() -> Vec<String> {
    let all: Vec<&str> = AGENT_SKILL_DIRS.iter().map(|(_, dir)| *dir).collect();
    deduplicated(&all)
}

fn deduplicated(dirs: &[&str]) -> Vec<String> {
    let mut unique: Vec<String> = Vec::new();
    for dir in dirs {
        if !unique.iter().any(|seen| seen == dir) {
            unique.push((*dir).to_string());
        }
    }
    unique
}

#[cfg(test)]
pub(crate) mod tests {
    use super::install::{InstallRequest, install_skill};
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    pub(crate) fn temp_root(name: &str) -> PathBuf {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be monotonic enough for tests")
            .as_nanos();
        std::env::temp_dir().join(format!("klint-skill-{name}-{id}"))
    }

    fn write_receipt(root: &Path, dir: &str, receipt: &SkillReceipt) {
        let skill_dir = root.join(dir).join(SKILL_DIR_NAME);
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        fs::write(
            skill_dir.join(RECEIPT_FILE_NAME),
            serde_json::to_string(receipt).expect("receipt should serialize"),
        )
        .expect("write receipt");
    }

    fn stale_receipt() -> SkillReceipt {
        SkillReceipt {
            version: "0.1.0".to_string(),
            sha256: "0".repeat(64),
        }
    }

    #[test]
    fn shipped_skill_hash_is_a_sha256_digest() {
        let hash = shipped_skill_hash();
        assert_eq!(hash.len(), 64);
        assert!(hash.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert_eq!(hash, shipped_skill_hash());
    }

    #[test]
    fn embedded_skill_carries_the_klint_rules_front_matter() {
        assert!(SKILL_MARKDOWN.contains("name: klint-rules"));
    }

    #[test]
    fn every_agent_resolves_to_a_directory() {
        for (name, dir) in AGENT_SKILL_DIRS {
            assert_eq!(dir_for_agent(name), Ok(dir));
        }
    }

    #[test]
    fn agents_sharing_a_directory_are_installed_once() {
        let dirs = agent_dirs_for(&["codex".to_string(), "opencode".to_string()])
            .expect("known agents should resolve");
        assert_eq!(dirs, vec![CANONICAL_SKILL_DIR.to_string()]);
    }

    #[test]
    fn no_agents_selects_every_unique_directory() {
        let dirs = agent_dirs_for(&[]).expect("the default selection should resolve");
        assert_eq!(
            dirs,
            vec![
                ".claude/skills".to_string(),
                ".agents/skills".to_string(),
                ".cursor/skills".to_string(),
            ]
        );
    }

    #[test]
    fn an_unknown_agent_is_rejected() {
        let error = agent_dirs_for(&["emacs".to_string()])
            .expect_err("an unknown agent should be rejected");
        assert!(error.contains("unknown agent \"emacs\""));
        assert!(error.contains("claude"));
    }

    #[test]
    fn a_fresh_shared_install_produces_no_advisory() {
        let root = temp_root("advisory-fresh");
        install_skill(&InstallRequest {
            root: root.clone(),
            agents: Vec::new(),
            shared: true,
            force: false,
        })
        .expect("install should succeed");

        assert!(skill_advisories(&root).is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_shared_install_warns_once_when_it_goes_stale() {
        let root = temp_root("advisory-shared-stale");
        install_skill(&InstallRequest {
            root: root.clone(),
            agents: Vec::new(),
            shared: true,
            force: false,
        })
        .expect("install should succeed");
        write_receipt(&root, CANONICAL_SKILL_DIR, &stale_receipt());

        let advisories = skill_advisories(&root);

        assert_eq!(advisories.len(), 1);
        assert_eq!(advisories[0].rule, STALE_RULE);
        assert!(advisories[0].message.contains("installed from klint 0.1.0"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn separate_copies_warn_once_each() {
        let root = temp_root("advisory-copies");
        for dir in unique_agent_dirs() {
            write_receipt(&root, &dir, &stale_receipt());
        }

        assert_eq!(skill_advisories(&root).len(), 3);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_legacy_node_modules_link_is_reported() {
        let root = temp_root("advisory-legacy");
        let dest = root.join(".claude/skills").join(SKILL_DIR_NAME);
        fs::create_dir_all(dest.parent().expect("parent exists")).expect("create parent");
        install::create_symlink(
            "../../node_modules/@konvert7/klint/skill/klint-rules",
            &dest,
        )
        .expect("create legacy link");

        let advisories = skill_advisories(&root);

        assert_eq!(advisories.len(), 1);
        assert_eq!(advisories[0].rule, LEGACY_RULE);
        assert_eq!(advisories[0].severity, "warn");
        assert_eq!(advisories[0].file, ".claude/skills/klint-rules");
        assert!(advisories[0].message.contains("node_modules"));
        assert!(advisories[0].message.contains("klint install-skill"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_skill_without_a_receipt_is_left_alone() {
        let root = temp_root("advisory-receiptless");
        let skill_dir = root.join(".claude/skills").join(SKILL_DIR_NAME);
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        fs::write(skill_dir.join(SKILL_FILE_NAME), "hand written").expect("write skill");

        assert!(skill_advisories(&root).is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn an_unreadable_receipt_is_left_alone() {
        let root = temp_root("advisory-corrupt");
        let skill_dir = root.join(".claude/skills").join(SKILL_DIR_NAME);
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        fs::write(skill_dir.join(RECEIPT_FILE_NAME), "{ not json").expect("write receipt");

        assert!(skill_advisories(&root).is_empty());
        let _ = fs::remove_dir_all(root);
    }
}
