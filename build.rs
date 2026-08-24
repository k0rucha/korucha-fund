use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Resolve a version string in priority order:
    //   1. BUILD_VERSION env var (CI override — e.g. set from the release tag)
    //   2. `git describe --tags --always --dirty` (tag preferred, SHA fallback)
    //   3. "dev"
    let version = env::var("BUILD_VERSION")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string())
        .or_else(git_describe)
        .unwrap_or_else(|| "dev".to_string());

    // Strip characters that could break HTML / Askama template parsing.
    // Git refs allow a restricted character set; this is belt-and-braces.
    let safe: String = version
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+'))
        .collect();
    let safe = if safe.is_empty() {
        "dev".to_string()
    } else {
        safe
    };

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_path = manifest_dir.join("templates/partials/_version.html");

    // Only rewrite when the value changes, so we don't churn template mtimes
    // and force unnecessary recompiles.
    let current = fs::read_to_string(&out_path).ok();
    if current.as_deref() != Some(safe.as_str()) {
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).expect("create partials dir");
        }
        fs::write(&out_path, &safe).expect("write version partial");
    }

    // Re-run when git state that affects `git describe` changes.
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/tags");
    println!("cargo:rerun-if-changed=.git/packed-refs");
    println!("cargo:rerun-if-env-changed=BUILD_VERSION");
    println!("cargo:rerun-if-changed=migrations");
}

fn git_describe() -> Option<String> {
    let output = Command::new("git")
        .args(["describe", "--tags", "--always", "--dirty"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}
