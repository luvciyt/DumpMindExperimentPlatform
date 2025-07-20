use crate::parse::parse::{build_path, kernel_source_path};
use crate::parse::report::CrashReport;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
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

        if trimmed_line.starts_with('#') || !trimmed_line.contains('='){
            continue;
        }

        if let Some((key, value)) = trimmed_line.split_once('=') {
            let key = key.trim().to_string();
            let value = value.trim().to_string();
            config.insert(key, value);
        }
    }

    info!("Loaded kernel config from file: {:?}", config);
    info!("Checking and modifying kernel config...");

    let mut update  = false;

    Ok(())
}
