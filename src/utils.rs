use std::{
    ffi::OsStr,
    fs::{self, File},
    io::{self, Write},
    os::unix::fs::PermissionsExt as _,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use log::info;

use crate::{EvalResult, EvalSummary, Program};

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
pub(crate) fn evaluate((pos, neg): (Output, Output)) -> EvalResult {
    if pos.status.success() && neg.status.success() {
        match (pos.stdout.is_empty(), neg.stdout.is_empty()) {
            (false, true) => EvalResult::Pass, // 通过
            (false, false) => EvalResult::FP,  // 误报
            _ => EvalResult::FN,               // 漏报
        }
    } else {
        EvalResult::Err
    }
}

pub(crate) fn write(path: PathBuf, (pos, neg): (&Program, &Program)) {
    std::fs::create_dir_all(&path).expect(" std::fs::create_dir_all failed");
    std::fs::write(path.join("POS.rs"), pos.merge()).expect("std::fs::write failed");
    std::fs::write(path.join("NEG.rs"), neg.merge()).expect("std::fs::write failed");
}

/// 保存 DOT 内容并生成图片
///
/// # Arguments
/// * `dot_content` - DOT 文件的内容
/// * `output_path` - 图片的输出路径
///
/// # Returns
/// 如果成功生成图片，返回 `Ok(())`；否则返回错误信息。
pub(crate) fn generate_image_from_dot(
    dot_content: &str,
    output_path: PathBuf,
) -> Result<(), String> {
    // 创建临时 DOT 文件路径（同目录下，带 `.dot` 扩展名）
    let mut dot_path = output_path.clone();
    dot_path.set_extension("dot");

    // 保存 DOT 内容到文件
    if let Err(e) = fs::write(&dot_path, dot_content) {
        return Err(format!("Failed to write DOT file: {}", e));
    }

    // 确定输出图片格式（根据文件扩展名）
    let format = match output_path.extension() {
        Some(ext) => ext.to_string_lossy().to_string(),
        None => return Err("Output path must have a valid extension".to_string()),
    };

    // 调用 Graphviz 生成图片
    let output = Command::new("dot")
        .args(&["-T", &format, "-o"])
        .arg(&output_path)
        .arg(&dot_path)
        .output();

    // 检查执行结果
    match output {
        Ok(result) => {
            if result.status.success() {
                // 清理临时文件
                let _ = fs::remove_file(dot_path);
                Ok(())
            } else {
                Err(format!(
                    "Graphviz error: {}",
                    String::from_utf8_lossy(&result.stderr)
                ))
            }
        }
        Err(e) => Err(format!("Failed to execute Graphviz: {}", e)),
    }
}

pub(crate) fn serialize_to_csv(summaries: &[EvalSummary], output_file: PathBuf) -> io::Result<()> {
    // Open or create the output file
    let file = File::create(output_file)?;

    // Create a CSV writer
    let mut writer = csv::Writer::from_writer(file);

    // Serialize each `EvalSummary` to the CSV
    for summary in summaries {
        writer.serialize(summary)?;
    }

    // Ensure all data is written to the file
    writer.flush()?;

    Ok(())
}