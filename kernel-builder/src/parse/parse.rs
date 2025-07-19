use crate::parse::report::CrashReport;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::{env, fs};
use tracing::info;

pub fn build_path(report: &CrashReport) -> PathBuf {
    let root = env::current_dir().unwrap();
    let id = report.id.clone();
    let suffix = format!("workspace/{}", id);
    let path = root.join(&suffix);

    PathBuf::from(path)
}

pub fn kernel_source_path(report: &CrashReport) -> PathBuf {
    let root = build_path(report);
    let commit = report.crashes.first().unwrap().kernel_source_commit.clone();
    let suffix = format!("linux-{}", commit);
    let path = root.join(suffix);

    PathBuf::from(path)
}

pub fn parse_file(filepath: &str) -> Result<CrashReport> {
    let json_content = fs::read_to_string(filepath)
        .with_context(|| format!("Failed to read json file {:?}", &filepath))?;

    let report: CrashReport = serde_json::from_str(&json_content)
        .with_context(|| format!("Failed to parse json file {:?}", &filepath))?;

    info!("Parsing crash report from file {} successfully", filepath);

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_path() {
        let crash_report =
            parse_file("datasets/0b6b2d6d6cefa8b462930e55be699efba635788f.json").unwrap();
        let path = build_path(&crash_report).to_string_lossy().into_owned();
        assert_eq!(path, "/home/luvciyt/Repo/DumpMindExperimentPlatform/kernel-builder/workspace/0b6b2d6d6cefa8b462930e55be699efba635788f".to_string())
    }

    #[test]
    fn test_kernel_source_path() {
        let crash_report =
            parse_file("datasets/0b6b2d6d6cefa8b462930e55be699efba635788f.json").unwrap();
        let path = kernel_source_path(&crash_report)
            .to_string_lossy()
            .into_owned();
        assert_eq!(path, "/home/luvciyt/Repo/DumpMindExperimentPlatform/kernel-builder/workspace/0b6b2d6d6cefa8b462930e55be699efba635788f/linux-02d5e016800d082058b3d3b7c3ede136cdc6ddcb".to_string())
    }
}
