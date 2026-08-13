use crate::output::Violation;
use crate::version::reported_version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

const SKILL_MARKDOWN: &str = include_str!("../../../skill/klint-rules/SKILL.md");
const SKILL_DIR_NAME: &str = "klint-rules";
const SKILL_FILE_NAME: &str = "SKILL.md";
const RECEIPT_FILE_NAME: &str = ".klint-skill.json";
const STALE_RULE: &str = "klint/skill-stale";

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

#[derive(Debug)]
pub struct InstallRequest {
    pub root: PathBuf,
    pub agents: Vec<String>,
}

pub fn shipped_skill_hash() -> String {
    let mut hasher = Sha256::new();
    hasher.update(SKILL_MARKDOWN.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn install_skill(request: &InstallRequest) -> Result<Vec<String>, String> {
    let mut installed = Vec::new();
    for dir in agent_dirs_for(&request.agents)? {
        let destination = request.root.join(&dir).join(SKILL_DIR_NAME);
        replace_directory(&destination)?;
        write_skill_files(&destination)?;
        installed.push(format!("{dir}/{SKILL_DIR_NAME}"));
    }
    Ok(installed)
}

pub(crate) fn stale_skill_advisories(root: &Path) -> Vec<Violation> {
    let shipped = shipped_skill_hash();
    let installed = reported_version();
    unique_agent_dirs()
        .into_iter()
        .filter_map(|dir| stale_advisory_for(root, &dir, &shipped, &installed))
        .collect()
}

fn stale_advisory_for(root: &Path, dir: &str, shipped: &str, installed: &str) -> Option<Violation> {
    let skill_dir = root.join(dir).join(SKILL_DIR_NAME);
    let receipt = read_receipt(&skill_dir.join(RECEIPT_FILE_NAME))?;
    if receipt.sha256 == shipped {
        return None;
    }

    Some(Violation {
        file: format!("{dir}/{SKILL_DIR_NAME}/{SKILL_FILE_NAME}"),
        line: 1,
        rule: STALE_RULE.to_string(),
        message: format!(
            "this klint-rules skill was installed from klint {} and no longer matches klint {installed} — reinstall it with: klint install-skill",
            receipt.version
        ),
        severity: "warn".to_string(),
        fix: None,
    })
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

fn replace_directory(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => {
            fs::remove_dir_all(path).map_err(|err| failure("remove", path, &err))?;
        }
        Ok(_) => fs::remove_file(path).map_err(|err| failure("remove", path, &err))?,
        Err(_) => {}
    }
    fs::create_dir_all(path).map_err(|err| failure("create", path, &err))
}

fn write_skill_files(destination: &Path) -> Result<(), String> {
    let skill_path = destination.join(SKILL_FILE_NAME);
    fs::write(&skill_path, SKILL_MARKDOWN).map_err(|err| failure("write", &skill_path, &err))?;

    let receipt = SkillReceipt {
        version: reported_version(),
        sha256: shipped_skill_hash(),
    };
    let body = serde_json::to_string_pretty(&receipt)
        .map_err(|err| format!("klint: failed to serialize the skill receipt: {err}"))?;
    let receipt_path = destination.join(RECEIPT_FILE_NAME);
    fs::write(&receipt_path, format!("{body}\n"))
        .map_err(|err| failure("write", &receipt_path, &err))
}

fn failure(action: &str, path: &Path, err: &std::io::Error) -> String {
    format!("klint: failed to {action} {}: {err}", path.display())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(name: &str) -> PathBuf {
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
        assert_eq!(dirs, vec![".agents/skills".to_string()]);
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
    fn install_writes_the_skill_and_a_matching_receipt() {
        let root = temp_root("install");
        let installed = install_skill(&InstallRequest {
            root: root.clone(),
            agents: vec!["claude".to_string()],
        })
        .expect("install should succeed");

        assert_eq!(installed, vec![".claude/skills/klint-rules".to_string()]);
        let skill_dir = root.join(".claude/skills/klint-rules");
        assert_eq!(
            fs::read_to_string(skill_dir.join(SKILL_FILE_NAME)).expect("skill should exist"),
            SKILL_MARKDOWN
        );
        let receipt =
            read_receipt(&skill_dir.join(RECEIPT_FILE_NAME)).expect("receipt should exist");
        assert_eq!(receipt.sha256, shipped_skill_hash());
        assert_eq!(receipt.version, reported_version());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn install_replaces_a_previous_installation() {
        let root = temp_root("replace");
        let skill_dir = root.join(".claude/skills").join(SKILL_DIR_NAME);
        fs::create_dir_all(&skill_dir).expect("create stale dir");
        fs::write(skill_dir.join("leftover.md"), "old").expect("write leftover");

        install_skill(&InstallRequest {
            root: root.clone(),
            agents: vec!["claude".to_string()],
        })
        .expect("install should succeed");

        assert!(!skill_dir.join("leftover.md").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_fresh_install_produces_no_advisory() {
        let root = temp_root("fresh");
        install_skill(&InstallRequest {
            root: root.clone(),
            agents: Vec::new(),
        })
        .expect("install should succeed");

        assert!(stale_skill_advisories(&root).is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_stale_receipt_warns_once_per_directory() {
        let root = temp_root("stale");
        let receipt = SkillReceipt {
            version: "0.1.0".to_string(),
            sha256: "0".repeat(64),
        };
        write_receipt(&root, ".claude/skills", &receipt);
        write_receipt(&root, ".agents/skills", &receipt);

        let advisories = stale_skill_advisories(&root);

        assert_eq!(advisories.len(), 2);
        assert_eq!(advisories[0].rule, STALE_RULE);
        assert_eq!(advisories[0].severity, "warn");
        assert_eq!(advisories[0].file, ".claude/skills/klint-rules/SKILL.md");
        assert_eq!(advisories[0].line, 1);
        assert!(advisories[0].message.contains("installed from klint 0.1.0"));
        assert!(advisories[0].message.contains("klint install-skill"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_skill_without_a_receipt_is_left_alone() {
        let root = temp_root("receiptless");
        let skill_dir = root.join(".claude/skills").join(SKILL_DIR_NAME);
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        fs::write(skill_dir.join(SKILL_FILE_NAME), "hand written").expect("write skill");

        assert!(stale_skill_advisories(&root).is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn an_unreadable_receipt_is_left_alone() {
        let root = temp_root("corrupt");
        let skill_dir = root.join(".claude/skills").join(SKILL_DIR_NAME);
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        fs::write(skill_dir.join(RECEIPT_FILE_NAME), "{ not json").expect("write receipt");

        assert!(stale_skill_advisories(&root).is_empty());
        let _ = fs::remove_dir_all(root);
    }
}
