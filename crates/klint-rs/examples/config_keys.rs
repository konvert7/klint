fn main() {
    println!(
        "{}",
        serde_json::to_string(&klint_rs::config_key_paths()).expect("key paths serialize")
    );
}
