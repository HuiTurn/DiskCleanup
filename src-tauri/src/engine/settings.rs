//! 设置持久化：JSON 存于系统配置目录。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(alias = "launch_at_login")]
    pub launch_at_login: bool,
    #[serde(alias = "close_confirm")]
    pub close_confirm: bool,
    pub language: String,
    #[serde(alias = "auto_clean_enabled")]
    pub auto_clean_enabled: bool,
    #[serde(alias = "auto_clean_interval_days")]
    pub auto_clean_interval_days: u32,
    /// 上次自动清理时间（UNIX 秒），null 表示从未执行
    #[serde(alias = "last_auto_clean_ts")]
    pub last_auto_clean_ts: Option<u64>,
    #[serde(alias = "large_file_min_mb")]
    pub large_file_min_mb: u32,
    #[serde(alias = "exclude_dirs")]
    pub exclude_dirs: Vec<String>,
    pub theme: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            launch_at_login: false,
            close_confirm: true,
            language: "zh-CN".into(),
            auto_clean_enabled: false,
            auto_clean_interval_days: 7,
            last_auto_clean_ts: None,
            large_file_min_mb: 100,
            exclude_dirs: default_excludes(),
            theme: "light".into(),
        }
    }
}

fn default_excludes() -> Vec<String> {
    #[cfg(target_os = "macos")]
    {
        vec!["/System".into(), "/private/var".into(), "/Volumes".into()]
    }
    #[cfg(target_os = "windows")]
    {
        vec!["C:\\Windows\\WinSxS".into(), "C:\\Windows\\System32".into()]
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        vec![]
    }
}

fn config_path() -> Option<PathBuf> {
    #[cfg(test)]
    {
        return Some(super::test_support::root().join("settings.json"));
    }
    #[cfg(not(test))]
    dirs::config_dir().map(|d| d.join("diskcleanup").join("settings.json"))
}

impl Settings {
    pub fn load() -> Self {
        if let Some(p) = config_path() {
            if let Ok(s) = std::fs::read_to_string(&p) {
                if let Ok(v) = serde_json::from_str(&s) {
                    return v;
                }
            }
        }
        Self::default()
    }

    pub fn load_checked() -> Result<Self, String> {
        let p = config_path().ok_or("无法定位配置文件")?;
        match std::fs::read_to_string(p) {
            Ok(text) => serde_json::from_str(&text)
                .map_err(|e| format!("设置文件损坏，为保护排除目录已停止清理：{e}")),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(format!("无法读取排除设置，已停止清理：{e}")),
        }
    }
    pub fn save(&self) -> Result<(), String> {
        use std::io::Write;
        let p = config_path().ok_or("无法定位配置文件")?;
        std::fs::create_dir_all(p.parent().unwrap()).map_err(|e| e.to_string())?;
        let tmp = p.with_extension(format!("{}.tmp", std::process::id()));
        let mut f = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
        f.write_all(
            serde_json::to_string_pretty(self)
                .map_err(|e| e.to_string())?
                .as_bytes(),
        )
        .and_then(|_| f.sync_all())
        .map_err(|e| e.to_string())?;
        std::fs::rename(tmp, p).map_err(|e| e.to_string())
    }
}
