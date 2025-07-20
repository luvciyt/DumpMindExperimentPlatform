use kernel_builder::kernel::download::{download_bug, download_config, DownloadError};
use kernel_builder::parse::parse::parse_file;
use std::sync::Arc;
use tracing::{error, info, warn};
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_line_number(true)
        .with_file(true)
        .pretty()
        .init();

    let mut handles = vec![];

    let path = "datasets/0b6b2d6d6cefa8b462930e55be699efba635788f.json";
    let report = Arc::new(parse_file(path).unwrap());

    let handle = {
        let report = Arc::clone(&report);
        tokio::spawn(async move { download_bug(&report).await })
    };
    handles.push(handle);

    let handle = {
        let report = Arc::clone(&report);
        tokio::spawn(async move { download_config(&report).await })
    };
    handles.push(handle);

    for handle in handles {
        match handle.await {
            Err(join_err) => {
                error!("任务 panic 或被取消: {:?}", join_err);
            }
            Ok(Err(err)) => {
                // 判断是否是 DownloadError::FileExists 错误
                if let Some(download_error) = err.downcast_ref::<DownloadError>() {
                    match download_error {
                        DownloadError::FileExists(path) => {
                            warn!("文件已存在，跳过错误: {}", path);
                            continue;
                        }
                        _ => {
                            error!("任务失败: {:?}", err);
                        }
                    }
                } else {
                    error!("任务失败: {:?}", err);
                }
            }
            Ok(Ok(())) => {
                info!("任务成功");
            }
        }
    }

    println!("All tasks completed");
}
