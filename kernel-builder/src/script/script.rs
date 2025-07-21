use anyhow::{Result, bail};
use std::env;
use std::sync::Arc;
use tokio::process::Command;
use crate::parse::report::CrashReport;

pub async fn mount(report: &Arc<CrashReport>) -> Result<()> {
    let id = report.id.clone();
    let commit = report.crashes.first().unwrap().kernel_source_commit.clone();

    let script_path = env::current_dir()?.join("script");

    let status = Command::new("./mount.sh")
        .arg(id)
        .arg(commit)
        .current_dir(script_path)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .await?; // 注意这里是 .await

    if !status.success() {
        bail!("failed to mount debian.img");
    }

    Ok(())
}

pub async fn get_vmcore(report: &Arc<CrashReport>) -> Result<()> {
    let id = report.id.clone();
    let commit = report.crashes.first().unwrap().kernel_source_commit.clone();

    let script_path = env::current_dir()?.join("script");

    let status = Command::new("./get.sh")
        .arg(id)
        .arg(commit)
        .current_dir(script_path)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .await?;

    if !status.success() {
        bail!("failed to get vmcore");
    }

    Ok(())
}
