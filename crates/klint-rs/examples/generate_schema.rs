use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let Some(version) = std::env::args().nth(1) else {
        eprintln!("usage: cargo run -p klint-rs --example generate_schema -- <version>");
        std::process::exit(2);
    };
    let schema = klint_rs::config_schema(&version);
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");

    write(
        repo_root.join("klint.schema.json"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&schema).expect("schema serializes to JSON")
        ),
    );
    write(
        repo_root.join("klint.schema.yaml"),
        serde_yaml::to_string(&schema).expect("schema serializes to YAML"),
    );
}

fn write(path: PathBuf, content: String) {
    fs::write(&path, content)
        .unwrap_or_else(|err| panic!("failed to write {}: {err}", path.display()));
    println!("Generated {}", path.display());
}
