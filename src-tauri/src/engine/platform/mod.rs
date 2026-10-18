//! 平台适配层：macOS / Windows 差异全部收敛在此。
//! 业务层（engine 其余模块、commands、前端）只依赖本模块暴露的函数。
//! 后续扩展 Linux：新增 linux.rs 并在下方各函数补一个 cfg 分支即可。

use std::path::PathBuf;

use crate::engine::junk::JunkRule;
use crate::engine::types::{LaunchItem, ToolInfo};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

/// 系统盘/系统卷路径
pub fn system_volume() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        return macos::system_volume();
    }
    #[cfg(target_os = "windows")]
    {
        return windows::system_volume();
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        std::env::temp_dir()
    }
}

/// 垃圾清理规则集（Windows 专属概念在 macOS 上映射为等价规则）
pub fn junk_rules() -> Vec<JunkRule> {
    #[cfg(target_os = "macos")]
    {
        return macos::junk_rules();
    }
    #[cfg(target_os = "windows")]
    {
        return windows::junk_rules();
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Vec::new()
    }
}

/// 系统工具宫格（按平台动态生成）
pub fn system_tools() -> Vec<ToolInfo> {
    #[cfg(target_os = "macos")]
    {
        return macos::system_tools();
    }
    #[cfg(target_os = "windows")]
    {
        return windows::system_tools();
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Vec::new()
    }
}

/// 执行系统工具动作，返回 Some(detail) 表示展示型信息（如 SMART 状态）
pub fn run_tool(id: &str) -> Result<Option<String>, String> {
    #[cfg(target_os = "macos")]
    {
        return macos::run_tool(id);
    }
    #[cfg(target_os = "windows")]
    {
        return windows::run_tool(id);
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("unsupported platform".into())
    }
}

/// 开机启动项列表
pub fn launch_items() -> Vec<LaunchItem> {
    #[cfg(target_os = "macos")]
    {
        return macos::launch_items();
    }
    #[cfg(target_os = "windows")]
    {
        return windows::launch_items();
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Vec::new()
    }
}

/// 注册/注销自动清理定时任务
pub fn schedule_auto_clean(enable: bool, interval_days: u32) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        return macos::schedule_auto_clean(enable, interval_days);
    }
    #[cfg(target_os = "windows")]
    {
        return windows::schedule_auto_clean(enable, interval_days);
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("unsupported platform".into())
    }
}

/// 自动清理任务是否已注册
pub fn auto_clean_scheduled() -> bool {
    #[cfg(target_os = "macos")]
    {
        return macos::auto_clean_scheduled();
    }
    #[cfg(target_os = "windows")]
    {
        return windows::auto_clean_scheduled();
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        false
    }
}

/// 设置/取消开机自启
pub fn set_launch_at_login(enable: bool) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        return macos::set_launch_at_login(enable);
    }
    #[cfg(target_os = "windows")]
    {
        return windows::set_launch_at_login(enable);
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("unsupported platform".into())
    }
}
