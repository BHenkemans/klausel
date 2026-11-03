use std::path::PathBuf;
use std::process::Command;

fn main() {
    if let Some(version) = git(&["describe", "--tags"]) {
        println!("cargo:rustc-env=KLAUSEL_VERSION={version}");
    }

    let Some(git_dir) = git(&["rev-parse", "--absolute-git-dir"]) else {
        return;
    };
    for watched in ["logs/HEAD", "refs/tags", "packed-refs"] {
        let path = PathBuf::from(&git_dir).join(watched);
        if path.exists() {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
}

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
