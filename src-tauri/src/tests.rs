//! 引擎功能验证测试（真实文件系统，不依赖 UI）

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::engine::{duplicates, junk, large_files, scanner, settings::Settings};

fn tmpdir(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join("diskcleanup_selftest").join(name);
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p.canonicalize().unwrap()
}

fn write_file(p: &Path, size: usize, fill: u8) {
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let mut f = fs::File::create(p).unwrap();
    let buf = vec![fill; 1024];
    let mut written = 0;
    while written < size {
        let n = std::cmp::min(buf.len(), size - written);
        f.write_all(&buf[..n]).unwrap();
        written += n;
    }
    f.flush().unwrap();
}

#[test]
fn dir_size_sums_all_files() {
    let root = tmpdir("dir_size");
    write_file(&root.join("a.bin"), 3000, 1);
    write_file(&root.join("sub/b.bin"), 5000, 2);
    let (bytes, files) = scanner::dir_size(&root, 10_000);
    assert_eq!(files, 2);
    assert_eq!(bytes, 8000);
}

#[test]
fn large_files_filters_by_size_and_sorts_desc() {
    let root = tmpdir("large_files");
    write_file(&root.join("big.bin"), 6 * 1024 * 1024, 3);
    write_file(&root.join("mid.bin"), 2 * 1024 * 1024, 4);
    write_file(&root.join("small.txt"), 1024, 5);

    let out = large_files::get_large_files(&root, 1024 * 1024, 10, vec![]);
    assert_eq!(out.len(), 2, "应只返回 >=1MB 的文件");
    assert_eq!(out[0].name, "big.bin");
    assert_eq!(out[1].name, "mid.bin");
    assert!(out[0].size_bytes > out[1].size_bytes);
    assert!(!out[0].modified.is_empty());
}

#[test]
fn duplicates_detects_identical_content() {
    let root = tmpdir("duplicates");
    let payload = vec![7u8; 1024];
    let a = root.join("x/dup1.bin");
    let b = root.join("y/dup2.bin");
    let c = root.join("z/unique.bin");
    fs::create_dir_all(a.parent().unwrap()).unwrap();
    fs::create_dir_all(b.parent().unwrap()).unwrap();
    fs::create_dir_all(c.parent().unwrap()).unwrap();
    let mut content = Vec::new();
    while content.len() < 2 * 1024 * 1024 {
        content.extend_from_slice(&payload);
    }
    fs::write(&a, &content).unwrap();
    fs::write(&b, &content).unwrap();
    let mut other = content.clone();
    other[0] = 9;
    fs::write(&c, other).unwrap();

    let groups = duplicates::find_duplicates(&root, 1024 * 1024, 10, vec![]);
    assert_eq!(groups.len(), 1, "应识别出 1 组重复");
    let g = &groups[0];
    assert_eq!(g.files.len(), 2, "组内应有 2 个副本");
    assert_eq!(g.size_bytes, content.len() as u64);
    let names: Vec<&str> = g.files.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"dup1.bin"));
    assert!(names.contains(&"dup2.bin"));
}

#[test]
fn similar_files_are_grouped_by_content_similarity() {
    let root = tmpdir("similar");
    // 构造两份「高度相似但不完全相同」的文件：中间 1% 内容不同
    let base: Vec<u8> = (0usize..4 * 1024 * 1024).map(|i| (i % 251) as u8).collect();
    let mut variant = base.clone();
    for i in (0..variant.len()).step_by(100) {
        variant[i] = variant[i].wrapping_add(1);
    }
    fs::create_dir_all(root.join("a")).unwrap();
    fs::create_dir_all(root.join("b")).unwrap();
    // 典型的「原图 + 副本」命名：photo.jpg / photo 副本.jpg
    fs::write(root.join("a/photo.jpg"), &base).unwrap();
    fs::write(root.join("b/photo 副本.jpg"), &variant).unwrap();
    // 完全不相关的文件，不应被判为相似
    let noise: Vec<u8> = (0usize..4 * 1024 * 1024)
        .map(|i| ((i.wrapping_mul(7919)) % 253) as u8)
        .collect();
    fs::write(root.join("b/noise.bin"), noise).unwrap();

    let groups = duplicates::find_duplicates(&root, 1024 * 1024, 20, vec![]);
    let similar: Vec<_> = groups.iter().filter(|g| g.kind == "similar").collect();
    assert_eq!(similar.len(), 1, "应识别出 1 组相似文件");
    assert_eq!(similar[0].files.len(), 2, "组内 2 个文件");
    assert!(similar[0].similarity >= 0.6);

    let exact: Vec<_> = groups.iter().filter(|g| g.kind == "exact").collect();
    assert!(exact.is_empty(), "没有内容完全相同的文件，不应产生 exact 组");
}

#[test]
fn exclude_dirs_are_skipped() {
    let root = tmpdir("excludes");
    write_file(&root.join("keep/big.bin"), 2 * 1024 * 1024, 6);
    write_file(&root.join("skip/big.bin"), 2 * 1024 * 1024, 6);

    let excludes = vec![root.join("skip")];
    let out = large_files::get_large_files(&root, 1024 * 1024, 10, excludes.clone());
    assert_eq!(out.len(), 1, "被排除目录下的文件不应出现在结果中");
    assert_eq!(out[0].dir, root.join("keep").to_string_lossy().to_string());

    let all = large_files::get_large_files(&root, 1024 * 1024, 10, vec![]);
    assert_eq!(all.len(), 2, "不传排除目录时应全部返回");
}

#[test]
fn junk_collect_computes_category_sizes() {
    let root = tmpdir("junk");
    write_file(&root.join("cache1.dat"), 4000, 8);
    write_file(&root.join("cache2.dat"), 6000, 9);

    let rules = vec![junk::JunkRule {
        category: "测试缓存".into(),
        name: "测试缓存目录".into(),
        description: "单元测试用".into(),
        path: root.clone(),
        safe: true,
    }];
    let cats = junk::collect(&rules);
    assert_eq!(cats.len(), 1);
    assert_eq!(cats[0].total_bytes, 10000);
    assert_eq!(cats[0].items.len(), 2);
    assert!(cats[0].items[0].safe);
    let payload = serde_json::to_value(&cats).unwrap();
    assert_eq!(payload[0]["totalBytes"], 10000);
    assert_eq!(payload[0]["items"][0]["sizeBytes"], 6000);
}

#[test]
fn settings_roundtrip_persists() {
    let mut s = Settings::default();
    s.large_file_min_mb = 500;
    s.exclude_dirs = vec!["/tmp/never".into()];
    s.save().unwrap();

    let loaded = Settings::load();
    assert_eq!(loaded.large_file_min_mb, 500);
    assert!(loaded.exclude_dirs.contains(&"/tmp/never".to_string()));

    // 还原默认值，避免污染开发环境
    Settings::default().save().unwrap();
}

// Exercise actual Rust serialization, not hand-written frontend-shaped mocks.
#[test]
fn ipc_junk_payload_matches_frontend_capacity_fields() {
    use crate::engine::types::{JunkCategory, JunkItem};
    let data = serde_json::to_value(vec![JunkCategory {
        id: "system".into(), name: "系统缓存".into(), total_bytes: 3072,
        items: vec![JunkItem {
            id: "/tmp/cache".into(), name: "系统缓存".into(), path: "/tmp/cache".into(),
            description: "缓存".into(), size_bytes: 3072, safe: true,
        }],
    }]).unwrap();
    assert_eq!(data[0]["totalBytes"], 3072);
    assert_eq!(data[0]["items"][0]["sizeBytes"], 3072);
    assert!(data[0].get("total_bytes").is_none());
}

#[test]
fn ipc_disk_scan_and_cleanup_payloads_match_frontend() {
    use crate::engine::types::*;
    let disk = serde_json::to_value(DiskInfo {
        name: "System".into(), mount_point: "/".into(), total_bytes: 4096,
        used_bytes: 1024, is_system: true,
    }).unwrap();
    assert_eq!(disk["mountPoint"], "/");
    assert_eq!(disk["totalBytes"], 4096);
    assert_eq!(disk["usedBytes"], 1024);
    assert_eq!(disk["isSystem"], true);
    let scan = serde_json::to_value(ScanStatus {
        scanned_bytes: 1024, scanned_files: 3, current_path: "/tmp".into(),
        recent_files: vec!["/tmp/test".into()], ..Default::default()
    }).unwrap();
    assert_eq!(scan["scannedBytes"], 1024);
    assert_eq!(scan["scannedFiles"], 3);
    assert_eq!(scan["currentPath"], "/tmp");
    assert_eq!(scan["recentFiles"][0], "/tmp/test");
    let result = serde_json::to_value(ScanResult {
        top_folders: vec![FolderStat { name: "tmp".into(), path: "/tmp".into(),
            size_bytes: 1024, file_count: 3, percent: 100.0 }], ..Default::default()
    }).unwrap();
    assert_eq!(result["topFolders"][0]["sizeBytes"], 1024);
    assert_eq!(result["topFolders"][0]["fileCount"], 3);
    assert!(result["dirNames"].is_array());
    assert!(result["sizeBuckets"].is_array());
    let summary = serde_json::to_value(CleanSummary { freed_bytes: 1024, files: 3, duration_ms: 50, ..Default::default() }).unwrap();
    assert_eq!(summary["freedBytes"], 1024);
    assert_eq!(summary["durationMs"], 50);
    let file = FileEntry { protected_reason: None, path: "/tmp/test".into(), name: "test".into(), dir: "/tmp".into(), size_bytes: 1024, modified: "2026-10-07".into() };
    let group = serde_json::to_value(DuplicateGroup { id: "g1".into(), size_bytes: 1024, category: "其他".into(), kind: "exact".into(), similarity: 1.0, files: vec![file] }).unwrap();
    assert_eq!(group["sizeBytes"], 1024);
    assert_eq!(group["files"][0]["sizeBytes"], 1024);
}

#[test]
fn ipc_settings_round_trip_and_legacy_files_remain_readable() {
    use crate::engine::types::{Snapshot, CleanupRecord};
    let legacy = serde_json::json!({
        "launch_at_login": true, "close_confirm": false, "language": "zh-CN",
        "auto_clean_enabled": true, "auto_clean_interval_days": 30,
        "last_auto_clean_ts": 123, "large_file_min_mb": 500,
        "exclude_dirs": ["/keep"], "theme": "dark"
    });
    let settings: Settings = serde_json::from_value(legacy).unwrap();
    let frontend = serde_json::to_value(&settings).unwrap();
    assert_eq!(frontend["largeFileMinMb"], 500);
    assert_eq!(frontend["excludeDirs"][0], "/keep");
    assert_eq!(frontend["launchAtLogin"], true);
    let back: Settings = serde_json::from_value(frontend.clone()).unwrap();
    assert_eq!(serde_json::to_value(back).unwrap(), frontend);
    let history: Snapshot = serde_json::from_value(serde_json::json!({
        "ts": 1, "root": "/", "total_bytes": 4096, "total_files": 3
    })).unwrap();
    assert_eq!(serde_json::to_value(history).unwrap()["totalBytes"], 4096);
    let cleanup: CleanupRecord = serde_json::from_value(serde_json::json!({
        "ts": 1, "items": [{"path": "/tmp/test", "size_bytes": 1024}]
    })).unwrap();
    assert_eq!(serde_json::to_value(cleanup).unwrap()["items"][0]["sizeBytes"], 1024);
}

#[test]
fn analysis_scans_real_files_and_reports_progress() {
    use std::sync::atomic::AtomicBool;
    let root = tmpdir("analysis-progress");
    write_file(&root.join("docs/a.txt"), 3000, 1);
    write_file(&root.join("docs/b.bin"), 7000, 2);
    write_file(&root.join("excluded/cache.dat"), 9000, 3);
    let mut updates = Vec::new();
    let result = scanner::scan_directory(&root, &AtomicBool::new(false), &[root.join("excluded")],
        |s| updates.push(s.clone())).unwrap().unwrap();
    assert_eq!(result.total_files, 2);
    let expected = scanner::analysis_bytes(&fs::metadata(root.join("docs/a.txt")).unwrap()) + scanner::analysis_bytes(&fs::metadata(root.join("docs/b.bin")).unwrap());
    assert_eq!(result.total_bytes, expected);
    assert_eq!(result.categories.iter().map(|c| c.bytes).sum::<u64>(), expected);
    assert_eq!(result.size_buckets.iter().map(|b| b.count).sum::<u64>(), 2);
    assert_eq!(updates.first().unwrap().current_path, root.to_string_lossy());
    assert_eq!(updates.last().unwrap().scanned_files, 2);
    assert_eq!(updates.last().unwrap().scanned_bytes, expected);
}

#[test]
fn analysis_cancellation_does_not_return_partial_success() {
    use std::sync::atomic::{AtomicBool, Ordering};
    let root = tmpdir("analysis-cancel");
    write_file(&root.join("a.bin"), 1000, 1);
    let cancel = AtomicBool::new(false);
    let result = scanner::scan_directory(&root, &cancel, &[], |s| {
        assert!(s.percent < 100.0);
        cancel.store(true, Ordering::Relaxed);
    }).unwrap();
    assert!(result.is_none());
}

#[test]
fn analysis_empty_directory_completes_and_invalid_root_fails() {
    use std::sync::atomic::AtomicBool;
    let root = tmpdir("analysis-empty");
    let cancel = AtomicBool::new(false);
    let mut reports = 0;
    let result = scanner::scan_directory(&root, &cancel, &[], |_| reports += 1).unwrap().unwrap();
    assert_eq!(result.total_files, 0);
    assert!(reports > 0);
    assert!(scanner::scan_directory(&root.join("missing"), &cancel, &[], |_| {}).is_err());
}

#[cfg(unix)]
#[test]
fn analysis_does_not_follow_symlink_loops() {
    use std::sync::atomic::AtomicBool;
    let root = tmpdir("analysis-links");
    write_file(&root.join("a.bin"), 1000, 1);
    std::os::unix::fs::symlink(&root, root.join("loop")).unwrap();
    let result = scanner::scan_directory(&root, &AtomicBool::new(false), &[], |_| {}).unwrap().unwrap();
    assert_eq!(result.total_files, 1);
    assert_eq!(result.total_bytes, scanner::analysis_bytes(&fs::metadata(root.join("a.bin")).unwrap()));
}

#[cfg(unix)]
#[test]
fn analysis_sparse_terabyte_image_counts_allocated_blocks_not_logical_length() {
    use std::sync::atomic::AtomicBool;
    use std::os::unix::fs::MetadataExt;
    let root = tmpdir("analysis-sparse");
    let path = root.join("Docker.raw");
    let mut image = fs::File::create(&path).unwrap();
    image.set_len(1024_u64.pow(4)).unwrap();
    image.write_all(&[1; 4096]).unwrap();
    image.sync_all().unwrap();
    let meta = fs::metadata(&path).unwrap();
    assert_eq!(meta.len(), 1024_u64.pow(4));
    assert!(meta.blocks() * 512 < meta.len());
    let mut progress = Vec::new();
    let result = scanner::scan_directory(&root, &AtomicBool::new(false), &[], |s| progress.push(s.clone())).unwrap().unwrap();
    assert_eq!(result.total_bytes, meta.blocks() * 512);
    assert_eq!(result.size_basis, "allocated");
    assert_eq!(result.total_files, 1);
    assert!(progress.iter().all(|s| s.percent == 0.0), "unknown work must not claim 99% complete");
    assert_eq!(progress.last().unwrap().scanned_bytes, result.total_bytes);
    fs::remove_file(path).unwrap();
}

#[cfg(unix)]
#[test]
fn analysis_hard_links_count_storage_once() {
    use std::sync::atomic::AtomicBool;
    let root = tmpdir("analysis-hardlinks");
    write_file(&root.join("original.bin"), 12000, 1);
    fs::hard_link(root.join("original.bin"), root.join("copy.bin")).unwrap();
    let result = scanner::scan_directory(&root, &AtomicBool::new(false), &[], |_| {}).unwrap().unwrap();
    assert_eq!(result.total_files, 1);
    assert_eq!(result.total_bytes, scanner::analysis_bytes(&fs::metadata(root.join("original.bin")).unwrap()));
    assert_eq!(result.categories.iter().map(|c| c.bytes).sum::<u64>(), result.total_bytes);
}
