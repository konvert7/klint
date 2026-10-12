use std::env;
use std::path::PathBuf;

const WORKSPACE_SKILL: &str = "../../skill/klint-rules/SKILL.md";
const PACKAGED_SKILL: &str = "skill/SKILL.md";

fn main() {
    println!("cargo:rerun-if-env-changed=KLINT_VERSION");
    let skill = bundled_skill();
    println!("cargo:rerun-if-changed={}", skill.display());
    println!("cargo:rustc-env=KLINT_SKILL_MARKDOWN={}", skill.display());
}

fn bundled_skill() -> PathBuf {
    let crate_root =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"));
    let workspace_skill = crate_root.join(WORKSPACE_SKILL);
    if workspace_skill.exists() {
        workspace_skill
    } else {
        crate_root.join(PACKAGED_SKILL)
    }
}
