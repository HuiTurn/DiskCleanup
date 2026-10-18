//! Durable per-file write-ahead journal. Recovery never guesses by original path alone.
use super::{junk, platform, types::*};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::Write,
    path::PathBuf,
};

fn journal_dir() -> Result<PathBuf, String> {
    #[cfg(test)]
    {
        return Ok(super::test_support::root().join("journals"));
    }
    #[cfg(not(test))]
    Ok(dirs::config_dir()
        .ok_or("无法定位配置目录")?
        .join("diskcleanup")
        .join("cleanup-journals"))
}
/// OS file lock also excludes a concurrent scheduled/headless process. Crash releases it.
pub struct OperationLock {
    _file: File,
}
impl OperationLock {
    pub fn acquire() -> Result<Self, String> {
        let dir = journal_dir()?;
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(dir.join("operation.lock"))
            .map_err(|e| e.to_string())?;
        file.try_lock()
            .map_err(|_| "另一个清理或恢复任务正在运行，请等待完成".to_string())?;
        Ok(Self { _file: file })
    }
}
pub struct Journal {
    file: File,
}
impl Journal {
    pub fn create() -> Result<Self, String> {
        let dir = journal_dir()?;
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let file = OpenOptions::new()
            .create_new(true)
            .append(true)
            .open(dir.join(format!("{stamp}-{}.jsonl", std::process::id())))
            .map_err(|e| e.to_string())?;
        Ok(Self { file })
    }
    #[cfg(any(windows, target_os = "linux", test))]
    fn reopen(path: &std::path::Path) -> Result<Self, String> {
        // Remove an interrupted tail before appending a new complete record.
        let bytes = fs::read(path).map_err(|e| e.to_string())?;
        let valid = bytes
            .iter()
            .rposition(|b| *b == b'\n')
            .map(|n| n + 1)
            .unwrap_or(0);
        let file = OpenOptions::new()
            .append(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        if valid < bytes.len() {
            // Windows 的 append 句柄只有 FILE_APPEND_DATA 权限，SetEndOfFile 需要可写句柄。
            let writable = OpenOptions::new()
                .write(true)
                .open(path)
                .map_err(|e| e.to_string())?;
            writable
                .set_len(valid as u64)
                .and_then(|_| writable.sync_all())
                .map_err(|e| e.to_string())?;
        }
        Ok(Self { file })
    }
    pub fn append(&mut self, item: &CleanupItem) -> Result<(), String> {
        serde_json::to_writer(&mut self.file, item)
            .map_err(|e| format!("清理记录写入失败：{e}"))?;
        self.file
            .write_all(b"\n")
            .and_then(|_| self.file.sync_all())
            .map_err(|e| format!("清理记录写入失败，已停止后续处理：{e}"))
    }
}
fn read_journal(path: &std::path::Path) -> Result<CleanupRecord, String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut items = BTreeMap::new();
    for line in text.split_inclusive('\n') {
        if !line.ends_with('\n') {
            break;
        } // Interrupted final write: keep earlier durable state.
        let item: CleanupItem = serde_json::from_str(line)
            .map_err(|e| format!("清理记录损坏，请手动核对回收站：{e}"))?;
        items.insert(item.path.clone(), item);
    }
    let ts = fs::metadata(path)
        .ok()
        .and_then(|m| m.created().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    Ok(CleanupRecord {
        ts,
        items: items
            .into_values()
            .filter(|i| matches!(i.state.as_str(), "pending" | "trashed"))
            .collect(),
    })
}
fn latest() -> Result<Option<(PathBuf, CleanupRecord)>, String> {
    let dir = journal_dir()?;
    if !dir.exists() {
        return Ok(None);
    }
    let mut files = fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "jsonl"))
        .collect::<Vec<_>>();
    files.sort();
    match files.pop() {
        Some(path) => {
            let record = read_journal(&path)?;
            Ok(Some((path, record)))
        }
        None => Ok(None),
    }
}
pub fn last_record() -> Option<CleanupRecord> {
    latest()
        .ok()
        .flatten()
        .map(|(_, r)| r)
        .filter(|r| !r.items.is_empty())
}

#[cfg(any(windows, target_os = "linux"))]
pub fn trash_ids() -> Result<std::collections::HashSet<String>, String> {
    Ok(trash::os_limited::list()
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter_map(|i| i.id.into_string().ok())
        .collect())
}
#[cfg(not(any(windows, target_os = "linux")))]
pub fn trash_ids() -> Result<std::collections::HashSet<String>, String> {
    Ok(Default::default())
}
pub fn identify(
    path: &std::path::Path,
    before: &std::collections::HashSet<String>,
) -> Option<String> {
    #[cfg(any(windows, target_os = "linux"))]
    {
        let matches = trash::os_limited::list()
            .ok()?
            .into_iter()
            .filter(|i| super::safety::key(&i.original_path()) == super::safety::key(path))
            .filter_map(|i| i.id.into_string().ok())
            .filter(|id| !before.contains(id))
            .collect::<Vec<_>>();
        if matches.len() == 1 {
            matches.into_iter().next()
        } else {
            None
        }
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let _ = (path, before);
        None
    }
}
pub fn restore_last() -> Result<RestoreSummary, String> {
    let _guard = OperationLock::acquire()?;
    #[cfg(any(windows, target_os = "linux"))]
    {
        let (path, record) = latest()?.ok_or("没有可恢复记录；旧版记录请在系统回收站中手动恢复")?;
        if record.items.is_empty() {
            return Err("最近一次清理没有待恢复项目".into());
        }
        let mut journal = Journal::reopen(&path)?;
        let mut result = RestoreSummary::default();
        for mut item in record.items {
            let restore = (|| {
                let id = item
                    .trash_id
                    .as_ref()
                    .ok_or("未能确定本次回收站条目标识，请手动核对；不会按路径猜测恢复")?;
                if std::path::Path::new(&item.path)
                    .try_exists()
                    .map_err(|e| e.to_string())?
                {
                    return Err("原位置已有文件，不会覆盖".into());
                }
                let targets = trash::os_limited::list()
                    .map_err(|e| e.to_string())?
                    .into_iter()
                    .filter(|i| {
                        i.id.to_str() == Some(id.as_str())
                            && super::safety::key(&i.original_path())
                                == super::safety::key(std::path::Path::new(&item.path))
                    })
                    .collect::<Vec<_>>();
                if targets.len() != 1 {
                    return Err("对应回收站条目缺失，保留记录供手动核对".into());
                }
                trash::os_limited::restore_all(targets).map_err(|e| e.to_string())
            })();
            match restore {
                Ok(()) => {
                    result.restored += 1;
                    item.state = "restored".into();
                    item.error = None;
                }
                Err(reason) => {
                    result.failures.push(CleanFailure {
                        path: item.path.clone(),
                        reason: reason.clone(),
                    });
                    item.error = Some(reason);
                }
            }
            journal.append(&item)?;
        }
        Ok(result)
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        Err("macOS 请在废纸篓中选择对应文件并使用「放回原处」".into())
    }
}
pub fn open_trash() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open")
        .arg(dirs::home_dir().ok_or("无法定位用户目录")?.join(".Trash"))
        .spawn();
    #[cfg(windows)]
    let result = std::process::Command::new("explorer")
        .arg("shell:RecycleBinFolder")
        .spawn();
    #[cfg(not(any(windows, target_os = "macos")))]
    let result = std::process::Command::new("xdg-open")
        .arg("trash:///")
        .spawn();
    result.map(|_| ()).map_err(|e| e.to_string())
}
pub fn run_auto_clean() -> Result<CleanSummary, String> {
    let guard = OperationLock::acquire()?;
    let excludes = super::settings::Settings::load_checked()?
        .exclude_dirs
        .into_iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    let cats = junk::collect_excluded(&platform::junk_rules(), &excludes);
    let paths = cats
        .into_iter()
        .flat_map(|c| c.items)
        .filter(|i| i.safe)
        .filter(|i| {
            // Recently modified files may still belong to a running application.
            fs::metadata(&i.path)
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.elapsed().ok())
                .is_some_and(|age| age.as_secs() >= 48 * 3600)
        })
        .map(|i| i.path)
        .collect();
    super::cleaner::execute(paths, guard, |_| {})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_write_keeps_last_durable_state_and_partial_restore_keeps_remaining() {
        let root = super::super::test_support::root();
        let p = root.join("fixture.jsonl");
        let mut j = Journal {
            file: OpenOptions::new()
                .create_new(true)
                .append(true)
                .open(&p)
                .unwrap(),
        };
        let mut a = CleanupItem {
            path: "/a".into(),
            size_bytes: 1,
            state: "trashed".into(),
            trash_id: Some("id-a".into()),
            error: None,
        };
        let b = CleanupItem {
            path: "/b".into(),
            size_bytes: 2,
            state: "trashed".into(),
            trash_id: Some("id-b".into()),
            error: Some("missing".into()),
        };
        j.append(&a).unwrap();
        j.append(&b).unwrap();
        a.state = "restored".into();
        j.append(&a).unwrap();
        j.file.write_all(b"{partial").unwrap();
        let record = read_journal(&p).unwrap();
        assert_eq!(record.items.len(), 1);
        assert_eq!(record.items[0].path, "/b");
        drop(j);
        let mut repaired = Journal::reopen(&p).unwrap();
        repaired.append(&b).unwrap();
        assert_eq!(read_journal(&p).unwrap().items.len(), 1);
    }
    #[test]
    fn operation_lock_rejects_second_job_and_releases_on_drop() {
        let guard = OperationLock::acquire().unwrap();
        assert!(OperationLock::acquire().is_err());
        drop(guard);
        assert!(OperationLock::acquire().is_ok());
    }
}
