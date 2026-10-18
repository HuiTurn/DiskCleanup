//! 磁盘扫描引擎：后台线程遍历目录，统计体积/分类/Top 文件夹，
//! 通过事件向前端推送进度，支持取消。平台无关。

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;

use super::types::{
    BucketStat, CategoryStat, DirNameStat, FolderStat, ScanResult, ScanStatus,
};

/// 大小分布桶标签
const BUCKET_LABELS: [&str; 6] = [
    "< 1 MB",
    "1 - 10 MB",
    "10 - 100 MB",
    "100 - 500 MB",
    "500 MB - 1 GB",
    "> 1 GB",
];

fn bucket_index(size: u64) -> usize {
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * MB;
    if size < MB {
        0
    } else if size < 10 * MB {
        1
    } else if size < 100 * MB {
        2
    } else if size < 500 * MB {
        3
    } else if size < GB {
        4
    } else {
        5
    }
}

pub struct ScanState {
    pub status: Arc<Mutex<ScanStatus>>,
    pub result: Arc<Mutex<Option<ScanResult>>>,
    pub cancel: Mutex<Option<Arc<AtomicBool>>>,
}

impl Default for ScanState {
    fn default() -> Self {
        Self {
            status: Arc::new(Mutex::new(ScanStatus::default())),
            result: Arc::new(Mutex::new(None)),
            cancel: Mutex::new(None),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct ScanFinished {
    pub result: ScanResult,
}

/// 文件扩展名 -> 展示分类
fn classify(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "ico" | "bmp" | "tiff" | "heic" => "图片文件",
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "dmg" | "iso" | "pkg" => "安装/压缩包",
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "txt" | "md" | "csv" | "pages"
        | "numbers" | "key" | "epub" => "文档",
        "mp3" | "wav" | "aac" | "flac" | "m4a" | "ogg" => "音频文件",
        "mp4" | "mov" | "avi" | "mkv" | "wmv" | "flv" | "webm" => "视频文件",
        "dll" | "so" | "dylib" | "sys" | "bin" | "app" | "exe" | "msi" | "framework" | "node" => "系统/应用文件",
        "cache" | "log" | "tmp" => "冗余缓存",
        _ => "其他",
    }
}

/// 分类展示顺序（截图 03 的顺序）
const CATEGORY_ORDER: [&str; 8] = [
    "系统/应用文件",
    "安装/压缩包",
    "图片文件",
    "系统更新缓存",
    "文档",
    "视频文件",
    "音频文件",
    "其他",
];

/// 启动扫描线程（取消句柄由 commands 层注册进 ScanState；结束后把结果写入 result_slot）
pub fn spawn_scan(
    app: AppHandle,
    root: PathBuf,
    cancel: Arc<AtomicBool>,
    result_slot: Arc<Mutex<Option<ScanResult>>>,
    status_slot: Arc<Mutex<ScanStatus>>,
    excludes: Vec<PathBuf>,
) {
    std::thread::spawn(move || {
        let initial = status_slot.lock().unwrap().clone();
        let _ = app.emit("scan-progress", initial);
        let outcome = scan_directory(&root, &cancel, &excludes, |status| {
            *status_slot.lock().unwrap() = status.clone();
            let _ = app.emit("scan-progress", status);
        });
        let mut status = status_slot.lock().unwrap();
        status.running = false;
        match outcome {
            Ok(Some(result)) if !cancel.load(Ordering::Relaxed) => {
                status.percent = 100.0;
                status.scanned_bytes = result.total_bytes;
                status.scanned_files = result.total_files;
                *result_slot.lock().unwrap() = Some(result.clone());
                // Store the result before announcing completion. Canceled scans never enter history.
                super::history::push(&result.root, result.total_bytes, result.total_files, &result.size_basis);
                let _ = app.emit("scan-finished", ScanFinished { result });
            }
            Err(error) => status.error = Some(error),
            _ => {}
        }
        let _ = app.emit("scan-progress", &*status);
    });
}

/// Pure traversal used by the worker and real-filesystem regression tests.
pub(crate) fn scan_directory(
    root: &Path,
    cancel: &AtomicBool,
    excludes: &[PathBuf],
    mut report: impl FnMut(&ScanStatus),
) -> Result<Option<ScanResult>, String> {
    std::fs::read_dir(root).map_err(|e| format!("无法读取扫描目录 {}：{}", root.display(), e))?;
    let mut folder_bytes: HashMap<PathBuf, (u64, u64)> = HashMap::new(); // 顶层子目录 -> (bytes, files)
    let mut dirname_bytes: HashMap<String, (u64, u64, u64)> = HashMap::new(); // 目录名 -> (bytes, files, 出现目录数)
    let mut bucket: Vec<(u64, u64)> = vec![(0u64, 0u64); BUCKET_LABELS.len()]; // (count, bytes)
    let mut seen_dirs: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
    let mut cat_bytes: HashMap<&'static str, u64> = HashMap::new();
    let mut status = ScanStatus {
        running: true,
        current_path: root.to_string_lossy().to_string(),
        ..Default::default()
    };
    report(&status);
    let mut recent: VecDeque<String> = VecDeque::with_capacity(8);
    let mut last_emit = Instant::now() - Duration::from_millis(500);
    let mut total_bytes: u64 = 0;
    let mut total_files: u64 = 0;


    #[cfg(unix)]
    let mut visited_dirs = std::collections::HashSet::new();
    #[cfg(unix)]
    let mut linked_files = std::collections::HashSet::new();
    let walker = WalkDir::new(&root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            if is_protected(e.path()) || is_excluded(e.path(), excludes) || redundant_mount(root, e.path()) { return false; }
            #[cfg(unix)]
            if e.file_type().is_dir() {
                use std::os::unix::fs::MetadataExt;
                if let Ok(meta) = e.metadata() {
                    // APFS firmlinks expose the same directory under multiple paths.
                    if !visited_dirs.insert((meta.dev(), meta.ino())) { return false; }
                }
            }
            true
        });

    for entry in walker {
        if cancel.load(Ordering::Relaxed) {
            report(&status);
            return Ok(None);
        }
        // Report directory traversal too, so unreadable/empty trees do not look frozen.
        if last_emit.elapsed() >= Duration::from_millis(180) {
            last_emit = Instant::now();
            report(&status);
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => { status.skipped_entries += 1; continue; }
        };
        status.current_path = entry.path().to_string_lossy().to_string();
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => { status.skipped_entries += 1; continue; },
        };
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if meta.nlink() > 1 && !linked_files.insert((meta.dev(), meta.ino())) { continue; }
        }
        let size = analysis_bytes(&meta);
        total_bytes += size;
        total_files += 1;

        // 顶层子目录聚合
        if let Some(top) = top_child(&root, path) {
            let e = folder_bytes.entry(top).or_insert((0, 0));
            e.0 += size;
            e.1 += 1;
        }
        // 目录名聚合（如 node_modules / Library 分散各处，按名字合并）
        if let Some(parent) = path.parent() {
            if seen_dirs.insert(parent.to_path_buf()) {
                let name = parent
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| parent.to_string_lossy().to_string());
                let e = dirname_bytes.entry(name).or_insert((0, 0, 0));
                e.2 += 1;
            }
            let name = parent
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| parent.to_string_lossy().to_string());
            let e = dirname_bytes.entry(name).or_insert((0, 0, 0));
            e.0 += size;
            e.1 += 1;
        }
        // 大小分布
        let bi = bucket_index(size);
        bucket[bi].0 += 1;
        bucket[bi].1 += size;
        // 分类聚合
        let cat = classify(
            path.file_name()
                .map(|s| s.to_string_lossy().to_string())
                .as_deref()
                .unwrap_or(""),
        );
        *cat_bytes.entry(cat).or_insert(0) += size;

        // 实时文件流
        let display = path.to_string_lossy().to_string();
        recent.push_back(display.clone());
        if recent.len() > 8 {
            recent.pop_front();
        }

        status.current_path = display;
        status.scanned_bytes = total_bytes;
        status.scanned_files = total_files;
        // Total traversal work is unknown: percent remains 0 until successful completion.
        status.recent_files = recent.iter().cloned().collect();

        if last_emit.elapsed() >= Duration::from_millis(180) {
            last_emit = Instant::now();
            report(&status);
        }
    }

    report(&status);
    if cancel.load(Ordering::Relaxed) { return Ok(None); }

    let mut categories: Vec<CategoryStat> = CATEGORY_ORDER
        .iter()
        .filter_map(|name| {
            cat_bytes.get(*name).map(|&bytes| CategoryStat {
                name: name.to_string(),
                bytes,
                percent: if total_bytes > 0 {
                    bytes as f64 / total_bytes as f64 * 100.0
                } else {
                    0.0
                },
            })
        })
        .collect();
    // 补充未列入固定顺序的分类
    for (name, bytes) in &cat_bytes {
        if !categories.iter().any(|c| &c.name == name) {
            categories.push(CategoryStat {
                name: name.to_string(),
                bytes: *bytes,
                percent: if total_bytes > 0 {
                    *bytes as f64 / total_bytes as f64 * 100.0
                } else {
                    0.0
                },
            });
        }
    }
    categories.sort_by(|a, b| b.bytes.cmp(&a.bytes));

    let mut top_folders: Vec<FolderStat> = folder_bytes
        .into_iter()
        .filter(|(p, _)| p.as_path() != root)
        .map(|(p, (bytes, files))| FolderStat {
            name: p
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| p.to_string_lossy().to_string()),
            path: p.to_string_lossy().to_string(),
            size_bytes: bytes,
            file_count: files,
            percent: if total_bytes > 0 {
                bytes as f64 / total_bytes as f64 * 100.0
            } else {
                0.0
            },
        })
        .collect();
    top_folders.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    top_folders.truncate(6);

    let mut dir_names: Vec<DirNameStat> = dirname_bytes
        .into_iter()
        .map(|(name, (bytes, files, occurrences))| DirNameStat {
            name,
            size_bytes: bytes,
            file_count: files,
            occurrences,
            percent: if total_bytes > 0 {
                bytes as f64 / total_bytes as f64 * 100.0
            } else {
                0.0
            },
        })
        .collect();
    dir_names.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    dir_names.truncate(10);

    let size_buckets: Vec<BucketStat> = BUCKET_LABELS
        .iter()
        .enumerate()
        .map(|(i, label)| BucketStat {
            label: label.to_string(),
            count: bucket[i].0,
            bytes: bucket[i].1,
            percent: if total_bytes > 0 {
                bucket[i].1 as f64 / total_bytes as f64 * 100.0
            } else {
                0.0
            },
        })
        .collect();

    let result = ScanResult {
        root: root.to_string_lossy().to_string(),
        total_bytes,
        total_files,
        categories,
        top_folders,
        dir_names,
        size_buckets,
        size_basis: size_basis().into(),
    };
    Ok(Some(result))
}

/// 判断是否应跳过的系统保护路径（权限受限或无意义）
fn is_protected(_p: &Path) -> bool {
    #[cfg(target_os = "macos")]
    {
        let s = _p.to_string_lossy();
        if s.starts_with("/System/Volumes/Data/.Spotlight")
            || s.starts_with("/private/var/db")
            || s.starts_with("/dev")
        {
            return true;
        }
    }
    false
}

/// 用户配置的排除目录（设置页「扫描排除目录」）
pub fn is_excluded(p: &Path, excludes: &[PathBuf]) -> bool {
    super::safety::excluded(p, excludes)
}

/// 返回 path 相对 root 的顶层子目录；若 path 直接位于 root 下则返回自身
fn top_child(root: &Path, path: &Path) -> Option<PathBuf> {
    let mut anc = path.ancestors().skip(1);
    loop {
        let a = anc.next()?;
        if a == root {
            return None;
        }
        if a.parent()? == root {
            return Some(a.to_path_buf());
        }
    }
}

/// 目录体积统计（供清理页使用，带条目数上限防卡死）
pub fn dir_size(path: &Path, max_entries: u64) -> (u64, u64) {
    let mut bytes = 0u64;
    let mut files = 0u64;
    for entry in WalkDir::new(path).follow_links(false).into_iter().flatten() {
        if files >= max_entries {
            break;
        }
        if entry.file_type().is_file() {
            if let Ok(m) = entry.metadata() {
                bytes += m.len();
                files += 1;
            }
        }
    }
    (bytes, files)
}

/// File allocation is different from apparent size for sparse VM/container images.
pub(crate) fn analysis_bytes(meta: &std::fs::Metadata) -> u64 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        meta.blocks().saturating_mul(512)
    }
    #[cfg(not(unix))]
    { meta.len() }
}

pub(crate) fn size_basis() -> &'static str {
    if cfg!(unix) { "allocated" } else { "logical" }
}

fn redundant_mount(root: &Path, path: &Path) -> bool {
    // Scan the merged macOS root once, not its backing APFS volumes and mounted images too.
    cfg!(target_os = "macos") && root == Path::new("/") &&
        (path.starts_with("/System/Volumes") || path.starts_with("/Volumes"))
}

#[cfg(all(test, target_os = "macos"))]
mod mount_tests {
    use super::*;
    #[test]
    fn analysis_merged_root_skips_backing_volumes_but_explicit_roots_are_allowed() {
        assert!(redundant_mount(Path::new("/"), Path::new("/System/Volumes/Data")));
        assert!(redundant_mount(Path::new("/"), Path::new("/Volumes/External")));
        assert!(!redundant_mount(Path::new("/"), Path::new("/Users")));
        assert!(!redundant_mount(Path::new("/Volumes/External"), Path::new("/Volumes/External/files")));
    }
}
