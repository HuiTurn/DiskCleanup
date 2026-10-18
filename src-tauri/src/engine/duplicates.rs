//! 重复文件检测：体积预筛 -> 前 8KB 哈希粗筛 -> 全量哈希确认。

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use super::types::{DuplicateGroup, FileEntry};

fn classify_group(files: &[FileEntry]) -> String {
    let ext = files[0]
        .name
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    let cat: &str = match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "heic" | "bmp" | "tiff" => "图片",
        "mp4" | "mov" | "avi" | "mkv" | "wmv" => "视频",
        "mp3" | "wav" | "aac" | "flac" | "m4a" => "音频",
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "txt" | "md" => "文档",
        "zip" | "rar" | "7z" | "tar" | "gz" | "dmg" => "压缩包",
        _ => "其他",
    };
    cat.to_string()
}

fn hash_head(path: &Path, head: u64) -> Option<[u8; 32]> {
    let f = File::open(path).ok()?;
    let mut limited = f.take(head);
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65_536];
    loop {
        if super::task::cancelled() {
            return None;
        }
        let n = limited.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Some(hasher.finalize().into())
}

fn hash_full(path: &Path) -> Option<[u8; 32]> {
    let mut f = File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65_536];
    loop {
        if super::task::cancelled() {
            return None;
        }
        match f.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => hasher.update(&buf[..n]),
            Err(_) => return None,
        }
    }
    Some(hasher.finalize().into())
}

fn modified_str(meta: &std::fs::Metadata) -> String {
    use std::time::UNIX_EPOCH;
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| {
            let secs = d.as_secs();
            // 粗略本地化展示（UTC），仅用于列表参考
            let days = secs / 86_400;
            let (y, mo, da) = civil_from_days(days as i64);
            format!("{y:04}-{mo:02}-{da:02}")
        })
        .unwrap_or_else(|| "-".into())
}

/// 供其他模块复用的日期格式化
pub fn format_date(meta: &std::fs::Metadata) -> String {
    modified_str(meta)
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// 文件名归一化：去掉扩展名与常见副本后缀，用于识别 photo.jpg / photo (1).jpg / photo副本.jpg
fn normalize_name(name: &str) -> String {
    let stem = name.rsplit_once('.').map(|(s, _)| s).unwrap_or(name);
    let mut out = stem.to_lowercase();
    const SUFFIX: [&str; 12] = [
        " (1)",
        " (2)",
        " (3)",
        "(1)",
        "(2)",
        "(3)",
        "副本",
        "的副本",
        "-copy",
        " copy",
        "_copy",
        " 副本",
    ];
    for pat in SUFFIX {
        out = out.replace(pat, "");
    }
    out.trim().to_string()
}

/// 文件名相似度 0~1
fn name_similarity(a: &str, b: &str) -> f64 {
    let na = normalize_name(a);
    let nb = normalize_name(b);
    if na.is_empty() || nb.is_empty() {
        return 0.0;
    }
    if na == nb {
        return 1.0;
    }
    if na.starts_with(&nb) || nb.starts_with(&na) {
        let l = na.len().min(nb.len()) as f64;
        let m = na.len().max(nb.len()) as f64;
        return (0.6 + 0.3 * (l / m)).min(1.0);
    }
    let sa: std::collections::HashSet<char> = na.chars().collect();
    let sb: std::collections::HashSet<char> = nb.chars().collect();
    let inter = sa.intersection(&sb).count() as f64;
    let union = sa.union(&sb).count() as f64;
    let j = if union > 0.0 { inter / union } else { 0.0 };
    if j >= 0.75 {
        j * 0.9
    } else {
        0.0
    }
}

/// 片段重合度：按 4KB 采样若干块，取块哈希集合的 Jaccard（位置无关）
fn content_overlap(a: &Path, b: &Path) -> f64 {
    let sa = chunk_hashes(a);
    let sb = chunk_hashes(b);
    if sa.is_empty() || sb.is_empty() {
        return 0.0;
    }
    let inter = sa.intersection(&sb).count() as f64;
    let union = sa.union(&sb).count() as f64;
    if union > 0.0 {
        inter / union
    } else {
        0.0
    }
}

fn chunk_hashes(path: &Path) -> std::collections::HashSet<u64> {
    use std::hash::{DefaultHasher, Hash, Hasher};
    const CHUNK: usize = 4096;
    const MAX_CHUNKS: usize = 512; // 最多采样 2MB
    let mut set = std::collections::HashSet::new();
    let mut f = match File::open(path) {
        Ok(f) => f,
        Err(_) => return set,
    };
    let mut buf = vec![0u8; CHUNK];
    for _ in 0..MAX_CHUNKS {
        if super::task::cancelled() {
            break;
        }
        let n = match Read::read(&mut f, &mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };
        let mut h = DefaultHasher::new();
        buf[..n].hash(&mut h);
        set.insert(h.finish());
    }
    set
}

/// 综合相似度：文件名权重 0.6 + 内容片段重合 0.4
fn combined_similarity(a: &FileEntry, b: &FileEntry) -> f64 {
    let ns = name_similarity(&a.name, &b.name);
    if ns == 0.0 {
        return 0.0;
    }
    let co = content_overlap(Path::new(&a.path), Path::new(&b.path));
    (ns * 0.6 + co * 0.4).min(1.0)
}

/// MinHash 草图：对文件按固定间隔采样若干块，取每块哈希的最小值集合。
/// 用于估算两个文件的 Jaccard 相似度（无需解码图片/文档）。
const SKETCH_SIZE: usize = 32;
const CHUNK: usize = 4096;

#[allow(dead_code)]
fn sketch(path: &Path) -> Option<Vec<u64>> {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let meta = std::fs::metadata(path).ok()?;
    let size = meta.len();
    if size == 0 {
        return None;
    }
    let mut f = File::open(path).ok()?;
    let step = std::cmp::max(CHUNK, (size / (SKETCH_SIZE as u64 * 2)).max(1) as usize);
    let mut out = Vec::with_capacity(SKETCH_SIZE);
    let mut buf = vec![0u8; CHUNK];
    let mut pos: u64 = 0;
    while pos < size && out.len() < SKETCH_SIZE {
        let read = Read::read(&mut f, &mut buf).ok()?;
        if read == 0 {
            break;
        }
        let mut h = DefaultHasher::new();
        buf[..read].hash(&mut h);
        out.push(h.finish());
        pos += read as u64;
        // 跳到下一个采样点
        let next = pos + step as u64 - (pos % step as u64);
        if next > size {
            break;
        }
        f.seek(SeekFrom::Start(next)).ok()?;
        pos = next;
    }
    if out.len() < 4 {
        return None;
    }
    Some(out)
}

/// Jaccard 相似度（草图交集 / 并集）
#[allow(dead_code)]
fn sketch_similarity(a: &[u64], b: &[u64]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut inter = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        if x == y {
            inter += 1;
        }
    }
    inter as f64 / a.len() as f64
}

/// 在候选文件中找「相似」组：
/// 归一化文件名相同/高度相似 + 体积相近（±40%），相似度由文件名与内容片段重合度合成。
fn find_similar(
    files: &[(u64, PathBuf)],
    threshold: f64,
    max_groups: usize,
) -> Vec<DuplicateGroup> {
    // 1) 构造 FileEntry 并按归一化名聚簇
    let entries: Vec<FileEntry> = files
        .iter()
        .filter_map(|(size, p)| {
            let meta = std::fs::metadata(p).ok()?;
            Some(FileEntry {
                protected_reason: super::safety::validate(p, &[]).err(),
                path: p.to_string_lossy().to_string(),
                name: p.file_name()?.to_string_lossy().to_string(),
                dir: p.parent()?.to_string_lossy().to_string(),
                size_bytes: *size,
                modified: modified_str(&meta),
            })
        })
        .collect();

    let mut clusters: HashMap<String, Vec<FileEntry>> = HashMap::new();
    for e in &entries {
        clusters
            .entry(normalize_name(&e.name))
            .or_default()
            .push(e.clone());
    }

    let mut groups: Vec<DuplicateGroup> = Vec::new();
    for (key, members) in clusters {
        if super::task::cancelled() {
            break;
        }
        if groups.len() >= max_groups {
            break;
        }
        if key.is_empty() || members.len() < 2 {
            continue;
        }
        // 体积相近校验：以最大者为基准，允许 ±40%
        let max_size = members.iter().map(|m| m.size_bytes).max().unwrap_or(0);
        let kept: Vec<FileEntry> = members
            .iter()
            .filter(|m| (m.size_bytes as f64) >= max_size as f64 * 0.6)
            .cloned()
            .collect();
        if kept.len() < 2 {
            continue;
        }
        // 与首个成员的相似度作为本组展示值
        let sim = kept[1..]
            .iter()
            .map(|m| combined_similarity(&kept[0], m))
            .fold(1.0f64, f64::min);
        if sim < threshold {
            continue;
        }
        let avg_size = kept.iter().map(|m| m.size_bytes).sum::<u64>() / kept.len() as u64;
        let category = classify_group(&kept);
        groups.push(DuplicateGroup {
            id: format!("s{}", groups.len() + 1),
            size_bytes: avg_size,
            category,
            kind: "similar".into(),
            similarity: (sim * 100.0).round() / 100.0,
            files: kept,
        });
    }
    groups
}

pub fn find_duplicates(
    root: &Path,
    min_size: u64,
    max_groups: usize,
    excludes: Vec<PathBuf>,
) -> Vec<DuplicateGroup> {
    // 1) 收集候选
    let mut by_size: HashMap<u64, Vec<PathBuf>> = HashMap::new();
    let mut candidates = 0usize;
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !super::scanner::is_excluded(e.path(), &excludes))
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => {
                super::task::skip();
                continue;
            }
        };
        if !super::task::visit(entry.path()) {
            break;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        let Ok(meta) = entry.metadata() else {
            super::task::skip();
            continue;
        };
        let size = meta.len();
        if size < min_size {
            continue;
        }
        super::safety::remember(entry.path());
        by_size
            .entry(size)
            .or_default()
            .push(entry.path().to_path_buf());
        candidates += 1;
        if candidates >= 120_000 {
            super::task::notice("候选数量达到上限，结果不完整；请缩小扫描范围。");
            break;
        }
    }

    // 2) 同体积分组，按体积降序处理
    let mut size_groups: Vec<(u64, Vec<PathBuf>)> =
        by_size.into_iter().filter(|(_, v)| v.len() > 1).collect();
    size_groups.sort_by(|a, b| b.0.cmp(&a.0));

    let mut groups: Vec<DuplicateGroup> = Vec::new();
    let mut hashed_budget: u64 = 512 * 1024 * 1024; // 最多全量哈希 512MB

    'outer: for (size, paths) in &size_groups {
        let size = *size;
        if super::task::cancelled() {
            break;
        }
        if groups.len() >= max_groups || hashed_budget < size {
            super::task::notice("达到分组或哈希预算上限，结果不完整；请缩小扫描范围。");
            break;
        }
        // 3) 头部哈希粗筛
        let mut by_head: HashMap<[u8; 32], Vec<PathBuf>> = HashMap::new();
        for p in paths {
            if let Some(h) = hash_head(p, 8192) {
                by_head.entry(h).or_default().push(p.clone());
            }
        }
        for (_, cands) in by_head {
            if cands.len() < 2 {
                continue;
            }
            // 4) 全量哈希确认
            let mut by_full: HashMap<[u8; 32], Vec<PathBuf>> = HashMap::new();
            for p in cands {
                if hashed_budget < size {
                    super::task::notice("达到哈希预算上限，部分候选未验证；请缩小扫描范围。");
                    break 'outer;
                }
                hashed_budget = hashed_budget.saturating_sub(size);
                super::task::visit(&p);
                if let Some(h) = hash_full(&p) {
                    by_full.entry(h).or_default().push(p);
                }
            }
            for (_, files) in by_full {
                if files.len() < 2 {
                    continue;
                }
                let entries: Vec<FileEntry> = files
                    .iter()
                    .filter_map(|p| {
                        let meta = std::fs::metadata(p).ok()?;
                        Some(FileEntry {
                            protected_reason: super::safety::validate(p, &[]).err(),
                            path: p.to_string_lossy().to_string(),
                            name: p
                                .file_name()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_default(),
                            dir: p
                                .parent()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_default(),
                            size_bytes: size,
                            modified: modified_str(&meta),
                        })
                    })
                    .collect();
                if entries.len() < 2 {
                    continue;
                }
                let category = classify_group(&entries);
                groups.push(DuplicateGroup {
                    id: format!("g{}", groups.len() + 1),
                    size_bytes: size,
                    category,
                    kind: "exact".into(),
                    similarity: 1.0,
                    files: entries,
                });
                if groups.len() >= max_groups {
                    break 'outer;
                }
            }
        }
    }

    // 5) 相似文件：对未组成完全相同组的文件做 MinHash 相似度估算
    if groups.len() < max_groups && !super::task::cancelled() {
        let exact_paths: std::collections::HashSet<PathBuf> = groups
            .iter()
            .filter(|g| g.kind == "exact")
            .flat_map(|g| g.files.iter().map(|f| PathBuf::from(&f.path)))
            .collect();
        let remain: Vec<(u64, PathBuf)> = size_groups
            .iter()
            .flat_map(|(size, paths)| paths.iter().map(move |p| (*size, p.clone())))
            .filter(|(size, p)| *size >= min_size && !exact_paths.contains(p))
            .take(3_000)
            .collect();
        if remain.len() == 3_000 {
            super::task::notice("相似文件最多比对 3,000 个候选，结果并非全部。");
        }
        let similar = find_similar(&remain, SIMILAR_THRESHOLD, max_groups - groups.len());
        groups.extend(similar);
    }

    groups.sort_by(|a, b| {
        let rank = |k: &str| if k == "exact" { 0u8 } else { 1u8 };
        rank(&a.kind)
            .cmp(&rank(&b.kind))
            .then(b.size_bytes.cmp(&a.size_bytes))
    });
    groups
}

/// 相似判定阈值（综合相似度）
const SIMILAR_THRESHOLD: f64 = 0.5;
