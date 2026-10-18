//! Known-folder relocation. All destructive work is delegated to Windows Shell.
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

#[cfg(any(windows, test))]
mod validation;
#[cfg(windows)]
mod windows;

pub static BUSY: AtomicBool = AtomicBool::new(false);
static STATUS: Mutex<MigrationStatus> = Mutex::new(MigrationStatus {
    phase: String::new(),
    message: String::new(),
    source: String::new(),
    target: String::new(),
    journal: String::new(),
});

#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MigrationStatus {
    pub phase: String,
    pub message: String,
    pub source: String,
    pub target: String,
    pub journal: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationFolder {
    pub id: String,
    pub name: String,
    pub path: String,
    pub blocked_reason: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationDrive {
    pub path: String,
    pub available_bytes: u64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationOptions {
    pub supported: bool,
    pub folders: Vec<MigrationFolder>,
    pub drives: Vec<MigrationDrive>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationRequest {
    pub folder_id: String,
    pub target_drive: String,
    pub expected_source: String,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationPlan {
    pub request: MigrationRequest,
    pub name: String,
    pub source: String,
    pub target: String,
    pub total_bytes: u64,
    pub file_count: u64,
    pub available_bytes: u64,
}

pub fn status() -> MigrationStatus {
    let mut current = STATUS.lock().unwrap_or_else(|e| e.into_inner());
    #[cfg(windows)]
    if current.phase.is_empty() {
        if let Some(previous) = windows::previous_status() {
            *current = previous;
        }
    }
    if !is_busy() && matches!(current.phase.as_str(), "checking" | "moving") {
        current.phase = "failed".into();
        current.message = "迁移任务意外结束，请按记录核对原目录与目标目录。".into();
    }
    current.clone()
}
pub fn is_busy() -> bool {
    BUSY.load(Ordering::SeqCst)
}
pub fn options() -> Result<MigrationOptions, String> {
    #[cfg(windows)]
    {
        windows::options()
    }
    #[cfg(not(windows))]
    {
        Ok(MigrationOptions {
            supported: false,
            folders: vec![],
            drives: vec![],
        })
    }
}
pub fn prepare(request: MigrationRequest) -> Result<MigrationPlan, String> {
    if is_busy() {
        return Err("已有目录正在迁移，请等待完成".into());
    }
    #[cfg(windows)]
    {
        windows::prepare(request)
    }
    #[cfg(not(windows))]
    {
        let _ = request;
        Err("系统目录迁移目前仅支持 Windows".into())
    }
}

/// The guard is acquired before spawning, so closing cannot race task startup.
pub struct MigrationGuard { _operation: super::autoclean::OperationLock }
impl MigrationGuard {
    pub fn acquire() -> Result<Self, String> {
        let operation = super::autoclean::OperationLock::acquire()?;
        BUSY.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map_err(|_| "已有目录正在迁移，请等待完成".to_string())?;
        *STATUS.lock().unwrap_or_else(|e| e.into_inner()) = MigrationStatus {
            phase: "checking".into(),
            message: "正在重新检查目录与磁盘空间…".into(),
            ..Default::default()
        };
        Ok(Self { _operation: operation })
    }
}
impl Drop for MigrationGuard {
    fn drop(&mut self) {
        BUSY.store(false, Ordering::SeqCst);
    }
}
pub fn execute(
    request: MigrationRequest,
    _guard: MigrationGuard,
) -> Result<MigrationStatus, String> {
    #[cfg(windows)]
    let result = windows::execute(request);
    #[cfg(not(windows))]
    let result: Result<(), String> = {
        let _ = request;
        Err("系统目录迁移目前仅支持 Windows".into())
    };
    let mut status = STATUS.lock().unwrap_or_else(|e| e.into_inner());
    match result {
        Ok(()) => {
            status.phase = "complete".into();
            status.message = "迁移完成，Windows 已使用新的目录位置。".into();
        }
        Err(error) => {
            status.phase = "failed".into();
            status.message = error;
        }
    }
    Ok(status.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn migration_guard_prevents_concurrent_jobs_and_releases_on_drop() {
        let guard = MigrationGuard::acquire().unwrap();
        assert!(is_busy());
        assert!(MigrationGuard::acquire().is_err());
        assert!(prepare(MigrationRequest {
            folder_id: "desktop".into(),
            target_drive: "D:\\".into(),
            expected_source: "C:\\Desktop".into()
        })
        .is_err());
        drop(guard);
        assert!(!is_busy());
    }
}
