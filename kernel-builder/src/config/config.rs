use crate::kvm::ssh::SSHError;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DurationSeconds;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{error, info};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub proxy: ProxyConfig,
    pub ssh: SSHConfig,
}

// proxy config
#[derive(Debug, Deserialize, Serialize)]
pub struct ProxyConfig {
    pub host: String,
    pub port: u16,
}

// ssh config
#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SSHConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub key_path: PathBuf,

    #[serde_as(as = "DurationSeconds<u64>")]
    pub timeout: Duration,

    pub max_retries: usize,

    #[serde_as(as = "DurationSeconds<u64>")]
    pub initial_backoff: Duration,

    #[serde_as(as = "DurationSeconds<u64>")]
    pub max_backoff: Duration,
    pub strict_host_key_checking: bool,
    pub compression: bool,
    #[serde_as(as = "Option<DurationSeconds<u64>>")]
    pub keep_alive_interval: Option<Duration>,
}

impl Default for Config {
    fn default() -> Self {
        load_config().unwrap_or_else(|e| {
            error!(
                "Failed to load config, using hardcoded default. Error: {:?}",
                e
            );
            Config {
                proxy: ProxyConfig {
                    host: "127.0.0.1".to_string(),
                    port: 7890,
                },
                ssh: SSHConfig {
                    host: "127.0.0.1".to_string(),
                    port: 22,
                    user: "root".to_string(),
                    key_path: PathBuf::from("~/.ssh/debian-key"),
                    timeout: Duration::from_secs(30),
                    max_retries: 5,
                    initial_backoff: Duration::from_secs(1),
                    max_backoff: Duration::from_secs(30),
                    compression: false,
                    strict_host_key_checking: false,
                    keep_alive_interval: Some(Duration::from_secs(60)),
                },
            }
        })
    }
}

impl SSHConfig {
    pub fn validate(&self) -> Result<(), SSHError> {
        if self.host.is_empty() {
            return Err(SSHError::ConnectionFailed(
                "Host cannot be empty".to_string(),
            ));
        }
        if self.max_retries == 0 {
            return Err(SSHError::ConnectionFailed(
                "Max retries must be greater than 0".to_string(),
            ));
        }
        Ok(())
    }
}

// default load config from config/settings.toml
fn load_config() -> Result<Config> {
    let mut config_file = PathBuf::from(std::env::current_dir()?);
    config_file.push("config");
    config_file.push("settings.toml");

    info!("Loading configuration from: {:?}", config_file);

    let config_content = fs::read_to_string(&config_file)
        .with_context(|| format!("Failed to read config file: {:?}", config_file))?;

    info!(
        "Loading configuration succeeded, File size: {} bytes",
        config_content.len()
    );

    let config: Config = toml::from_str(&config_content)
        .with_context(|| format!("Failed to parse config file: {:?}", config_file))?;

    info!("Loaded configuration succeeded");

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.proxy.host, "127.0.0.1");
        assert_eq!(config.proxy.port, 9870);
        assert_eq!(config.ssh.port, 22);
    }
}
