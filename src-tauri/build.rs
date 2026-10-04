use std::hash::{DefaultHasher, Hasher};
use std::path::{Path, PathBuf};

/// Names the bundled files and fingerprints them, so Android unpacks them again only when they change.
fn bundled_list() {
    let mut names = Vec::new();
    let mut hasher = DefaultHasher::new();
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        let modules = Path::new("../modules");
        println!("cargo:rerun-if-changed=../modules");
        println!("cargo:rerun-if-changed=../catalog/videos.grt");
        let mut found: Vec<String> = std::fs::read_dir(modules)
            .expect("the modules folder")
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.ends_with(".grtb"))
            .map(|n| format!("modules/{n}"))
            .collect();
        found.sort();
        found.push("catalog/videos.grt".into());
        for name in &found {
            hasher.write(name.as_bytes());
            hasher.write(&std::fs::read(Path::new("..").join(name)).expect("a bundled file"));
        }
        names = found;
    }
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("bundled.rs");
    let code = format!("pub const FILES: &[&str] = &{names:?};\npub const STAMP: &str = \"{:016x}\";\n", hasher.finish());
    std::fs::write(out, code).expect("the bundled file list");
}

fn main() {
    bundled_list();
    tauri_build::build()
}
