//! Final deletion boundary. Only individually reviewed regular files may be trashed.
use std::{
    collections::HashMap,
    fs,
    path::{Component, Path, PathBuf},
    sync::{LazyLock, Mutex},
    time::SystemTime,
};

pub fn key(p: &Path) -> String {
    let raw = p.to_string_lossy();
    let windows = raw.as_bytes().get(1) == Some(&b':') || raw.starts_with("\\\\");
    let raw = if windows {
        raw.replace('\\', "/").to_lowercase()
    } else {
        raw.into_owned()
    };
    let mut parts = Vec::new();
    for part in raw.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(part),
        }
    }
    let prefix = if raw.starts_with("//") {
        "//"
    } else if raw.starts_with('/') {
        "/"
    } else {
        ""
    };
    format!("{prefix}{}", parts.join("/"))
}
pub fn within(path: &Path, parent: &Path) -> bool {
    let (p, e) = (key(path), key(parent));
    !e.is_empty() && (p == e || p.starts_with(&format!("{}/", e.trim_end_matches('/'))))
}
pub fn excluded(path: &Path, excludes: &[PathBuf]) -> bool {
    excludes.iter().any(|ex| {
        within(path, ex)
            || match (path.canonicalize(), ex.canonicalize()) {
                (Ok(p), Ok(e)) => within(&p, &e),
                _ => false,
            }
    })
}
/// Conservative denylist; unknown files still require explicit review.
pub fn protected_reason(path: &Path) -> Option<&'static str> {
    let normalized = key(path).to_lowercase();
    if normalized
        .rsplit_once('/')
        .is_some_and(|(parent, _)| parent.ends_with("/appdata/local/microsoft/windows/explorer"))
    {
        let name = normalized.rsplit('/').next().unwrap_or("");
        if name.starts_with("thumbcache_") && name.ends_with(".db") {
            return None;
        }
    }
    let components: Vec<_> = normalized.split('/').collect();
    if components.iter().any(|c| {
        matches!(
            *c,
            "windows"
                | "program files"
                | "program files (x86)"
                | "applications"
                | "system"
                | "$recycle.bin"
                | ".trash"
                | "node_modules"
                | ".git"
                | ".venv"
                | "venv"
                | "sdk"
                | "sdks"
                | "android"
                | "docker"
                | "wsl"
                | "packages"
        ) || c.ends_with(".app")
            || c.ends_with(".framework")
            || c.ends_with(".pvm")
            || c.ends_with(".vmwarevm")
    }) {
        return Some("系统、应用、开发环境或虚拟机数据受保护，请使用所属应用管理");
    }
    let ext = path
        .extension()
        .and_then(|x| x.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if matches!(
        ext.as_str(),
        "vhd"
            | "vhdx"
            | "vmdk"
            | "qcow"
            | "qcow2"
            | "vdi"
            | "hdd"
            | "raw"
            | "db"
            | "sqlite"
            | "sqlite3"
            | "mdb"
            | "dll"
            | "sys"
            | "dylib"
            | "so"
            | "sln"
            | "csproj"
            | "vcxproj"
    ) {
        return Some("虚拟磁盘、数据库或程序依赖文件受保护");
    }
    None
}
#[derive(Clone, Debug, PartialEq)]
pub struct Stamp {
    pub len: u64,
    modified: SystemTime,
    created: Option<SystemTime>,
    identity: u128,
}
fn stamp(path: &Path) -> Result<Stamp, String> {
    let m = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !m.is_file() || m.file_type().is_symlink() {
        return Err("仅支持普通文件，不删除整个目录或链接".into());
    }
    #[cfg(unix)]
    let identity = {
        use std::os::unix::fs::MetadataExt;
        if m.nlink() > 1 {
            return Err("硬链接文件受保护".into());
        }
        ((m.dev() as u128) << 64) | m.ino() as u128
    };
    #[cfg(not(unix))]
    let identity = 0;
    Ok(Stamp {
        len: m.len(),
        modified: m.modified().map_err(|e| e.to_string())?,
        created: m.created().ok(),
        identity,
    })
}
pub fn validate(path: &Path, excludes: &[PathBuf]) -> Result<Stamp, String> {
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err("需要不含上级跳转的绝对路径".into());
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let root = path.ancestors().last().ok_or("无法识别磁盘")?;
        let wide = root
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        if unsafe {
            windows::Win32::Storage::FileSystem::GetDriveTypeW(windows::core::PCWSTR(wide.as_ptr()))
        } != 3
        {
            return Err("仅支持本地固定磁盘，网络盘和移动介质请使用文件管理器处理".into());
        }
    }
    if excluded(path, excludes) {
        return Err("已被排除设置保护".into());
    }
    if let Some(reason) = protected_reason(path) {
        return Err(reason.into());
    }
    for ancestor in path.ancestors() {
        if ancestor.is_dir()
            && [
                ".git",
                "package.json",
                "Cargo.toml",
                "pom.xml",
                "pyproject.toml",
                ".idea",
                ".vs",
            ]
            .iter()
            .any(|name| ancestor.join(name).exists())
        {
            return Err("开发项目目录受保护，请从所属 IDE 或文件管理器处理".into());
        }
        let m = fs::symlink_metadata(ancestor).map_err(|e| e.to_string())?;
        if m.file_type().is_symlink() {
            return Err("路径包含符号链接，请从真实位置重新扫描".into());
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if m.file_attributes() & (0x400 | 0x1000 | 0x4000) != 0 {
                return Err("云端占位、重解析点或加密文件受保护".into());
            }
        }
    }
    stamp(path)
}
static SNAPSHOTS: LazyLock<Mutex<HashMap<String, Stamp>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
pub fn remember(path: &Path) {
    if let Ok(s) = stamp(path) {
        let mut all = SNAPSHOTS.lock().unwrap();
        if all.len() >= 100_000 {
            all.clear();
        }
        all.insert(key(path), s);
    }
}
pub fn reviewed(path: &Path) -> Result<Stamp, String> {
    SNAPSHOTS
        .lock()
        .unwrap()
        .get(&key(path))
        .cloned()
        .ok_or_else(|| "扫描记录已过期，请重新扫描后选择".into())
}
pub fn revalidate(path: &Path, expected: &Stamp, excludes: &[PathBuf]) -> Result<(), String> {
    if validate(path, excludes)? != *expected {
        return Err("文件自扫描后已变化，已跳过；请重新扫描".into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows_boundaries() {
        let ex = Path::new(r"C:\Users\Demo\Keep\");
        assert!(within(Path::new(r"c:\users\demo\keep\a.txt"), ex));
        assert!(!within(Path::new(r"C:\Users\Demo\Keeper\a.txt"), ex));
        assert!(within(Path::new("/a/b"), Path::new("/")));
    }
    #[test]
    fn detects_sensitive_data() {
        for p in [
            "/data/ext4.vhdx",
            "/data/Docker.raw",
            "/data/A.app/Contents/x",
            "/data/.git/config",
            "/data/a.sqlite",
        ] {
            assert!(protected_reason(Path::new(p)).is_some(), "{p}");
        }
        assert!(protected_reason(Path::new("/data/movie.mp4")).is_none());
    }
}

#[cfg(all(test, unix))]
mod link_tests {
    use super::*;
    #[test]
    fn replacement_symlinks_and_hardlinks_are_rejected() {
        let root = super::super::test_support::root();
        let file = root.join("reviewed.txt");
        let target = root.join("personal.txt");
        fs::write(&file, b"cache").unwrap();
        fs::write(&target, b"personal").unwrap();
        remember(&file);
        let original = reviewed(&file).unwrap();
        fs::remove_file(&file).unwrap();
        std::os::unix::fs::symlink(&target, &file).unwrap();
        assert!(revalidate(&file, &original, &[]).is_err());
        assert_eq!(fs::read(&target).unwrap(), b"personal");
        fs::remove_file(&file).unwrap();
        fs::hard_link(&target, &file).unwrap();
        assert!(validate(&file, &[]).is_err());
    }
}
