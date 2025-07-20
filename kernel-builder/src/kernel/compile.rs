use crate::parse::compiler::{select_compiler, CompilerType};
use crate::parse::parse::{build_path, kernel_source_path};
use crate::parse::report::CrashReport;
use anyhow::{Context, Result};
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::try_exists;
use tokio::process::Command;
use tracing::info;

struct NixCommand {
    shell_script: PathBuf,
    compiler: String,
    working_dir: PathBuf,
}
impl NixCommand {
    fn new(shell_script: PathBuf, compiler: &str, working_dir: PathBuf) -> Self {
        Self {
            shell_script,
            compiler: compiler.to_string(),
            working_dir,
        }
    }

    async fn execute(&self, command: &str) -> Result<()> {
        let status = Command::new("nix-shell")
            .arg(&self.shell_script)
            .arg("--pure")
            .arg("--argstr")
            .arg("compiler")
            .arg(&self.compiler)
            .arg("--run")
            .arg(command)
            .current_dir(&self.working_dir)
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .await
            .context("Failed to execute nix-shell command")?;

        if !status.success() {
            anyhow::bail!(
                "Command failed with exit code: {:?}\nCommand: {}",
                status.code(),
                command
            );
        }

        Ok(())
    }
}
pub async fn make_kernel(report: &Arc<CrashReport>) -> Result<()> {
    let build_dir = build_path(report);
    let compiler = select_compiler(report)?;
    let kernel_source_dir = kernel_source_path(report);
    let shell_script_path = env::current_dir()?.join("nix").join("shell.nix");

    info!(
        "Starting kernel compilation with compiler: {}",
        format!(
            "{}-{}.{}.{}",
            compiler.compiler_type.to_string(),
            compiler.major,
            compiler.minor,
            compiler.patch
        )
    );

    let num_cpu = num_cpus::get();
    let make_cmd = match compiler.compiler_type {
        CompilerType::GCC => {
            format!("bear -- make O=../build -j{}", num_cpu - 2)
        }
        CompilerType::CLANG => {
            format!(
                "bear -- make O=../build LLVM=1 CC=clang LD=ld.lld AR=llvm-ar NM=llvm-nm OBJCOPY=llvm-objcopy -j{}",
                num_cpu - 2
            )
        }
    };

    let compiler_str = format!("{}-{}", compiler.compiler_type.to_string(), compiler.major);
    let nix_cmd = NixCommand::new(shell_script_path, &compiler_str, kernel_source_dir);

    nix_cmd
        .execute(&make_cmd)
        .await
        .context("Failed to execute nix-shell command")?;

    info!("compilation succeeded");

    let bz_image_path = build_dir.join("build").join("arch/x86_64/boot/bzImage");
    if !try_exists(&bz_image_path).await? {
        anyhow::bail!("bzImage not found in: {}", bz_image_path.display());
    }

    info!("start linux headers install");

    let header_install_cmd = "make O=../build headers_install INSTALL_HDR_PATH=../install";

    nix_cmd
        .execute(header_install_cmd)
        .await
        .context("Failed to execute header install command")?;

    Ok(())
}

pub async fn rebuild_kernel(report: &Arc<CrashReport>) -> Result<()> {
    info!("Rebuilding kernel...");

    let build_dir = build_path(report);
    let kernel_source_dir = kernel_source_path(report);

    let status = Command::new("make")
        .arg("-C")
        .arg(&kernel_source_dir)
        .arg("-j")
        .arg(num_cpus::get().to_string())
        .current_dir(&build_dir)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .await?;

    if !status.success() {
        anyhow::bail!(
            "error running kernel rebuild, exit code: {:?}",
            status.code()
        );
    }

    info!("Kernel rebuild succeeded");

    Ok(())
}
