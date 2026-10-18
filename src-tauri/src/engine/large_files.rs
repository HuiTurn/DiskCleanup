//! Bounded memory top-N across the entire traversal, rather than its first N files.
use super::types::FileEntry;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;
pub fn get_large_files(
    root: &Path,
    min_bytes: u64,
    limit: usize,
    excludes: Vec<PathBuf>,
) -> Vec<FileEntry> {
    let limit = limit.clamp(1, 5000);
    let mut files = BTreeMap::new();
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !super::scanner::is_excluded(e.path(), &excludes))
    {
        let e = match entry {
            Ok(e) => e,
            Err(_) => {
                super::task::skip();
                continue;
            }
        };
        if !super::task::visit(e.path()) {
            break;
        }
        if !e.file_type().is_file() {
            continue;
        }
        let meta = match e.metadata() {
            Ok(m) => m,
            Err(_) => {
                super::task::skip();
                continue;
            }
        };
        if meta.len() < min_bytes {
            continue;
        }
        let p = e.path();
        let f = FileEntry {
            protected_reason: super::safety::validate(p, &excludes).err(),
            path: p.to_string_lossy().into_owned(),
            name: e.file_name().to_string_lossy().into_owned(),
            dir: p
                .parent()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
            size_bytes: meta.len(),
            modified: super::duplicates::format_date(&meta),
        };
        super::safety::remember(p);
        files.insert((f.size_bytes, f.path.clone()), f);
        if files.len() > limit {
            files.pop_first();
            super::task::notice("仅展示体积最大的候选文件；权限失败项未计入排序。");
        }
    }
    files.into_values().rev().collect()
}
