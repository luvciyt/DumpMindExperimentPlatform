use crate::config::config::{Config, SSHConfig};
use openssh::KnownHosts::Strict;
use openssh::{KnownHosts, Session, SessionBuilder};
use rand::Rng;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::time::sleep;
use tracing::{debug, error, info};

#[derive(Error, Debug)]
pub enum SSHError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    #[error("Session failed: {0}")]
    SessionFailed(String),
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error("OpenSSH error: {0}")]
    OpenSSH(#[from] openssh::Error),
    #[error("Client not initialized")]
    ClientNotInitialized,
    #[error("SSH command execution failed: {0}")]
    CommandExecutionFailed(String),
    #[error("Host key verification failed")]
    HostKeyVerificationFailed,
    #[error("Timeout error: {0}")]
    TimeoutError(String),
    #[error("Unexpected EOF or connection closed")]
    UnexpectedEof,
}

pub struct SSHManager {
    config: SSHConfig,
    session: Option<Session>,
    connected_at: Option<Instant>,
}

impl SSHManager {
    pub fn new(config: SSHConfig) -> Result<Self, SSHError> {
        config.validate()?;
        Ok(SSHManager {
            config,
            session: None,
            connected_at: None,
        })
    }

    pub fn builder() -> SSHConfigBuilder {
        SSHConfigBuilder::default()
    }

    pub async fn connect(&mut self) -> Result<(), SSHError> {
        info!(
            "Connecting to SSH server at {}:{}",
            self.config.host, self.config.port
        );

        let mut rng = rand::rng();
        let mut backoff = self.config.initial_backoff;

        for attempt in 0..self.config.max_retries {
            match self.try_connect().await {
                Ok(()) => {
                    info!(
                        "Successfully connected to SSH server on attempt {}",
                        attempt + 1
                    );
                    self.connected_at = Some(Instant::now());
                    return Ok(());
                }
                Err(e) => {
                    error!("Connection attempt {} failed: {}", attempt + 1, e);

                    if attempt < self.config.max_retries - 1 {
                        let jitter = rng.random_range(0..backoff.as_millis() as u64);
                        let sleep_duration = backoff + Duration::from_millis(jitter);

                        info!(
                            "Retrying in {:?} (attempt {}/{})",
                            sleep_duration,
                            attempt + 2,
                            self.config.max_retries
                        );
                        sleep(sleep_duration).await;

                        backoff = std::cmp::min(backoff * 2, self.config.max_backoff);
                    } else {
                        return Err(SSHError::ConnectionFailed(format!(
                            "Failed to connect after {} attempts: {}",
                            self.config.max_retries, e,
                        )));
                    }
                }
            }
        }

        Err(SSHError::ConnectionFailed(
            "Max retries reached without a successful connection".to_string(),
        ))
    }

    async fn try_connect(&mut self) -> Result<(), SSHError> {
        let dest = format!("{}@{}", self.config.user, self.config.host);

        let mut builder = SessionBuilder::default();
        builder
            .connect_timeout(self.config.timeout)
            .server_alive_interval(
                self.config
                    .keep_alive_interval
                    .unwrap_or(Duration::from_secs(60)),
            );

        if self.config.compression {
            builder.compression(true);
        }

        if self.config.strict_host_key_checking {
            builder.known_hosts_check(KnownHosts::Strict);
        } else {
            builder.known_hosts_check(KnownHosts::Accept);
        }

        builder.port(self.config.port);

        builder.keyfile(std::fs::canonicalize(&self.config.key_path)?);

        let session = tokio::time::timeout(self.config.timeout, builder.connect(&dest))
            .await
            .map_err(|_| SSHError::TimeoutError("Connection timed out".to_string()))?
            .map_err(|e| {
                SSHError::ConnectionFailed(format!("Failed to connect to {}: {:#?}", dest, e))
            })?;

        self.session = Some(session);

        Ok(())
    }

    pub async fn execute(&self, cmd: &str) -> Result<String, SSHError> {
        let session = self
            .session
            .as_ref()
            .ok_or(SSHError::ClientNotInitialized)?;

        debug!("Executing command: {}", cmd);

        let output = tokio::time::timeout(
            self.config.timeout,
            session.command("bash").arg("-lc").arg(cmd).output(),
        )
        .await
        .map_err(|_| SSHError::TimeoutError("Command execution timed out".to_string()))?
        .map_err(|e| {
            SSHError::CommandExecutionFailed(format!("Failed to execute command: {:#?}", e))
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        info!("Command executed. Exit status: {:?}", output.status.code());
        debug!("Command output: {}", stdout);

        if !stderr.is_empty() {
            error!("Command error output: {}", stderr);
        }

        if !output.status.success() {
            return Err(SSHError::CommandExecutionFailed(format!(
                "Command failed with status: {:?}, stderr: {}",
                output.status, stderr
            )));
        }

        Ok(stdout)
    }

    pub async fn execute_batch(&self, commands: &[&str]) -> Result<Vec<String>, SSHError> {
        let mut results = Vec::new();

        for (i, cmd) in commands.iter().enumerate() {
            info!(
                "Executing batch command {}/{}: {}",
                i + 1,
                commands.len(),
                cmd
            );
            let result = self.execute(cmd).await?;
            results.push(result);
        }

        Ok(results)
    }

    pub async fn is_connected(&self) -> bool {
        if let Some(session) = &self.session {
            match tokio::time::timeout(
                Duration::from_secs(5),
                session.command("echo test").output(),
            )
            .await
            {
                Ok(Ok(output)) => output.status.success(),
                _ => false,
            }
        } else {
            false
        }
    }

    pub fn connection_info(&self) -> Option<ConnectionInfo> {
        self.session.as_ref().map(|_| ConnectionInfo {
            host: self.config.host.clone(),
            port: self.config.port,
            user: self.config.user.clone(),
            connected_at: self.connected_at.unwrap_or_else(Instant::now),
        })
    }
    pub async fn disconnect(&mut self) -> Result<(), SSHError> {
        if let Some(session) = self.session.take() {
            info!("Disconnecting SSH session");
            session
                .close()
                .await
                .map_err(|e| SSHError::SessionFailed(format!("Failed to close session: {}", e)))?;
        }
        self.connected_at = None;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub connected_at: Instant,
}

#[derive(Default)]
pub struct SSHConfigBuilder {
    host: Option<String>,
    port: Option<u16>,
    user: Option<String>,
    key_path: Option<PathBuf>,
    timeout: Option<Duration>,
    max_retries: Option<usize>,
    initial_backoff: Option<Duration>,
    max_backoff: Option<Duration>,
    compression: Option<bool>,
    strict_host_key_checking: Option<bool>,
    keep_alive_interval: Option<Duration>,
}

impl SSHConfigBuilder {
    pub fn host<S: Into<String>>(mut self, host: S) -> Self {
        self.host = Some(host.into());
        self
    }
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }
    pub fn user<S: Into<String>>(mut self, user: S) -> Self {
        self.user = Some(user.into());
        self
    }
    pub fn key_path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.key_path = Some(path.into());
        self
    }
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
    pub fn max_retries(mut self, max_retries: usize) -> Self {
        self.max_retries = Some(max_retries);
        self
    }
    pub fn backoff(mut self, initial: Duration, max: Duration) -> Self {
        self.initial_backoff = Some(initial);
        self.max_backoff = Some(max);
        self
    }
    pub fn compression(mut self, enable: bool) -> Self {
        self.compression = Some(enable);
        self
    }
    pub fn strict_host_key_checking(mut self, enable: bool) -> Self {
        self.strict_host_key_checking = Some(enable);
        self
    }
    pub fn keep_alive_interval(mut self, interval: Duration) -> Self {
        self.keep_alive_interval = Some(interval);
        self
    }
    pub fn build(self) -> Result<SSHConfig, SSHError> {
        let default = Config::default().ssh.clone();
        let config = SSHConfig {
            host: self.host.unwrap_or(default.host),
            port: self.port.unwrap_or(default.port),
            user: self.user.unwrap_or(default.user),
            key_path: self.key_path.unwrap_or(default.key_path),
            timeout: self.timeout.unwrap_or(default.timeout),
            max_retries: self.max_retries.unwrap_or(default.max_retries),
            initial_backoff: self.initial_backoff.unwrap_or(default.initial_backoff),
            max_backoff: self.max_backoff.unwrap_or(default.max_backoff),
            compression: self.compression.unwrap_or(default.compression),
            strict_host_key_checking: self
                .strict_host_key_checking
                .unwrap_or(default.strict_host_key_checking),
            keep_alive_interval: self.keep_alive_interval.or(default.keep_alive_interval),
        };

        config.validate()?;
        Ok(config)
    }
}

pub struct SSHConnectionPool {
    connections: std::collections::HashMap<String, SSHManager>,
    max_connections: usize,
}

impl SSHConnectionPool {
    pub fn new(max_connections: usize) -> Self {
        SSHConnectionPool {
            connections: std::collections::HashMap::new(),
            max_connections,
        }
    }

    pub async fn get_or_create_connection(
        &mut self,
        key: String,
        config: SSHConfig,
    ) -> Result<&mut SSHManager, SSHError> {
        if !self.connections.contains_key(&key) {
            if self.connections.len() >= self.max_connections {
                return Err(SSHError::ConnectionFailed(
                    "Maximum number of connections reached".to_string(),
                ));
            }

            let mut manager = SSHManager::new(config)?;
            manager.connect().await?;
            self.connections.insert(key.clone(), manager);
        }

        Ok(self.connections.get_mut(&key).unwrap())
    }

    pub async fn remove_connection(&mut self, key: &str) -> Result<(), SSHError> {
        if let Some(mut connection) = self.connections.remove(key) {
            connection.disconnect().await?;
        }
        Ok(())
    }

    pub async fn close_all(&mut self) -> Result<(), SSHError> {
        for (_, mut connection) in self.connections.drain() {
            if let Err(e) = connection.disconnect().await {
                error!("Error closing connection: {}", e);
            }
        }
        Ok(())
    }
}
