use serde::{Deserialize, Serialize};

/// 序列化通用类型：Rust 引擎 <-> 前端 的数据契约。

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub is_system: bool,
}

#[derive(Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScanStatus {
    pub running: bool,
    pub percent: f64,
    pub scanned_bytes: u64,
    pub scanned_files: u64,
    pub current_path: String,
    pub recent_files: Vec<String>,
    pub skipped_entries: u64,
    pub error: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryStat {
    pub name: String,
    pub bytes: u64,
    pub percent: f64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderStat {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub file_count: u64,
    pub percent: f64,
}

/// 按目录名（basename）聚合的统计，如 node_modules、Library
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirNameStat {
    pub name: String,
    pub size_bytes: u64,
    pub file_count: u64,
    pub occurrences: u64,
    pub percent: f64,
}

/// 文件大小分布桶
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BucketStat {
    pub label: String,
    pub count: u64,
    pub bytes: u64,
    pub percent: f64,
}

/// 历史快照（每次扫描完成写入一条）
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    #[serde(default, alias = "size_basis")]
    pub size_basis: String,
    pub ts: u64,
    pub root: String,
    #[serde(alias = "total_bytes")]
    pub total_bytes: u64,
    #[serde(alias = "total_files")]
    pub total_files: u64,
}

/// 清理记录（用于恢复）
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupRecord {
    pub ts: u64,
    pub items: Vec<CleanupItem>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupItem {
    pub path: String,
    #[serde(alias = "size_bytes")]
    pub size_bytes: u64,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub trash_id: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub size_basis: String,
    pub root: String,
    pub total_bytes: u64,
    pub total_files: u64,
    pub categories: Vec<CategoryStat>,
    pub top_folders: Vec<FolderStat>,
    pub dir_names: Vec<DirNameStat>,
    pub size_buckets: Vec<BucketStat>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JunkItem {
    pub id: String,
    pub name: String,
    pub path: String,
    pub description: String,
    pub size_bytes: u64,
    pub safe: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JunkCategory {
    pub id: String,
    pub name: String,
    pub total_bytes: u64,
    pub items: Vec<JunkItem>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub protected_reason: Option<String>,
    pub path: String,
    pub name: String,
    pub dir: String,
    pub size_bytes: u64,
    pub modified: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup {
    pub id: String,
    pub size_bytes: u64,
    pub category: String,
    /// "exact" = 内容完全一致；"similar" = 内容高度相似（minhash 估算）
    pub kind: String,
    /// exact 为 1.0；similar 为 0~1 的相似度
    pub similarity: f64,
    pub files: Vec<FileEntry>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    /// "open" | "junk-page" | "smart" | "empty-trash"
    pub action: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchItem {
    pub name: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanProgress {
    pub done: usize,
    pub total: usize,
    pub path: String,
    pub freed_bytes: u64,
}

#[derive(Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CleanSummary {
    /// Compatibility field, never reports recycled bytes as reclaimed space.
    pub freed_bytes: u64,
    pub moved_bytes: u64,
    pub failures: Vec<CleanFailure>,
    pub files: u64,
    pub duration_ms: u64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanFailure {
    pub path: String,
    pub reason: String,
}
#[derive(Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RestoreSummary {
    pub restored: usize,
    pub failures: Vec<CleanFailure>,
}
