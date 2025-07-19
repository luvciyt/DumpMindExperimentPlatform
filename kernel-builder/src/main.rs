use std::env;
use std::path::{PathBuf};
use tokio::task;
use kernel_builder::kernel::download::{decompress_file, download_file};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(true)                              // 显示模块路径
        .with_thread_ids(true)                          // 显示线程 ID
        .with_thread_names(true)                        // 显示线程名
        .with_line_number(true)                         // 显示行号
        .with_file(true)                                // 显示文件路径
        .compact()                                      // 更紧凑格式（可改成 .pretty() 更易读）
        .init();
    let mut handles = vec![];

    let tar = PathBuf::from("linux1.tar.gz");
    let path = PathBuf::from("linux2.tar.gz");
    let handle = task::spawn(async move {
        download_file("https://github.com/torvalds/linux/archive/eb7081409f94a9a8608593d0fb63a1aa3d6f95d8.tar.gz", &path, false).await.expect("Failed to download file");
    });
    handles.push(handle);
    
    let handle = task::spawn(async move {
        decompress_file(&tar, &env::current_dir().unwrap()).await.expect("Failed to decompress file");
    });
    handles.push(handle);

    for handle in handles {
        if let Err(e) = handle.await {
            eprintln!("Task failed: {:?}", e);
        }
    }

    println!("All tasks completed");
}
