use kernel_builder::kernel::compile::{apply_patch, make_kernel, rebuild_kernel};
use kernel_builder::kernel::download::{
    download_bug, download_config, download_kernel, DownloadError,
};
use kernel_builder::kernel::modify::check_fix_config;
use kernel_builder::kvm::ssh::SSHManager;
use kernel_builder::parse::parse::{build_path, parse_file};
use kernel_builder::script::script::mount;
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

    // let config = SSHManager::builder().build().unwrap();
    //
    // let mut ssh = SSHManager::new(config).unwrap();
    // ssh.connect().await.unwrap();
    //
    // match ssh.execute("kexec -p /boot/crash-bzImage --initrd=/boot/crash-initramfs.cpio.gz --append=\"root=/dev/ram0 console=ttyS0\"").await {
    //     Ok(output) => {
    //         println!("命令输出: {}", output);
    //     }
    //     Err(e) => {
    //         eprintln!("SSH 命令执行失败: {}", e);
    //     }
    // }
    //
    // match ssh.execute("./bug").await {
    //     Ok(output) => {
    //         println!("命令输出: {}", output);
    //     }
    //     Err(e) => {
    //         eprintln!("SSH 命令执行失败: {}", e);
    //     }
    // }

    let mut handles = vec![];

    let path = "datasets/0be4824a86385f022a4f6f5104bcb9246032fdd9.json";
    let report = Arc::new(parse_file(path).unwrap());
    // let build_dir = build_path(&report);
    // let patch_path = build_dir.join("patch.diff");

    // apply_patch(&report, patch_path)
    //     .await
    //     .unwrap_or_else(|err| {
    //         error!("Failed to apply patch: {}", err);
    //     });

    // rebuild_kernel(&report).await.expect("TODO: panic message");

    match download_kernel(&report).await {
        Ok(()) => {}
        Err(err) => {
            error!("{}", err);
        }
    }

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

    match check_fix_config(&report).await {
        Ok(()) => {}
        Err(err) => {
            error!("{}", err);
        }
    }

    match make_kernel(&report).await {
        Ok(()) => {}
        Err(err) => {
            error!("{}", err);
        }
    }

    match mount(&report).await {
        Ok(()) => {}
        Err(err) => {
            error!("{}", err);
        }
    }
}
