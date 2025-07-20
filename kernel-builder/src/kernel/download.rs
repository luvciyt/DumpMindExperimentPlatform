use crate::config::config::Config;
use crate::parse::parse::{build_path, kernel_source_path};
use crate::parse::report::{CrashReport};
use anyhow::{Context, Result};
use reqwest::Client;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use tokio::fs;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};
use tracing::{error, info, warn};

const KERNEL_DOWNLOAD_URL: &str = "https://github.com/torvalds/linux/archive/";
const SYZKALLER_URL: &str = "https://syzkaller.appspot.com/";

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("File already exists: {0}")]
    FileExists(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

async fn download_file(url: &str, target: &Path, use_proxy: bool) -> Result<()> {
    info!("Downloading file from: {}", url);
    info!("Saving to: {}", target.display());

    if Path::exists(target) {
        return Err(DownloadError::FileExists(target.display().to_string()).into());
    }

    let client = if use_proxy {
        let config: Config = Config::default();
        let proxy_url = format!("http://{}:{}", config.proxy.host, config.proxy.port);

        let proxy = reqwest::Proxy::all(&proxy_url)
            .with_context(|| format!("Failed to create HTTP proxy with URL {}", proxy_url))?;

        Client::builder()
            .proxy(proxy)
            .build()
            .with_context(|| "Failed to create HTTP client")?
    } else {
        Client::builder()
            .no_proxy()
            .build()
            .with_context(|| "Failed to create HTTP client")?
    };

    let mut response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to download from {}", url))?
        .error_for_status()
        .with_context(|| format!("HTTP error while downloading from {}", url))?;

    let mut file = BufWriter::new(
        File::create(&target)
            .await
            .with_context(|| format!("Failed to create file: {}", target.display()))?,
    );

    while let Some(chunk) = response
        .chunk()
        .await
        .with_context(|| "Failed to read response chunk")?
    {
        file.write_all(&chunk)
            .await
            .with_context(|| format!("Failed to write chunk to file: {}", target.display()))?;
    }

    file.flush()
        .await
        .with_context(|| format!("Failed to flush file: {}", target.display()))?;

    info!("Download completed successfully");

    Ok(())
}

async fn decompress_file(source: &Path, target: &Path) -> Result<()> {
    info!("Decompressing file from: {}", source.display());
    info!("Saving decompressed content to: {}", target.display());

    if !fs::try_exists(source).await? {
        anyhow::bail!("Source file does not exist: {}", source.display());
    }

    if !fs::try_exists(target).await? {
        fs::create_dir_all(target)
            .await
            .with_context(|| format!("Failed to create target directory: {}", target.display()))?;
    }

    let source = source.to_owned();
    let target = target.to_owned();

    tokio::task::spawn_blocking(move || -> Result<()> {
        use std::fs::File;
        use std::io::BufReader;

        let file = File::open(&source)
            .with_context(|| format!("Failed to open source file: {}", source.display()))?;
        let buf_reader = BufReader::new(file);

        let decoder = flate2::read::GzDecoder::new(buf_reader);
        let mut archive = tar::Archive::new(decoder);
        archive
            .unpack(&target)
            .with_context(|| format!("Failed to unpack archive to: {}", target.display()))?;

        Ok(())
    })
    .await??;

    info!("Decompression completed successfully");

    Ok(())
}

pub async fn download_kernel(report: &CrashReport) -> Result<()> {
    if report.crashes.is_empty() {
        anyhow::bail!("No crashes found in the report, cannot download kernel.");
    }

    let commit = report.crashes.first().unwrap().kernel_source_commit.clone();
    let download_url = format!("{}{}.tar.gz", KERNEL_DOWNLOAD_URL, commit);

    let file_name = format!("linux-{}.tar.gz", commit);
    let save_dir = build_path(report);

    info!("Preparing to download kernel source from: {}", download_url);

    fs::create_dir_all(&save_dir)
        .await
        .with_context(|| format!("Failed to create directory: {}", save_dir.display()))?;

    let target_path = save_dir.join(file_name);
    let source_dir = kernel_source_path(report);

    if fs::try_exists(&source_dir).await? {
        warn!(
            "Kernel source directory already exists: {}. Skipping download.",
            source_dir.display()
        );
        return Ok(());
    }

    match download_file(&download_url, &target_path, false).await {
        Ok(_) => info!(
            "Kernel source downloaded successfully to: {}",
            target_path.display()
        ),
        Err(e) => {
            if let Some(DownloadError::FileExists(_)) = e.downcast_ref::<DownloadError>() {
                warn!(
                    "Kernel source file already exists: {}. Skipping download.",
                    target_path.display()
                );
            } else {
                error!("Failed to download kernel source: {}", e);
                return Err(e);
            }
        }
    }

    match decompress_file(&target_path, &save_dir).await {
        Ok(_) => info!(
            "Kernel source decompressed successfully to: {}",
            save_dir.display()
        ),
        Err(e) => {
            error!("Failed to decompress kernel source: {}", e);
            return Err(e);
        }
    }

    info!("Kernel source download and extraction completed successfully");

    Ok(())
}

pub async fn download_bug(report: &Arc<CrashReport>) -> Result<()> {
    if report.crashes.is_empty() {
        anyhow::bail!("No crashes found in the report, cannot download bug.");
    }

    let c_reproducer = report.crashes.first().unwrap().c_reproducer.clone();
    let c_reproducer = c_reproducer.trim().trim_start_matches('/');
    let download_url = format!("{}{}", SYZKALLER_URL, c_reproducer);

    info!(
        "Preparing to download bug reproducer from: {}",
        download_url
    );

    let build_dir = build_path(report);
    let reproducer_path = build_dir.join("reproducer.c");

    info!("Saving bug reproducer to: {}", reproducer_path.display());

    if !fs::try_exists(&build_dir).await? {
        anyhow::bail!(
            "Build directory does not exist or is not a directory: {}",
            build_dir.display()
        );
    }

    download_file(&download_url, &reproducer_path, true)
        .await
        .with_context(|| format!("Failed to download bug reproducer from {}", download_url))?;

    info!(
        "Bug reproducer downloaded successfully to: {}",
        reproducer_path.display()
    );

    Ok(())
}

pub async fn download_config(report: &Arc<CrashReport>) -> Result<()> {
    if report.crashes.is_empty() {
        anyhow::bail!("No crashes found in the report, cannot download config.");
    }

    let config = report.crashes.first().unwrap().kernel_config.clone();
    let config = config.trim().trim_start_matches('/');
    let download_url = format!("{}{}", SYZKALLER_URL, config);

    let build_dir = build_path(report).join("build");
    let config_path = build_dir.join(".config");

    info!("Preparing to download kernel config from: {}", download_url);

    fs::create_dir_all(&build_dir)
        .await
        .with_context(|| format!("Failed to create directory: {}", build_dir.display()))?;

    download_file(&download_url, &config_path, true)
        .await
        .with_context(|| format!("Failed to download kernel config from {}", download_url))?;

    info!(
        "Kernel config downloaded successfully to: {}",
        config_path.display()
    );

    Ok(())
}
