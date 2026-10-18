//! 垃圾清理规则：按平台加载规则集（由 platform 层提供），统计体积后归类返回。

use std::path::PathBuf;

use super::types::{JunkCategory, JunkItem};

#[derive(Clone)]
pub struct JunkRule {
    pub category: String,
    pub name: String,
    pub description: String,
    pub path: PathBuf,
    /// 是否默认勾选（相对安全的缓存类）
    pub safe: bool,
}

/// 展开 ~/.x 形式的路径
pub fn expand(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix('~') {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest.trim_start_matches('/'));
        }
    }
    PathBuf::from(p)
}

/// Emit disjoint, individually selectable files; never delete a rule's directory.
#[cfg(test)]
pub fn collect(rules: &[JunkRule]) -> Vec<JunkCategory> {
    collect_excluded(rules, &[])
}
pub fn collect_excluded(rules: &[JunkRule], excludes: &[PathBuf]) -> Vec<JunkCategory> {
    let mut map: std::collections::BTreeMap<String, JunkCategory> =
        std::collections::BTreeMap::new();
    let mut seen = std::collections::HashSet::new();
    // More-specific rules own their files, even when a parent rule also exists.
    let mut rules: Vec<_> = rules.iter().collect();
    rules.sort_by_key(|r| std::cmp::Reverse(r.path.components().count()));
    'rules: for rule in rules {
        if !rule.path.exists() || super::safety::excluded(&rule.path, excludes) {
            continue;
        }
        for entry in walkdir::WalkDir::new(&rule.path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !super::safety::excluded(e.path(), excludes))
        {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => {
                    super::task::skip();
                    continue;
                }
            };
            if !super::task::visit(entry.path()) {
                break 'rules;
            }
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if !seen.insert(super::safety::key(path)) {
                continue;
            }
            if rule.name == "缩略图缓存" {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if !(name.starts_with("thumbcache_") && name.ends_with(".db")) {
                    continue;
                }
            }
            let meta = match super::safety::validate(path, excludes) {
                Ok(m) => m,
                Err(_) => {
                    super::task::skip();
                    continue;
                }
            };
            let size = meta.len;
            if size == 0 {
                continue;
            }
            let path_string = path.to_string_lossy().into_owned();
            super::safety::remember(path);
            let cat = map
                .entry(rule.category.clone())
                .or_insert_with(|| JunkCategory {
                    id: rule.category.clone(),
                    name: rule.category.clone(),
                    total_bytes: 0,
                    items: vec![],
                });
            cat.total_bytes += size;
            cat.items.push(JunkItem {
                id: path_string.clone(),
                name: entry.file_name().to_string_lossy().into_owned(),
                path: path_string,
                description: format!("{} · {}", rule.name, rule.description),
                size_bytes: size,
                safe: rule.safe,
            });
            if map.values().map(|c| c.items.len()).sum::<usize>() >= 10_000 {
                super::task::notice(
                    "本次最多列出 10,000 个候选文件；结果并非全部，处理后可重新扫描。",
                );
                break 'rules;
            }
        }
    }
    map.into_values()
        .map(|mut c| {
            c.items.sort_by_key(|i| std::cmp::Reverse(i.size_bytes));
            c
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overlapping_rules_and_excluded_children_produce_disjoint_leaf_candidates() {
        let root = super::super::test_support::root();
        std::fs::create_dir(root.join("browser")).unwrap();
        std::fs::write(root.join("general.txt"), b"one").unwrap();
        std::fs::write(root.join("browser/keep.txt"), b"keep").unwrap();
        std::fs::write(root.join("browser/old.txt"), b"old").unwrap();
        let rules = vec![
            JunkRule {
                category: "general".into(),
                name: "parent".into(),
                description: "".into(),
                path: root.clone(),
                safe: false,
            },
            JunkRule {
                category: "browser".into(),
                name: "browser".into(),
                description: "".into(),
                path: root.join("browser"),
                safe: true,
            },
        ];
        let cats = collect_excluded(&rules, &[root.join("browser/keep.txt")]);
        let items = cats.iter().flat_map(|c| &c.items).collect::<Vec<_>>();
        assert_eq!(items.len(), 2);
        assert_eq!(items.iter().map(|i| i.size_bytes).sum::<u64>(), 6);
        assert!(items
            .iter()
            .all(|i| std::path::Path::new(&i.path).is_file()));
        assert_eq!(
            cats.iter().find(|c| c.id == "general").unwrap().items[0].name,
            "general.txt"
        );
        let excluded = collect_excluded(&rules, &[root.join("browser")]);
        assert_eq!(excluded.iter().flat_map(|c| &c.items).count(), 1);
        assert!(root.join("browser/keep.txt").exists());
    }
}
