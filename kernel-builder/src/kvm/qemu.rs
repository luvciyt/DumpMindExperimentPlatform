use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum QEMUError {
    #[error("VM startup failed: {0}")]
    VMStartupFailed(String),
    #[error("Monitor connection failed: {0}")]
    MonitorConnectionFailed(String),
    #[error("Monitor command execution failed: {0}")]
    MonitorCommandExecutionFailed(String),
    #[error("SSH connection failed: {0}")]
    SSHConnectionFailed(String),
    #[error("Process error: {0}")]
    ProcessError(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("VM not running")]
    VMNotRunning,
    #[error("Monitor not connected")]
    MonitorNotConnected,
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Timeout error: {0}")]
    TimeoutError(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] toml::de::Error),
    #[error("File not found: {0}")]
    FileNotFound(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VMConfig {
    pub name: String,
    pub image_path: String,
    pub kernel_path: Option<String>,
    pub memory: String,
    pub monitor_port: u16,
    pub ssh_port: u16,
    pub kernel_append: Option<String>,
    pub log_file: Option<String>,
    
}
