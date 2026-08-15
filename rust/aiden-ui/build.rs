use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=AIDEN_BUILD_VERSION");
    println!("cargo:rerun-if-changed=../../.git/HEAD");

    if std::env::var("AIDEN_BUILD_VERSION")
        .ok()
        .is_some_and(|version| !version.trim().is_empty())
    {
        return;
    }

    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let version = Command::new("git")
        .args([
            "-C",
            &manifest,
            "describe",
            "--tags",
            "--match",
            "v[0-9]*",
            "--abbrev=0",
        ])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|version| version.trim().trim_start_matches('v').to_string())
        .filter(|version| !version.is_empty());

    if let Some(version) = version {
        println!("cargo:rustc-env=AIDEN_BUILD_VERSION={version}");
    }
}
