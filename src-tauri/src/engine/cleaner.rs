//! No directory deletes. Every selected file is revalidated immediately before trashing.
use super::{
    autoclean::{Journal, OperationLock},
    types::*,
};
use serde::Serialize;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    time::Instant,
};
use tauri::{AppHandle, Emitter};
#[derive(Serialize, Clone)]
pub struct CleanFinished {
    pub summary: CleanSummary,
}
pub fn clean_paths(app: AppHandle, paths: Vec<String>) -> Result<(), String> {
    let guard = OperationLock::acquire()?;
    std::thread::spawn(move || {
        let summary = execute(paths, guard, |progress| {
            let _ = app.emit("clean-progress", progress);
        })
        .unwrap_or_else(|e| CleanSummary {
            failures: vec![CleanFailure {
                path: "清理任务".into(),
                reason: e,
            }],
            ..Default::default()
        });
        let _ = app.emit("clean-finished", CleanFinished { summary });
    });
    Ok(())
}
pub fn execute(
    paths: Vec<String>,
    _guard: OperationLock,
    progress: impl FnMut(CleanProgress),
) -> Result<CleanSummary, String> {
    execute_with(paths, _guard, progress, |path| {
        let before = super::autoclean::trash_ids()?;
        trash::delete(path).map_err(|e| e.to_string())?;
        Ok(super::autoclean::identify(path, &before))
    })
}
fn execute_with(
    paths: Vec<String>,
    _guard: OperationLock,
    mut progress: impl FnMut(CleanProgress),
    mut recycle: impl FnMut(&Path) -> Result<Option<String>, String>,
) -> Result<CleanSummary, String> {
    let started = Instant::now();
    let mut summary = CleanSummary::default();
    let mut journal = Journal::create()?;
    let mut seen = HashSet::new();
    let paths = paths
        .into_iter()
        .filter(|p| seen.insert(super::safety::key(Path::new(p))))
        .collect::<Vec<_>>();
    // Capture reviewed fingerprints once, so concurrent rescans cannot replace this plan.
    let planned = paths
        .iter()
        .map(|p| super::safety::reviewed(Path::new(p)))
        .collect::<Vec<_>>();
    for (index, (p, expected)) in paths.iter().zip(planned).enumerate() {
        let path = Path::new(p);
        let operation = (|| {
            let expected = expected?;
            let excludes = super::settings::Settings::load_checked()?
                .exclude_dirs
                .into_iter()
                .map(PathBuf::from)
                .collect::<Vec<_>>();
            super::safety::revalidate(path, &expected, &excludes)?;
            let mut item = CleanupItem {
                path: p.clone(),
                size_bytes: expected.len,
                state: "pending".into(),
                trash_id: None,
                error: None,
            };
            journal.append(&item)?;
            super::safety::revalidate(path, &expected, &excludes)?;
            match recycle(path) {
                Ok(id) => {
                    summary.files += 1;
                    summary.moved_bytes += expected.len;
                    item.state = "trashed".into();
                    item.trash_id = id;
                }
                Err(e) => {
                    item.state = "failed".into();
                    item.error = Some(e.to_string());
                }
            }
            journal.append(&item)?;
            if let Some(error) = item.error {
                return Err(error);
            }
            Ok(())
        })();
        if let Err(reason) = operation {
            let stop = reason.contains("记录写入失败");
            summary.failures.push(CleanFailure {
                path: p.clone(),
                reason,
            });
            if stop {
                for remaining in paths.iter().skip(index + 1) {
                    summary.failures.push(CleanFailure {
                        path: remaining.clone(),
                        reason: "记录写入失败，未执行".into(),
                    });
                }
                break;
            }
        }
        progress(CleanProgress {
            done: index + 1,
            total: paths.len(),
            path: p.clone(),
            freed_bytes: 0,
        });
    }
    summary.duration_ms = started.elapsed().as_millis() as u64;
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    fn fixture() -> PathBuf {
        let root = super::super::test_support::root();
        let mut settings = super::super::settings::Settings::default();
        settings.exclude_dirs = vec![];
        settings.save().unwrap();
        root
    }
    #[test]
    fn duplicate_paths_move_once_and_never_report_recycled_bytes_as_free_space() {
        let root = fixture();
        let path = root.join("photo.txt");
        fs::write(&path, b"hello").unwrap();
        super::super::safety::remember(&path);
        let mut calls = 0;
        let result = execute_with(
            vec![path.display().to_string(), path.display().to_string()],
            OperationLock::acquire().unwrap(),
            |_| {},
            |p| {
                calls += 1;
                let pending = super::super::autoclean::last_record().unwrap();
                assert_eq!(
                    pending.items[0].state, "pending",
                    "write-ahead journal must exist before mutation"
                );
                fs::rename(p, root.join("simulated-trash")).unwrap();
                Ok(Some("unique-test-id".into()))
            },
        )
        .unwrap();
        assert_eq!(calls, 1);
        assert_eq!(result.files, 1);
        assert_eq!(result.moved_bytes, 5);
        assert_eq!(result.freed_bytes, 0);
        let record = super::super::autoclean::last_record().unwrap();
        assert_eq!(record.items[0].trash_id.as_deref(), Some("unique-test-id"));
    }
    #[test]
    fn changed_excluded_sensitive_and_directory_requests_never_reach_delete() {
        let root = fixture();
        let changed = root.join("changed.txt");
        let excluded = root.join("keep.txt");
        let disk = root.join("ext4.vhdx");
        for p in [&changed, &excluded, &disk] {
            fs::write(p, b"old").unwrap();
            super::super::safety::remember(p);
        }
        fs::write(&changed, b"changed data").unwrap();
        let mut settings = super::super::settings::Settings::default();
        settings.exclude_dirs = vec![excluded.display().to_string()];
        settings.save().unwrap();
        let paths = [&changed, &excluded, &disk, &root]
            .into_iter()
            .map(|p| p.display().to_string())
            .collect();
        let result = execute_with(
            paths,
            OperationLock::acquire().unwrap(),
            |_| {},
            |_| panic!("protected selection must not reach delete"),
        )
        .unwrap();
        assert_eq!(result.files, 0);
        assert_eq!(result.failures.len(), 4);
        assert!(changed.exists() && excluded.exists() && disk.exists());
    }
    #[test]
    fn permission_errors_preserve_successes_and_report_exact_path() {
        let root = fixture();
        let good = root.join("good.txt");
        let locked = root.join("locked.txt");
        for p in [&good, &locked] {
            fs::write(p, b"x").unwrap();
            super::super::safety::remember(p);
        }
        let result = execute_with(
            vec![good.display().to_string(), locked.display().to_string()],
            OperationLock::acquire().unwrap(),
            |_| {},
            |p| {
                if p == locked {
                    return Err("permission denied / file in use".into());
                }
                fs::rename(p, root.join("simulated-trash")).unwrap();
                Ok(None)
            },
        )
        .unwrap();
        assert_eq!(result.files, 1);
        assert_eq!(result.failures.len(), 1);
        assert_eq!(result.failures[0].path, locked.display().to_string());
        assert!(locked.exists());
        assert_eq!(
            super::super::autoclean::last_record().unwrap().items.len(),
            1
        );
    }
    #[test]
    fn damaged_settings_fail_closed() {
        let root = fixture();
        let path = root.join("a.txt");
        fs::write(&path, b"x").unwrap();
        super::super::safety::remember(&path);
        fs::write(root.join("settings.json"), b"invalid json").unwrap();
        let result = execute_with(
            vec![path.display().to_string()],
            OperationLock::acquire().unwrap(),
            |_| {},
            |_| panic!("invalid exclusions must stop deletion"),
        )
        .unwrap();
        assert_eq!(result.failures.len(), 1);
        assert!(path.exists());
    }
}
