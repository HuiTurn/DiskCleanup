//! 扫描历史快照：每次扫描完成写入一条，供「历史变化」Tab 展示趋势。

use super::types::Snapshot;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Serialize, Deserialize, Default)]
struct History {
    snapshots: Vec<Snapshot>,
}

fn history_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("diskcleanup").join("history.json"))
}

fn now_ts() -> u64 {
    use std::time::UNIX_EPOCH;
    std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn load() -> History {
    if let Some(p) = history_path() {
        if let Ok(s) = std::fs::read_to_string(&p) {
            if let Ok(h) = serde_json::from_str::<History>(&s) {
                return h;
            }
        }
    }
    History::default()
}

fn save(h: &History) {
    if let Some(p) = history_path() {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(s) = serde_json::to_string_pretty(h) {
            let _ = std::fs::write(&p, s);
        }
    }
}

/// 追加一条快照，最多保留最近 30 条
pub fn push(root: &str, total_bytes: u64, total_files: u64, size_basis: &str) {
    let mut h = load();
    h.snapshots.push(Snapshot {
        size_basis: size_basis.into(),
        ts: now_ts(),
        root: root.to_string(),
        total_bytes,
        total_files,
    });
    if h.snapshots.len() > 30 {
        let extra = h.snapshots.len() - 30;
        h.snapshots.drain(0..extra);
    }
    save(&h);
}

pub fn list() -> Vec<Snapshot> {
    load().snapshots
}

#[allow(dead_code)]
pub fn clear() {
    save(&History::default());
}
