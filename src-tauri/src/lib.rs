mod commands;
mod engine;

use std::sync::Mutex;

use engine::scanner::ScanState;
use engine::settings::Settings;
use tauri::{Emitter, Manager};

pub struct AppState {
    pub scan: ScanState,
    pub settings: Mutex<Settings>,
}

/// 二进制名（用于 launchd 定位 .app 内可执行文件）
#[allow(dead_code)]
pub fn binary_name() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|s| s.to_string_lossy().to_string()))
        .unwrap_or_else(|| "diskcleanup".into())
}

/// 无头自动清理（launchd / 计划任务调用）
pub fn run_auto_clean_headless() -> ! {
    match engine::autoclean::run_auto_clean() {
        Ok(summary) => {
            let mut settings = Settings::load();
            settings.last_auto_clean_ts = Some(now_ts());
            if let Err(e) = settings.save() { eprintln!("{e}"); }
            println!("{}", serde_json::to_string(&summary).unwrap_or_default());
            std::process::exit(if summary.failures.is_empty() { 0 } else { 1 });
        }
        Err(e) => { eprintln!("{e}"); std::process::exit(1); }
    }
}

fn now_ts() -> u64 {
    use std::time::UNIX_EPOCH;
    std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn run() {
    let settings = Settings::load();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            scan: ScanState::default(),
            settings: Mutex::new(settings),
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_platform,
            commands::get_task_status,
            commands::cancel_task,
            commands::get_migration_options,
            commands::prepare_migration,
            commands::migrate_folder,
            commands::get_migration_status,
            commands::pick_directory,
            commands::get_disks,
            commands::start_scan,
            commands::cancel_scan,
            commands::get_scan_status,
            commands::get_scan_result,
            commands::get_junk_categories,
            commands::clean_junk,
            commands::get_large_files,
            commands::find_duplicates,
            commands::get_system_tools,
            commands::run_system_tool,
            commands::get_launch_items,
            commands::get_settings,
            commands::save_settings,
            commands::get_auto_clean_status,
            commands::run_auto_clean_now,
            commands::get_scan_history,
            commands::restore_last_cleanup,
            commands::open_trash,
            commands::last_cleanup_record,
            commands::close_window,
        ])
        .on_window_event(|window, event| {
            // 退出前确认：拦截关闭请求，交由前端弹窗确认
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if engine::migration::is_busy() {
                    api.prevent_close();
                    let _ = window.emit("migration-close-blocked", ());
                    return;
                }
                if let Some(state) = window.try_state::<AppState>() {
                    let need_confirm = state.settings.lock().map(|s| s.close_confirm).unwrap_or(false);
                    if need_confirm {
                        api.prevent_close();
                        let _ = window.emit("close-requested", ());
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests;
