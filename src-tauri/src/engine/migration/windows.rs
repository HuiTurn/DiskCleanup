use super::{validation::*, *};
use ::windows::{
    core::{GUID, PCWSTR, PWSTR},
    Win32::{Storage::FileSystem::*, System::Com::*, UI::Shell::*},
};
use std::{
    fs,
    io::Write,
    os::windows::fs::MetadataExt,
    path::{Path, PathBuf},
};

const FOLDERS: [(&str, &str, &str, GUID); 6] = [
    ("desktop", "桌面", "Desktop", FOLDERID_Desktop),
    ("documents", "文档", "Documents", FOLDERID_Documents),
    ("downloads", "下载", "Downloads", FOLDERID_Downloads),
    ("pictures", "图片", "Pictures", FOLDERID_Pictures),
    ("music", "音乐", "Music", FOLDERID_Music),
    ("videos", "视频", "Videos", FOLDERID_Videos),
];
struct Com;
impl Com {
    fn new() -> Result<Self, String> {
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED)
                .ok()
                .map_err(|e| e.to_string())?;
        }
        Ok(Self)
    }
}
impl Drop for Com {
    fn drop(&mut self) {
        unsafe { CoUninitialize() }
    }
}
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
fn manager() -> Result<IKnownFolderManager, String> {
    unsafe {
        CoCreateInstance(&KnownFolderManager, None, CLSCTX_INPROC_SERVER).map_err(|e| e.to_string())
    }
}
fn known_path(id: &GUID) -> Result<String, String> {
    unsafe {
        let raw = SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None).map_err(|e| e.to_string())?;
        let result = raw.to_string().map_err(|e| e.to_string());
        CoTaskMemFree(Some(raw.0.cast()));
        result
    }
}
fn normalized(path: &str) -> String {
    path.trim_start_matches(r"\\?\")
        .trim_end_matches(['\\', '/'])
        .to_lowercase()
}
fn within(path: &str, root: &str) -> bool {
    let p = normalized(path);
    let r = normalized(root);
    p == r || p.starts_with(&(r + "\\"))
}
fn volume(path: &str) -> Result<String, String> {
    unsafe {
        let input = wide(path);
        let mut mount = [0u16; 32768];
        GetVolumePathNameW(PCWSTR(input.as_ptr()), &mut mount).map_err(|e| e.to_string())?;
        let mut id = [0u16; 128];
        GetVolumeNameForVolumeMountPointW(PCWSTR(mount.as_ptr()), &mut id)
            .map_err(|e| e.to_string())?;
        Ok(String::from_utf16_lossy(
            &id[..id.iter().position(|v| *v == 0).unwrap_or(id.len())],
        ))
    }
}
fn available(path: &str) -> Result<u64, String> {
    let input = wide(path);
    let mut free = 0;
    unsafe {
        GetDiskFreeSpaceExW(PCWSTR(input.as_ptr()), Some(&mut free), None, None)
            .map_err(|e| e.to_string())?;
    }
    Ok(free)
}
fn check_ancestors(path: &Path) -> Result<(), String> {
    for p in path.ancestors() {
        match fs::symlink_metadata(p) {
            Ok(meta) if meta.file_attributes() & 0x400 != 0 => {
                return Err(format!(
                    "路径包含链接或挂载点，暂不支持迁移：{}",
                    p.display()
                ))
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}
fn check_folder(id: &GUID, path: &str, mgr: &IKnownFolderManager) -> Result<(), String> {
    if !Path::new(path).is_dir() {
        return Err("目录不存在或无法访问".into());
    }
    if path.starts_with(r"\\") || path.as_bytes().get(1) != Some(&b':') {
        return Err("仅支持本地磁盘上的用户目录".into());
    }
    let profile = known_path(&FOLDERID_Profile)?;
    if normalized(path).len() <= 2 || within(&profile, path) {
        return Err("不能迁移磁盘根目录或整个用户目录".into());
    }
    for protected in [
        FOLDERID_Windows,
        FOLDERID_ProgramFiles,
        FOLDERID_LocalAppData,
        FOLDERID_RoamingAppData,
    ] {
        let root = known_path(&protected)?;
        if within(path, &root) || within(&root, path) {
            return Err("此系统目录位置与系统文件或应用数据重叠，不能迁移".into());
        }
    }
    for key in ["OneDrive", "OneDriveConsumer", "OneDriveCommercial"] {
        if let Ok(root) = std::env::var(key) {
            if within(path, &root) {
                return Err("此目录由 OneDrive 备份管理，请先在 OneDrive 中停止该目录备份".into());
            }
        }
    }
    if Path::new(path).components().any(|c| {
        c.as_os_str()
            .to_string_lossy()
            .to_lowercase()
            .starts_with("onedrive")
    }) {
        return Err("OneDrive 目录暂不支持迁移，请先停止文件夹备份".into());
    }
    check_ancestors(Path::new(path))?;
    let caps = unsafe {
        mgr.GetFolder(id)
            .and_then(|f| f.GetRedirectionCapabilities())
            .map_err(|e| e.to_string())?
    };
    if caps & KF_REDIRECTION_CAPABILITIES_DENY_ALL.0 as u32 != 0
        || caps & KF_REDIRECTION_CAPABILITIES_REDIRECTABLE.0 as u32 == 0
    {
        return Err("系统策略或权限不允许迁移此目录".into());
    }
    Ok(())
}
pub fn options() -> Result<MigrationOptions, String> {
    let _com = Com::new()?;
    let mgr = manager()?;
    let folders = FOLDERS
        .iter()
        .map(|(id, name, _, guid)| {
            let (path, blocked_reason) = match known_path(guid) {
                Ok(p) => {
                    let e = check_folder(guid, &p, &mgr).err();
                    (p, e)
                }
                Err(e) => (String::new(), Some(e)),
            };
            MigrationFolder {
                id: (*id).into(),
                name: (*name).into(),
                path,
                blocked_reason,
            }
        })
        .collect();
    let mut drives = vec![];
    let mask = unsafe { GetLogicalDrives() };
    for n in 0..26 {
        if mask & (1 << n) != 0 {
            let path = format!("{}:\\", (b'A' + n) as char);
            let w = wide(&path);
            if unsafe { GetDriveTypeW(PCWSTR(w.as_ptr())) } == 3 {
                if let Ok(free) = available(&path) {
                    drives.push(MigrationDrive {
                        path,
                        available_bytes: free,
                    });
                }
            }
        }
    }
    Ok(MigrationOptions {
        supported: true,
        folders,
        drives,
    })
}
pub fn prepare(request: MigrationRequest) -> Result<MigrationPlan, String> {
    let _com = Com::new()?;
    let mgr = manager()?;
    let (_, name, leaf, guid) = FOLDERS
        .iter()
        .find(|(id, _, _, _)| *id == request.folder_id)
        .ok_or("不支持的系统目录")?;
    let source = known_path(guid)?;
    if normalized(&source) != normalized(&request.expected_source) {
        return Err("目录位置已变化，请刷新后重新检查".into());
    }
    check_folder(guid, &source, &mgr)?;
    let drive = &request.target_drive;
    let bytes = drive.as_bytes();
    if bytes.len() != 3 || !bytes[0].is_ascii_alphabetic() || bytes[1] != b':' || bytes[2] != b'\\'
    {
        return Err("请选择有效的本地目标盘".into());
    }
    let w = wide(drive);
    if unsafe { GetDriveTypeW(PCWSTR(w.as_ptr())) } != 3 {
        return Err("请选择本地固定磁盘，不支持移动设备或网络盘".into());
    }
    if volume(&source)? == volume(drive)? {
        return Err("源目录与目标位于同一个磁盘卷，请选择其他盘".into());
    }
    let profile = known_path(&FOLDERID_Profile)?;
    let user = Path::new(&profile)
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("无法识别用户目录名称")?;
    let target = Path::new(drive).join("Users").join(user).join(leaf);
    check_ancestors(&target)?;
    require_new_target(&target)?;
    let target_str = target.to_string_lossy().into_owned();
    for (_, _, _, other) in FOLDERS {
        let p = known_path(&other)?;
        if other != *guid && (within(&p, &source) || within(&source, &p)) {
            return Err("系统目录相互嵌套，需先在 Windows 中恢复独立位置".into());
        }
        if within(&target_str, &p) || within(&p, &target_str) {
            return Err("目标路径与现有系统目录重叠".into());
        }
    }
    let (total_bytes, file_count) = scan_source(Path::new(&source))?;
    let available_bytes = available(drive)?;
    require_space(total_bytes, available_bytes)?;
    Ok(MigrationPlan {
        request,
        name: (*name).into(),
        source,
        target: target_str,
        total_bytes,
        file_count,
        available_bytes,
    })
}
pub fn execute(request: MigrationRequest) -> Result<(), String> {
    let plan = prepare(request)?;
    let _com = Com::new()?;
    let mgr = manager()?;
    let guid = &FOLDERS
        .iter()
        .find(|(id, _, _, _)| *id == plan.request.folder_id)
        .ok_or("不支持的目录")?
        .3;
    let log_dir = dirs::data_local_dir()
        .ok_or("无法定位日志目录")?
        .join("diskcleanup")
        .join("migrations");
    fs::create_dir_all(&log_dir).map_err(|e| e.to_string())?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let log_path = log_dir.join(format!("{stamp}.jsonl"));
    let mut log = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&log_path)
        .map_err(|e| e.to_string())?;
    writeln!(
        log,
        "{}",
        serde_json::json!({"phase":"started","plan":plan})
    )
    .and_then(|_| log.sync_all())
    .map_err(|e| format!("无法保存迁移记录，尚未移动文件：{e}"))?;
    {
        let mut status = STATUS.lock().unwrap_or_else(|e| e.into_inner());
        status.phase = "moving".into();
        status.source = plan.source.clone();
        status.target = plan.target.clone();
        status.journal = log_path.to_string_lossy().into_owned();
        status.message = "Windows 正在迁移文件并更新目录位置，请勿关闭电脑。".into();
    }
    let manifest = super::validation::manifest(Path::new(&plan.source), |path, count| {
        let mut status=STATUS.lock().unwrap_or_else(|e|e.into_inner());
        status.message=format!("正在保存迁移前校验清单，已检查 {count} 个文件：{}",path.display());
    })?;
    writeln!(log, "{}", serde_json::json!({"phase":"manifest","files":manifest})).and_then(|_|log.sync_all()).map_err(|e|format!("校验清单保存失败，尚未移动文件：{e}"))?;
    STATUS.lock().unwrap_or_else(|e|e.into_inner()).message="Windows 正在移动文件；详细进度与取消请使用系统迁移窗口。请勿强制退出。".into();
    let target = PathBuf::from(&plan.target);
    // No user files are manually deleted or merged by this application.
    fs::create_dir_all(target.parent().ok_or("目标路径无效")?).map_err(|e| e.to_string())?;
    check_ancestors(&target)?;
    require_new_target(&target)?;
    let dest = wide(&plan.target);
    let mut shell_error = PWSTR::null();
    let flags = (KF_REDIRECT_COPY_CONTENTS.0
        | KF_REDIRECT_DEL_SOURCE_CONTENTS.0
        | KF_REDIRECT_COPY_SOURCE_DACL.0
        | KF_REDIRECT_WITH_UI.0) as u32;
    let result = unsafe {
        mgr.Redirect(
            guid,
            None,
            flags,
            PCWSTR(dest.as_ptr()),
            None,
            Some(&mut shell_error),
        )
    };
    let detail = if shell_error.is_null() {
        String::new()
    } else {
        unsafe {
            let text = shell_error.to_string().unwrap_or_default();
            CoTaskMemFree(Some(shell_error.0.cast()));
            text
        }
    };
    let current = known_path(guid).unwrap_or_default();
    let result = result
        .map_err(|e| format!("Windows 未完成迁移：{e} {detail}"))
        .and_then(|_| {
            if normalized(&current) == normalized(&plan.target) {
                Ok(())
            } else {
                Err("迁移已取消或系统目录位置未更新".into())
            }
        });
    let result = result.and_then(|_| {
        super::validation::verify(Path::new(&plan.target), &manifest, |path,count| {
            STATUS.lock().unwrap_or_else(|e|e.into_inner()).message=format!("正在验证目标文件内容 {count}/{}：{}",manifest.len(),path.display());
        })?;
        if Path::new(&plan.source).exists() {
            let (_, remaining) = scan_source(Path::new(&plan.source))?;
            if remaining > 0 {
                return Err("新目录位置已启用，但原目录仍有文件，请核对剩余内容".into());
            }
        }
        Ok(())
    });
    let record = serde_json::json!({"phase":if result.is_ok(){"complete"}else{"failed"},"currentPath":current,"error":result.as_ref().err()});
    let logged = writeln!(log, "{record}").and_then(|_| log.sync_all());
    result.map_err(|e| {
        format!(
            "{e}。文件可能已部分移动，请核对原目录与目标目录，不要直接删除任一目录。记录：{}",
            log_path.display()
        )
    })?;
    logged.map_err(|e| {
        format!(
            "目录位置已更新，但完成记录写入失败：{e}。请核对新位置：{}",
            plan.target
        )
    })?;
    Ok(())
}

/// A started-only record survives a process crash and is never treated as success.
pub fn previous_status() -> Option<MigrationStatus> {
    let directory = dirs::data_local_dir()?
        .join("diskcleanup")
        .join("migrations");
    let latest = fs::read_dir(directory)
        .ok()?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("jsonl"))
        .max()?;
    let text = fs::read_to_string(&latest).ok()?;
    let mut lines = text.lines();
    let initial: serde_json::Value = serde_json::from_str(lines.next()?).ok()?;
    let result = lines
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .last();
    let complete = result.as_ref().and_then(|v| v["phase"].as_str()) == Some("complete");
    let plan = &initial["plan"];
    Some(MigrationStatus {
        phase: if complete { "complete" } else { "failed" }.into(),
        message: if complete {
            "上次迁移已完成。".into()
        } else {
            result.as_ref().and_then(|v|v["error"].as_str()).map(str::to_owned)
                .unwrap_or_else(||"上次迁移未记录完成，可能被中断。请核对原目录和目标目录后再操作；不要删除任一目录。".into())
        },
        source: plan["source"].as_str().unwrap_or_default().into(),
        target: plan["target"].as_str().unwrap_or_default().into(),
        journal: latest.to_string_lossy().into_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn path_containment_is_case_insensitive_and_component_bounded() {
        assert!(within(
            r"C:\Users\Demo\OneDrive\Desktop",
            r"c:\users\demo\onedrive"
        ));
        assert!(!within(
            r"C:\Users\Demo\OneDrive-other",
            r"C:\Users\Demo\OneDrive"
        ));
        assert!(!within(r"D:\Desktop", r"C:\Desktop"));
    }
}
