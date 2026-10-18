//! macOS 平台实现：将截图中 Windows 专属工具映射为等价功能。

use std::path::PathBuf;
use std::process::Command;

use super::super::junk::JunkRule;
use super::super::types::{LaunchItem, ToolInfo};

pub fn system_volume() -> PathBuf {
    PathBuf::from("/")
}

pub fn junk_rules() -> Vec<JunkRule> {
    let mut rules = Vec::new();
    let mut add = |category: &str, name: &str, desc: &str, path: &str, safe: bool| {
        rules.push(JunkRule {
            category: category.into(),
            name: name.into(),
            description: desc.into(),
            path: crate::engine::junk::expand(path),
            safe,
        });
    };
    // 对应截图 04：系统缓存 / 应用缓存 / 日志文件 / 开发缓存 / 废纸篓
    add("系统缓存", "系统缓存", "其他应用缓存，仅手动选择", "~/Library/Caches", false);
    add("应用缓存", "应用日志", "应用程序运行日志", "~/Library/Logs", true);
    add(
        "应用缓存",
        "Chrome 浏览器缓存",
        "Chrome 浏览器产生的缓存文件",
        "~/Library/Caches/Google/Chrome",
        true,
    );
    add(
        "应用缓存",
        "Firefox 浏览器缓存",
        "Firefox 浏览器产生的缓存文件",
        "~/Library/Caches/Firefox",
        true,
    );
    add(
        "开发缓存",
        "npm 缓存",
        "Node.js 包管理器缓存",
        "~/.npm/_cacache",
        true,
    );
    add(
        "开发缓存",
        "pip 缓存",
        "Python 包管理器缓存",
        "~/Library/Caches/pip",
        true,
    );
    add(
        "开发缓存",
        "Xcode DerivedData",
        "Xcode 编译中间产物，请先关闭 Xcode",
        "~/Library/Developer/Xcode/DerivedData",
        false,
    );
    add(
        "开发缓存",
        "Cargo registry 缓存",
        "Rust 包注册表缓存",
        "~/.cargo/registry/cache",
        true,
    );

    add(
        "系统缓存",
        "系统更新残留",
        "macOS 软件更新下载的安装包",
        "/Library/Updates",
        false,
    );
    rules
}

pub fn system_tools() -> Vec<ToolInfo> {
    vec![
        ToolInfo {
            id: "login-items".into(),
            name: "开机启动管理".into(),
            description: "管理开机自动启动的应用".into(),
            action: "open".into(),
        },
        ToolInfo {
            id: "junk-clean".into(),
            name: "系统文件清理".into(),
            description: "清理系统缓存与日志文件".into(),
            action: "junk-page".into(),
        },
        ToolInfo {
            id: "smart".into(),
            name: "磁盘健康".into(),
            description: "查看磁盘 S.M.A.R.T. 状态".into(),
            action: "smart".into(),
        },
        ToolInfo {
            id: "disk-utility".into(),
            name: "磁盘工具".into(),
            description: "打开 macOS 磁盘工具".into(),
            action: "open".into(),
        },
        ToolInfo {
            id: "about".into(),
            name: "系统信息".into(),
            description: "查看系统版本与硬件信息".into(),
            action: "open".into(),
        },
        ToolInfo {
            id: "storage".into(),
            name: "存储空间管理".into(),
            description: "查看与管理各分类存储占用".into(),
            action: "open".into(),
        },
        ToolInfo {
            id: "software-update".into(),
            name: "系统更新清理".into(),
            description: "清理系统更新下载残留".into(),
            action: "junk-page".into(),
        },
        ToolInfo {
            id: "trash".into(),
            name: "废纸篓管理".into(),
            description: "查看与清倒废纸篓".into(),
            action: "empty-trash".into(),
        },
    ]
}

pub fn run_tool(id: &str) -> Result<Option<String>, String> {
    let open = |url: &str| -> Result<(), String> {
        Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    };
    match id {
        "login-items" => {
            open("x-apple.systempreferences:com.apple.LoginItems-Settings.extension")?;
            Ok(None)
        }
        "disk-utility" => {
            open("/System/Applications/Utilities/Disk Utility.app")?;
            Ok(None)
        }
        "about" => {
            open("x-apple.systempreferences:com.apple.SystemProfiler.AboutExtension")?;
            Ok(None)
        }
        "storage" => {
            open("x-apple.systempreferences:com.apple.StorageManagement-Settings.extension")?;
            Ok(None)
        }
        "software-update" => {
            open("x-apple.systempreferences:com.apple.Software-Update-Settings.extension")?;
            Ok(None)
        }
        "smart" => {
            let out = Command::new("diskutil")
                .args(["info", "/"])
                .output()
                .map_err(|e| e.to_string())?;
            let text = String::from_utf8_lossy(&out.stdout);
            let line = text
                .lines()
                .find(|l| l.contains("SMART"))
                .unwrap_or("未获取到 S.M.A.R.T. 信息")
                .trim()
                .to_string();
            Ok(Some(line))
        }
        "trash" => {
            Command::new("open")
                .arg(dirs::home_dir().map(|h| h.join(".Trash")).unwrap_or_default())
                .spawn()
                .map_err(|e| e.to_string())?;
            Ok(None)
        }
        _ => Err(format!("unknown tool: {id}")),
    }
}

/// 注册/注销自动清理定时任务（launchd LaunchAgent）
pub fn schedule_auto_clean(enable: bool, interval_days: u32) -> Result<(), String> {
    let home = dirs::home_dir().ok_or_else(|| "无法定位用户目录".to_string())?;
    let dir = home.join("Library/LaunchAgents");
    let plist = dir.join("cn.aipalnet.diskcleanup.autoclean.plist");
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe = exe
        .ancestors()
        .find(|p| p.extension().map(|e| e == "app").unwrap_or(false))
        .map(|p| p.join("Contents/MacOS").join(crate::binary_name()))
        .unwrap_or(exe);

    // 先卸载旧任务
    let _ = Command::new("launchctl")
        .args(["unload", &plist.to_string_lossy().to_string()])
        .output();

    if !enable {
        let _ = std::fs::remove_file(&plist);
        return Ok(());
    }

    let interval = (interval_days.max(1) as u64) * 86_400;
    let exe_arg = exe.to_string_lossy().to_string();
    let home_str = home.to_string_lossy().to_string();
    let content = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>cn.aipalnet.diskcleanup.autoclean</string>
  <key>ProgramArguments</key>
  <array>
    <string>{exe_arg}</string>
    <string>--auto-clean</string>
  </array>
  <key>StartInterval</key>
  <integer>{interval}</integer>
  <key>RunAtLoad</key>
  <false/>
  <key>StandardOutPath</key>
  <string>{home_str}/Library/Logs/diskcleanup-autoclean.log</string>
</dict>
</plist>
"#
    );
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(&plist, content).map_err(|e| e.to_string())?;

    let out = Command::new("launchctl")
        .args(["load", "-w", &plist.to_string_lossy().to_string()])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).to_string())
    }
}

/// 查询定时任务是否已注册
pub fn auto_clean_scheduled() -> bool {
    dirs::home_dir()
        .map(|h| {
            h.join("Library/LaunchAgents/cn.aipalnet.diskcleanup.autoclean.plist")
                .exists()
        })
        .unwrap_or(false)
}

pub fn launch_items() -> Vec<LaunchItem> {
    let out = Command::new("osascript")
        .arg("-e")
        .arg(r#"tell application "System Events" to get the name of every login item"#)
        .output();
    match out {
        Ok(o) => {
            let s = String::from_utf8_lossy(&o.stdout);
            s.split(',')
                .map(|x| LaunchItem {
                    name: x.trim().to_string(),
                })
                .filter(|x| !x.name.is_empty())
                .collect()
        }
        Err(_) => Vec::new(),
    }
}

pub fn set_launch_at_login(enable: bool) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    // target/.../app.app/Contents/MacOS/binary 或 dev 下直接二进制
    let app_path = exe
        .ancestors()
        .find(|p| p.extension().map(|e| e == "app").unwrap_or(false))
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| exe);
    let script = if enable {
        format!(
            r#"tell application "System Events" to make login item at end with properties {{path:"{}", hidden:false}}"#,
            app_path.display()
        )
    } else {
        format!(
            r#"tell application "System Events" to delete login item whose path is "{}""#,
            app_path.display()
        )
    };
    let out = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).to_string())
    }
}
