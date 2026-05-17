use std::env;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=TESSERA_GIT_SHA");
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/refs");

    let git_sha = env::var("TESSERA_GIT_SHA")
        .ok()
        .filter(|value| is_full_git_sha(value))
        .or_else(read_git_sha)
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=TESSERA_GIT_SHA={git_sha}");
}

fn read_git_sha() -> Option<String> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").ok()?;
    let output = Command::new("git")
        .args(["-C", &manifest_dir, "rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let sha = String::from_utf8(output.stdout).ok()?.trim().to_string();
    is_full_git_sha(&sha).then_some(sha)
}

fn is_full_git_sha(value: &str) -> bool {
    value.len() == 40 && value.chars().all(|character| character.is_ascii_hexdigit())
}
