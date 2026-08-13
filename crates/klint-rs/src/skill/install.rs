use super::{
    CANONICAL_SKILL_DIR, RECEIPT_FILE_NAME, SKILL_DIR_NAME, SKILL_FILE_NAME, SKILL_MARKDOWN,
    SkillReceipt, agent_dirs_for, shipped_skill_hash,
};
use crate::version::reported_version;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct InstallRequest {
    pub root: PathBuf,
    pub agents: Vec<String>,
    pub shared: bool,
    pub force: bool,
}

pub fn install_skill(request: &InstallRequest) -> Result<Vec<String>, String> {
    let dirs = agent_dirs_for(&request.agents)?;
    if request.shared {
        install_shared(request, &dirs)
    } else {
        install_copies(request, &dirs)
    }
}

fn install_shared(request: &InstallRequest, dirs: &[String]) -> Result<Vec<String>, String> {
    let hub = format!("{CANONICAL_SKILL_DIR}/{SKILL_DIR_NAME}");
    let mut report = vec![install_copy(request, &hub)?];

    for dir in dirs {
        if dir == CANONICAL_SKILL_DIR {
            continue;
        }
        report.push(link_to_hub(
            request,
            &format!("{dir}/{SKILL_DIR_NAME}"),
            &hub,
        )?);
    }
    Ok(report)
}

fn install_copies(request: &InstallRequest, dirs: &[String]) -> Result<Vec<String>, String> {
    dirs.iter()
        .map(|dir| install_copy(request, &format!("{dir}/{SKILL_DIR_NAME}")))
        .collect()
}

fn install_copy(request: &InstallRequest, target: &str) -> Result<String, String> {
    let destination = request.root.join(target);
    prepare_destination(&destination, target, request.force)?;
    write_skill_files(&destination)?;
    Ok(format!("installed {target}"))
}

fn link_to_hub(request: &InstallRequest, target: &str, hub: &str) -> Result<String, String> {
    let destination = request.root.join(target);
    prepare_destination(&destination, target, request.force)?;

    let link_path = relative_to_parent(target, hub);
    match create_symlink(&link_path, &destination) {
        Ok(()) => Ok(format!("linked {target} -> {link_path}")),
        Err(_) => {
            write_skill_files(&destination)?;
            Ok(format!("installed {target} (symlinks unavailable here)"))
        }
    }
}

fn relative_to_parent(target: &str, hub: &str) -> String {
    let depth = target.matches('/').count();
    format!("{}{hub}", "../".repeat(depth))
}

#[cfg(unix)]
pub(super) fn create_symlink(link_path: &str, destination: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(link_path, destination)
}

#[cfg(windows)]
pub(super) fn create_symlink(link_path: &str, destination: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(link_path, destination)
}

fn prepare_destination(destination: &Path, target: &str, force: bool) -> Result<(), String> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|err| failure("create", parent, &err))?;
    }

    let Ok(metadata) = fs::symlink_metadata(destination) else {
        return Ok(());
    };

    if !metadata.is_dir() {
        fs::remove_file(destination).map_err(|err| failure("remove", destination, &err))
    } else if force || destination.join(RECEIPT_FILE_NAME).exists() {
        fs::remove_dir_all(destination).map_err(|err| failure("remove", destination, &err))
    } else {
        Err(format!(
            "klint: {target} exists and was not installed by klint — pass --force to replace it"
        ))
    }
}

fn write_skill_files(destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|err| failure("create", destination, &err))?;

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
    use crate::skill::tests::temp_root;

    fn request(root: &Path, shared: bool, force: bool) -> InstallRequest {
        InstallRequest {
            root: root.to_path_buf(),
            agents: Vec::new(),
            shared,
            force,
        }
    }

    #[test]
    fn shared_mode_keeps_one_real_skill_and_links_the_rest() {
        let root = temp_root("shared");
        let report = install_skill(&request(&root, true, false)).expect("install should succeed");

        assert_eq!(
            report,
            vec![
                "installed .agents/skills/klint-rules".to_string(),
                "linked .claude/skills/klint-rules -> ../../.agents/skills/klint-rules".to_string(),
                "linked .cursor/skills/klint-rules -> ../../.agents/skills/klint-rules".to_string(),
            ]
        );
        assert!(
            fs::symlink_metadata(root.join(".agents/skills/klint-rules"))
                .expect("hub should exist")
                .is_dir()
        );
        for linked in [".claude/skills/klint-rules", ".cursor/skills/klint-rules"] {
            assert!(
                fs::symlink_metadata(root.join(linked))
                    .expect("link should exist")
                    .is_symlink()
            );
            assert_eq!(
                fs::read_to_string(root.join(linked).join(SKILL_FILE_NAME))
                    .expect("link should resolve"),
                SKILL_MARKDOWN
            );
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn shared_mode_writes_a_single_receipt() {
        let root = temp_root("shared-receipt");
        install_skill(&request(&root, true, false)).expect("install should succeed");

        let receipts = [".agents", ".claude", ".cursor"]
            .into_iter()
            .filter(|dir| {
                fs::symlink_metadata(root.join(dir).join("skills/klint-rules"))
                    .is_ok_and(|metadata| metadata.is_dir())
            })
            .count();

        assert_eq!(receipts, 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn copy_mode_writes_an_independent_skill_per_directory() {
        let root = temp_root("copies");
        let report = install_skill(&request(&root, false, false)).expect("install should succeed");

        assert_eq!(report.len(), 3);
        for dir in [".agents", ".claude", ".cursor"] {
            let skill_dir = root.join(dir).join("skills/klint-rules");
            assert!(
                fs::symlink_metadata(&skill_dir)
                    .expect("skill should exist")
                    .is_dir()
            );
            assert!(skill_dir.join(RECEIPT_FILE_NAME).exists());
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_legacy_symlink_is_replaced_without_force() {
        let root = temp_root("legacy");
        let dest = root.join(".claude/skills/klint-rules");
        fs::create_dir_all(dest.parent().expect("parent exists")).expect("create parent");
        create_symlink(
            "../../node_modules/@konvert7/klint/skill/klint-rules",
            &dest,
        )
        .expect("create legacy link");

        install_skill(&request(&root, true, false)).expect("install should succeed");

        assert_eq!(
            fs::read_link(&dest).expect("link should exist"),
            PathBuf::from("../../.agents/skills/klint-rules")
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_dangling_legacy_symlink_is_replaced() {
        let root = temp_root("dangling");
        let dest = root.join(".claude/skills/klint-rules");
        fs::create_dir_all(dest.parent().expect("parent exists")).expect("create parent");
        create_symlink("../../node_modules/gone/klint-rules", &dest).expect("create dangling link");

        install_skill(&request(&root, true, false)).expect("install should succeed");

        assert_eq!(
            fs::read_link(&dest).expect("link should exist"),
            PathBuf::from("../../.agents/skills/klint-rules")
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn an_unmanaged_skill_directory_is_refused_without_force() {
        let root = temp_root("unmanaged");
        let dest = root.join(".agents/skills/klint-rules");
        fs::create_dir_all(&dest).expect("create skill dir");
        fs::write(dest.join(SKILL_FILE_NAME), "hand written").expect("write skill");

        let error = install_skill(&request(&root, true, false))
            .expect_err("an unmanaged skill should be refused");

        assert!(error.contains("was not installed by klint"));
        assert!(error.contains("--force"));
        assert_eq!(
            fs::read_to_string(dest.join(SKILL_FILE_NAME)).expect("skill should survive"),
            "hand written"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn force_replaces_an_unmanaged_skill_directory() {
        let root = temp_root("forced");
        let dest = root.join(".agents/skills/klint-rules");
        fs::create_dir_all(&dest).expect("create skill dir");
        fs::write(dest.join(SKILL_FILE_NAME), "hand written").expect("write skill");

        install_skill(&request(&root, true, true)).expect("force should replace the skill");

        assert_eq!(
            fs::read_to_string(dest.join(SKILL_FILE_NAME)).expect("skill should exist"),
            SKILL_MARKDOWN
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn no_install_path_points_into_node_modules() {
        let root = temp_root("no-node-modules");
        install_skill(&request(&root, true, false)).expect("install should succeed");

        for linked in [".claude/skills/klint-rules", ".cursor/skills/klint-rules"] {
            let target = fs::read_link(root.join(linked)).expect("link should exist");
            assert!(!target.to_string_lossy().contains("node_modules"));
        }
        let _ = fs::remove_dir_all(root);
    }
}
