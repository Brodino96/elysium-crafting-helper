use std::fs;

fn main() {
    // Try reading version from package.json (local dev).
    // Falls back to CARGO_PKG_VERSION (set in Cargo.toml) when package.json
    // is absent, e.g. when building from a crates.io source checkout.
    if let Ok(pkg) = fs::read_to_string("../package.json") {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&pkg) {
            if let Some(version) = json["version"].as_str() {
                println!("cargo:rustc-env=PKG_VERSION={}", version);
            }
        }
        println!("cargo:rerun-if-changed=../package.json");
    }
    tauri_build::build()
}
