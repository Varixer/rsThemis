use std::{ffi::OsStr, process::Command};

use log::info;

/// Usage: `cargo new --vcs none --edtion 2018 harness`
pub(crate) fn generate_harness<P>(path: P)
where
    P: AsRef<OsStr>,
{
    let output = Command::new("cargo")
        .arg("new")
        .args(["--vcs", "none", "--edition", "2018"])
        .arg(path)
        .output()
        .expect("cargo command failed to start");
    if output.status.success() {
        info!("{}", String::from_utf8_lossy(&output.stdout));
    }
}
