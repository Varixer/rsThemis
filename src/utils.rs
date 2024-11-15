use std::{ffi::OsStr, fs, os::unix::fs::PermissionsExt as _, path::Path, process::{Command, Output}};

use log::info;

use crate::Res;


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

/// Check if `path` points to an executable file
pub(crate) fn is_executable<P>(path: P)
where
    P: AsRef<Path> + std::fmt::Debug,
{
    let metadata = fs::metadata(&path).unwrap_or_else(|err| {
        panic!("Failed to access {:?}: {}", path, err);
    });

    if !metadata.is_file() {
        panic!("{:?} is not a file", path);
    }

    let permissions = metadata.permissions();
    if permissions.mode() & 0o111 == 0 {
        // 检查是否具有任何执行权限
        panic!("Tool({:?}) does not have execution permissions", path)
    }
}

/// Logical implementation of test case evaluation
pub(crate) fn evaluate([pos, neg]: [Output; 2]) -> Res {
    if pos.status.success() && neg.status.success() {
        match (pos.stdout.is_empty(), neg.stdout.is_empty()) {
            (false, true) => Res::Pass, // 通过
            (false, false) => Res::FP, // 误报
            _ => Res::FN, // 漏报
        }
    } else {
        Res::Err
    }
}
