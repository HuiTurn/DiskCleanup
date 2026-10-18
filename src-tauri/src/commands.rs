//! Tauri IPC 命令层：前端 bridge 的唯一后端入口。

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::engine::{
    autoclean, cleaner, duplicates, history, junk, large_files, list_disks, platform, scanner,
    settings::Settings, types::*,
};
use crate::AppState;

#[tauri::command]
pub fn get_platform() -> String {
    std::env::consts::OS.to_string()
}

#[tauri::command]
pub async fn get_disks() -> Result<Vec<DiskInfo>, String> {
    background(list_disks).await
}

#[tauri::command]
pub fn start_scan(app: AppHandle, state: State<AppState>, root: Option<String>) -> Result<(), String> {
    let root = root
        .map(PathBuf::from)
        .unwrap_or_else(platform::system_volume);
    let excludes: Vec<PathBuf> = state
        .settings
        .lock()
        .unwrap()
        .exclude_dirs
        .iter()
        .map(PathBuf::from)
        .collect();

    let mut status = state.scan.status.lock().unwrap();
    if status.running {
        return Err("已有扫描正在运行，请等待完成或取消后重试".into());
    }
    let cancel = Arc::new(AtomicBool::new(false));
    let result_slot = state.scan.result.clone();
    *result_slot.lock().unwrap() = None;
    *status = ScanStatus {
        running: true,
        current_path: root.to_string_lossy().to_string(),
        ..Default::default()
    };
    *state.scan.cancel.lock().unwrap() = Some(cancel.clone());
    drop(status);
    scanner::spawn_scan(app, root, cancel, result_slot, state.scan.status.clone(), excludes);
    Ok(())
}

#[tauri::command]
pub fn cancel_scan(state: State<AppState>) {
    if let Some(c) = state.scan.cancel.lock().unwrap().as_ref() {
        c.store(true, Ordering::Relaxed);
    }
}

#[tauri::command]
pub fn get_scan_status(state: State<AppState>) -> ScanStatus {
    state.scan.status.lock().unwrap().clone()
}

#[tauri::command]
pub fn get_scan_result(state: State<AppState>) -> Option<ScanResult> {
    state.scan.result.lock().unwrap().clone()
}

#[tauri::command]
pub async fn get_junk_categories(state: State<'_, AppState>) -> Result<Vec<JunkCategory>, String> {
    let excludes = state.settings.lock().unwrap().exclude_dirs.iter().map(PathBuf::from).collect::<Vec<_>>();
    let guard = crate::engine::task::start("junk")?;
    background(move || guard.run(|| junk::collect_excluded(&platform::junk_rules(), &excludes))).await?
}

#[tauri::command]
pub fn clean_junk(app: AppHandle, paths: Vec<String>) -> Result<(), String> {
    cleaner::clean_paths(app, paths)
}

#[tauri::command]
pub async fn get_large_files(
    state: State<'_, AppState>,
    root: Option<String>,
    min_mb: f64,
    limit: Option<usize>,
) -> Result<Vec<FileEntry>, String> {
    let root = root
        .map(PathBuf::from)
        .unwrap_or_else(platform::system_volume);
    let excludes: Vec<PathBuf> = state
        .settings
        .lock()
        .unwrap()
        .exclude_dirs
        .iter()
        .map(PathBuf::from)
        .collect();
    if !min_mb.is_finite() || min_mb < 0.0 { return Err("无效的大小阈值".into()); }
    std::fs::read_dir(&root).map_err(|e| format!("无法访问扫描目录：{e}"))?;
    let guard = crate::engine::task::start("large")?;
    background(move || guard.run(|| large_files::get_large_files(&root, (min_mb * 1024.0 * 1024.0) as u64, limit.unwrap_or(500), excludes))).await?
}

#[tauri::command]
pub async fn find_duplicates(state: State<'_, AppState>, root: Option<String>) -> Result<Vec<DuplicateGroup>, String> {
    let root = root
        .map(PathBuf::from)
        .unwrap_or_else(platform::system_volume);
    let excludes: Vec<PathBuf> = state
        .settings
        .lock()
        .unwrap()
        .exclude_dirs
        .iter()
        .map(PathBuf::from)
        .collect();
    std::fs::read_dir(&root).map_err(|e| format!("无法访问扫描目录：{e}"))?;
    let guard = crate::engine::task::start("duplicates")?;
    background(move || guard.run(|| duplicates::find_duplicates(&root, 1024 * 1024, 60, excludes))).await?
}

#[tauri::command]
pub fn get_system_tools() -> Vec<ToolInfo> {
    platform::system_tools()
}

#[tauri::command]
pub fn run_system_tool(id: String) -> Result<Option<String>, String> {
    platform::run_tool(&id)
}

#[tauri::command]
pub async fn get_launch_items() -> Result<Vec<LaunchItem>, String> {
    background(platform::launch_items).await
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
pub fn save_settings(state: State<AppState>, settings: Settings) -> Result<(), String> {
    let old = state.settings.lock().unwrap().clone();

    // 开机自启变更走平台接口
    if old.launch_at_login != settings.launch_at_login {
        platform::set_launch_at_login(settings.launch_at_login)?;
    }
    // 自动清理：开关或周期变化则重新注册系统定时任务
    if old.auto_clean_enabled != settings.auto_clean_enabled
        || old.auto_clean_interval_days != settings.auto_clean_interval_days
    {
        platform::schedule_auto_clean(settings.auto_clean_enabled, settings.auto_clean_interval_days)?;
    }

    settings.save()?;
    *state.settings.lock().unwrap() = settings;
    Ok(())
}

#[tauri::command]
pub fn get_auto_clean_status() -> bool {
    platform::auto_clean_scheduled()
}

/// Runs off the window thread and returns per-file failures.
#[tauri::command]
pub async fn run_auto_clean_now(state: State<'_, AppState>) -> Result<CleanSummary, String> {
    let result = background(autoclean::run_auto_clean).await??;
    let mut settings = state.settings.lock().unwrap();
    settings.last_auto_clean_ts = Some(now_ts());
    settings.save()?;
    Ok(result)
}

#[tauri::command]
pub fn get_task_status(kind: String) -> crate::engine::task::Status { crate::engine::task::status(&kind) }
#[tauri::command]
pub fn cancel_task(kind: String) { crate::engine::task::cancel(&kind); }

#[tauri::command]
pub fn get_scan_history() -> Vec<Snapshot> {
    history::list()
}

#[tauri::command]
pub async fn restore_last_cleanup() -> Result<RestoreSummary, String> {
    background(autoclean::restore_last).await?
}

#[tauri::command]
pub fn open_trash() -> Result<(), String> {
    autoclean::open_trash()
}

#[tauri::command]
pub fn last_cleanup_record() -> Option<CleanupRecord> {
    autoclean::last_record()
}

#[tauri::command]
pub fn close_window(window: tauri::WebviewWindow) -> Result<(), String> {
    if crate::engine::migration::is_busy() { return Err("目录迁移中，请等待完成后退出".into()); }
    window.close().map_err(|e| e.to_string())
}

fn now_ts() -> u64 {
    use std::time::UNIX_EPOCH;
    std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// File traversal and platform subprocesses must never run on the window thread.
async fn background<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work).await.map_err(|e| e.to_string())
}

#[cfg(test)]
mod responsiveness_tests {
    use super::background;

    #[test]
    fn disk_work_runs_off_the_calling_thread() {
        let caller = std::thread::current().id();
        let worker = tauri::async_runtime::block_on(background(|| std::thread::current().id())).unwrap();
        assert_ne!(caller, worker);
    }

    #[test]
    fn blocked_disk_work_does_not_block_other_commands() {
        use std::sync::mpsc;
        use std::time::Duration;
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let slow = tauri::async_runtime::spawn(background(move || {
            started_tx.send(()).unwrap();
            release_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        }));
        started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let (ready_tx, ready_rx) = mpsc::channel();
        tauri::async_runtime::spawn(async move { ready_tx.send(()).unwrap(); });
        let responsive = ready_rx.recv_timeout(Duration::from_secs(2));
        release_tx.send(()).unwrap();
        tauri::async_runtime::block_on(slow).unwrap().unwrap();
        assert!(responsive.is_ok());
    }
}

/// Native folder picker; cancellation leaves the exclusion list unchanged.
#[tauri::command]
pub async fn pick_directory(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog().file().set_title("选择排除目录").blocking_pick_folder()
            .map(|path| path.into_path().map(|p| p.to_string_lossy().into_owned()).map_err(|e| e.to_string()))
            .transpose()
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_migration_options() -> Result<crate::engine::migration::MigrationOptions, String> {
    background(crate::engine::migration::options).await?
}
#[tauri::command]
pub async fn prepare_migration(request: crate::engine::migration::MigrationRequest) -> Result<crate::engine::migration::MigrationPlan, String> {
    let guard = crate::engine::task::start("migration-check")?;
    background(move || guard.run(|| crate::engine::migration::prepare(request))).await??
}
#[tauri::command]
pub async fn migrate_folder(request: crate::engine::migration::MigrationRequest) -> Result<crate::engine::migration::MigrationStatus, String> {
    let guard = crate::engine::migration::MigrationGuard::acquire()?;
    background(move || crate::engine::migration::execute(request, guard)).await?
}
#[tauri::command]
pub fn get_migration_status() -> crate::engine::migration::MigrationStatus {
    crate::engine::migration::status()
}
