use serde::{Deserialize, Serialize};

// crash report struct
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CrashReport {
    pub version: i32,
    pub title: String,
    #[serde(rename = "display-title")]
    pub display_title: String,
    pub id: String,
    pub status: String,
    #[serde(rename = "fix-commits")]
    pub fix_commits: Vec<FixCommit>,
    pub discussions: Vec<String>,
    pub crashes: Vec<Crash>,
    pub subsystems: Vec<String>,
    #[serde(rename = "parent_of_fix_commit")]
    pub parent_of_fix_commit: String,
    pub patch: String,
    #[serde(rename = "patch_modified_files")]
    pub patch_modified_files: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FixCommit {
    pub title: String,
    pub link: String,
    pub hash: String,
    pub repo: String,
    pub branch: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Crash {
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "syz-reproducer")]
    pub syz_reproducer: String,
    #[serde(rename = "c-reproducer")]
    pub c_reproducer: String,
    #[serde(rename = "kernel-config")]
    pub kernel_config: String,
    #[serde(rename = "kernel-source-git")]
    pub kernel_source_git: String,
    #[serde(rename = "kernel-source-commit")]
    pub kernel_source_commit: String,
    #[serde(rename = "syzkaller-git")]
    pub syzkaller_git: String,
    #[serde(rename = "syzkaller-commit")]
    pub syzkaller_commit: String,
    #[serde(rename = "compiler-description")]
    pub compiler_description: String,
    #[serde(rename = "architecture")]
    pub architecture: String,
    #[serde(rename = "crash-report-link")]
    pub crash_report_link: String,
}
