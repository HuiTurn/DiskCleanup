//! Windows 平台实现（仅在 Windows 目标下编译）。
//! 对应截图中的 C盘 / 注册表 / 碎片整理等原生概念。

use std::path::PathBuf;
use std::process::Command;

use super::super::junk::JunkRule;
use super::super::types::{LaunchItem, ToolInfo};

pub fn system_volume() -> PathBuf {
    std::env::var("SystemDrive")
        .map(|d| PathBuf::from(d + "\\"))
        .unwrap_or_else(|_| PathBuf::from("C:\\"))
}

pub fn junk_rules() -> Vec<JunkRule> {
    let mut rules = Vec::new();
    let mut add = |category: &str, name: &str, desc: &str, path: String, safe: bool| {
        rules.push(JunkRule {
            category: category.into(),
            name: name.into(),
            description: desc.into(),
            path: PathBuf::from(path),
            safe,
        });
    };
    let temp = std::env::var("TEMP").unwrap_or_else(|_| "C:\\Windows\\Temp".into());
    let local = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\Users\\Public\\AppData\\Local".into());
    add("系统缓存", "Windows 临时文件", "系统临时目录", temp, true);
    add(
        "系统更新",
        "Windows 更新缓存",
        "Windows 更新下载的安装包",
        "C:\\Windows\\SoftwareDistribution\\Download".into(),
        false,
    );
    add(
        "应用缓存",
        "Chrome 浏览器缓存",
        "Chrome 浏览器产生的缓存文件",
        format!(r"{local}\Google\Chrome\User Data\Default\Cache"),
        true,
    );
    add(
        "应用缓存",
        "Edge 浏览器缓存",
        "Edge 浏览器产生的缓存文件",
        format!(r"{local}\Microsoft\Edge\User Data\Default\Cache"),
        true,
    );
    add(
        "开发缓存",
        "npm 缓存",
        "Node.js 包管理器缓存",
        format!(r"{local}\npm-cache"),
        true,
    );
    add(
        "应用缓存",
        "崩溃转储",
        "应用崩溃产生的 dump 文件",
        format!(r"{local}\CrashDumps"),
        true,
    );
    add(
        "应用缓存",
        "缩略图缓存",
        "资源管理器缩略图缓存",
        format!(r"{local}\Microsoft\Windows\Explorer"),
        true,
    );
    rules
}

pub fn system_tools() -> Vec<ToolInfo> {
    vec![
        ToolInfo {
            id: "startup".into(),
            name: "开机启动管理".into(),
            description: "管理开机自动启动的应用".into(),
            action: "open".into(),
        },
        ToolInfo {
            id: "startup-folders".into(),
            name: "启动项管理".into(),
            description: "管理启动文件夹与计划任务".into(),
            action: "open".into(),
        },
        ToolInfo {
            id: "junk-clean".into(),
            name: "系统文件清理".into(),
            description: "清理系统缓存与临时文件".into(),
            action: "junk-page".into(),
        },
        ToolInfo {
            id: "smart".into(),
            name: "磁盘健康".into(),
            description: "查看磁盘 S.M.A.R.T. 状态".into(),
            action: "smart".into(),
        },
        ToolInfo {
            id: "defrag".into(),
            name: "磁盘碎片整理".into(),
            description: "打开碎片整理与优化工具".into(),
            action: "open".into(),
        },
        ToolInfo {
            id: "about".into(),
            name: "系统信息".into(),
            description: "查看系统版本与硬件信息".into(),
            action: "open".into(),
        },
        ToolInfo {
            id: "update-clean".into(),
            name: "Windows 更新清理".into(),
            description: "清理系统更新下载残留".into(),
            action: "junk-page".into(),
        },
        ToolInfo {
            id: "storage-sense".into(),
            name: "存储感知".into(),
            description: "配置自动清理与存储策略".into(),
            action: "open".into(),
        },
    ]
}

pub fn run_tool(id: &str) -> Result<Option<String>, String> {
    let shell = |cmd: &str| -> Result<Option<String>, String> {
        Command::new("cmd")
            .args(["/C", "start", "", cmd])
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(None)
    };
    match id {
        "startup" => shell("taskmgr.exe"),
        "startup-folders" => shell("shell:startup"),
        "defrag" => shell("dfrgui.exe"),
        "about" => shell("ms-settings:about"),
        "storage-sense" => shell("ms-settings:storagepolicies"),
        "smart" => {
            let out = Command::new("powershell")
                .args(["-NoProfile", "-Command", "Get-PhysicalDisk | Select-Object -First 1 HealthStatus | Format-List"])
                .output()
                .map_err(|e| e.to_string())?;
            let text = String::from_utf8_lossy(&out.stdout);
            Ok(Some(text.trim().to_string()))
        }
        _ => Err(format!("unknown tool: {id}")),
    }
}

/// 注册/注销自动清理定时任务（schtasks）
pub fn schedule_auto_clean(enable: bool, interval_days: u32) -> Result<(), String> {
    let task = "DiskCleanupAutoClean";
    let _ = Command::new("schtasks")
        .args(["/delete", "/tn", task, "/f"])
        .output();
    if !enable {
        return Ok(());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let tr = format!("\"{}\" --auto-clean", exe.to_string_lossy());
    let mo = interval_days.max(1).to_string();
    let out = Command::new("schtasks")
        .args(["/create", "/tn", task, "/sc", "daily", "/mo", &mo, "/tr", &tr, "/f"])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).to_string())
    }
}

pub fn auto_clean_scheduled() -> bool {
    Command::new("schtasks")
        .args(["/query", "/tn", "DiskCleanupAutoClean"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn launch_items() -> Vec<LaunchItem> {
    let out = Command::new("reg")
        .args([
            "query",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
        ])
        .output();
    match out {
        Ok(o) => {
            let s = String::from_utf8_lossy(&o.stdout);
            s.lines()
                .filter(|l| l.contains("REG_SZ"))
                .filter_map(|l| l.split_whitespace().next())
                .map(|name| LaunchItem { name: name.to_string() })
                .collect()
        }
        Err(_) => Vec::new(),
    }
}

pub fn set_launch_at_login(enable: bool) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let key = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
    let out = if enable {
        Command::new("reg")
            .args(["add", key, "/v", "DiskCleanup", "/t", "REG_SZ", "/d"])
            .arg(&exe)
            .output()
            .map_err(|e| e.to_string())?
    } else {
        Command::new("reg")
            .args(["delete", key, "/v", "DiskCleanup", "/f"])
            .output()
            .map_err(|e| e.to_string())?
    };
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).to_string())
    }
}
