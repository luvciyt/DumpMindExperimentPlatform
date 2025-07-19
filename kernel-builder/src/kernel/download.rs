use crate::config::config::Config;
use anyhow::{Context, Result};
use reqwest::Client;
use std::path::Path;
use tokio::fs;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufReader, BufWriter};
use tracing::info;

const KERNEL_DOWNLOAD_URL: &str = "https://github.com/torvalds/linux/archive/";
const SYZKALLER_URL: &str = "https://syzkaller.appspot.com/";

pub async fn download_file(url: &str, target: &Path, use_proxy: bool) -> Result<()> {
    info!("Downloading file from: {}", url);
    info!("Saving to: {}", target.display());

    if Path::exists(target) {
        anyhow::bail!("File already exists: {}", target.display());
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

pub async fn decompress_file(source: &Path, target: &Path) -> Result<()> {
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
