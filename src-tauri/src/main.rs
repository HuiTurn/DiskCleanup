#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // 无头自动清理模式（由 launchd / 计划任务调用）
    if std::env::args().any(|a| a == "--auto-clean") {
        diskcleanup_lib::run_auto_clean_headless();
    }
    diskcleanup_lib::run()
}
