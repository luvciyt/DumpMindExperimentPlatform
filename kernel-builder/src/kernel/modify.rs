use crate::parse::compiler::select_compiler;
use crate::parse::parse::{build_path, kernel_source_path};
use crate::parse::report::CrashReport;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::info;

async fn load_kernel_config() -> Result<HashMap<String, String>> {
    let mut kernel_config_path = PathBuf::from(env::current_dir()?);
    kernel_config_path.push("config");
    kernel_config_path.push("kernel.toml");

    info!(
        "Loading kernel configuration from: {}",
        kernel_config_path.display()
    );

    let kernel_config_content = tokio::fs::read_to_string(&kernel_config_path)
        .await
        .with_context(|| {
            format!(
                "Failed to read kernel configuration file from {}",
                kernel_config_path.display()
            )
        })?;

    info!("Kernel configuration loaded successfully");

    let config: HashMap<String, String> =
        toml::from_str(&kernel_config_content).with_context(|| {
            format!(
                "Failed to parse kernel configuration from {}",
                kernel_config_path.display()
            )
        })?;

    Ok(config)
}

pub async fn check_fix_config(report: &Arc<CrashReport>) -> Result<()> {
    let root_dir = build_path(report);
    let kernel_source_dir = kernel_source_path(report);

    let config_path = root_dir.join("build").join(".config");
    let shell_script_path = env::current_dir()?.join("nix").join("shell.nix");

    let kernel_config = load_kernel_config().await?; // configuration to be modified

    let file = File::open(&config_path)
        .await
        .with_context(|| format!("Failed to open config file at {}", config_path.display()))?;
    let reader = BufReader::new(file);
    let mut lines_stream = reader.lines();

    let mut lines = Vec::new();
    let mut config = HashMap::new();

    while let Some(line) = lines_stream.next_line().await? {
        let trimmed_line = line.trim();
        lines.push(trimmed_line.to_string());

        if let Some(key) = trimmed_line
            .strip_prefix("# CONFIG_")
            .and_then(|s| s.strip_suffix(" is not set"))
        {
            let full = format!("CONFIG_{}", key.trim());
            config.insert(full, "n".to_string());
            continue;
        }

        if trimmed_line.starts_with('#') || !trimmed_line.contains('=') {
            continue;
        }

        if let Some((key, value)) = trimmed_line.split_once('=') {
            let key = key.trim().to_string();
            let value = value.trim().to_string();
            config.insert(key, value);
        }
    }

    info!("Checking and modifying kernel config...");

    let mut update = false;
    let mut original = lines.clone();
    let mut found_keys = std::collections::HashSet::new();

    for (i, line) in lines.iter().enumerate() {
        for (key, expected) in &kernel_config {
            if line.starts_with(&format!("{}=", key)) || *line == format!("# {} is not set", key) {
                found_keys.insert(key.clone());
                let actual_value = config.get(key).map_or("n", |v| v.as_str());
                if actual_value != expected {
                    println!(
                        "[✘] error config: {} (expected: {}, actually: {})",
                        key, expected, actual_value
                    );

                    if expected == "n" {
                        original[i] = format!("# {} is not set", key);
                    } else {
                        original[i] = format!("{}={}", key, expected);
                    }
                    update = true;
                } else {
                    println!("[✔] {}={}", key, expected);
                }
            }
        }
    }

    for (key, expected) in kernel_config {
        if !found_keys.contains(&key) {
            println!("[✘] lack config: {} (expected: {})", key, expected);

            if expected == "n" {
                original.push(format!("# {} is not set", key));
            } else {
                original.push(format!("{}={}", key, expected));
            }
            update = true;
        }
    }

    if update {
        info!("updating config file");

        let content = original.join("\n") + "\n";
        fs::write(&config_path, content).await?;

        info!("config file updated successfully. running \"make O=../build olddefconfig\"");

        let make_cmd = "make O=../build olddefconfig";

        let compiler = select_compiler(&report)?;
        let compiler_str = format!("{}-{}", compiler.compiler_type.to_string(), compiler.major);

        let status = Command::new("nix-shell")
            .arg(shell_script_path)
            .arg("--pure")
            .arg("--argstr")
            .arg("compiler")
            .arg(compiler_str)
            .arg("--run")
            .arg(make_cmd)
            .current_dir(kernel_source_dir)
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .await?;

        if !status.success() {
            anyhow::bail!(
                "error running make old defconfig, exit code: {:?}",
                status.code()
            );
        }
    } else {
        println!("all needed config are satisfied");
    }

    Ok(())
}
