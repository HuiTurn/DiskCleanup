use std::{fs, path::Path};
use walkdir::WalkDir;

pub fn scan_source(source: &Path) -> Result<(u64, u64), String> {
    let mut bytes = 0u64;
    let mut files = 0u64;
    for entry in WalkDir::new(source).follow_links(false) {
        let entry = entry.map_err(|e| format!("无法完整读取目录，请先处理权限或占用问题：{e}"))?;
        if !crate::engine::task::visit(entry.path()) {
            return Err("迁移检查已取消".into());
        }
        if let Some(reason) = sensitive(entry.path()) {
            return Err(format!(
                "{reason}：{}。请使用对应应用的迁移功能或先移出这些数据。",
                entry.path().display()
            ));
        }
        let meta = fs::symlink_metadata(entry.path()).map_err(|e| e.to_string())?;
        if meta.file_type().is_symlink() {
            return Err("目录包含符号链接，请先移出链接后再迁移".into());
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            // Reparse points include junctions and cloud placeholders. EFS is not copied blindly.
            if meta.file_attributes() & (0x400 | 0x1000 | 0x4000) != 0 {
                return Err(format!(
                    "包含云端占位、重解析点或加密文件，暂不迁移：{}",
                    entry.path().display()
                ));
            }
        }
        if meta.is_file() {
            bytes = bytes
                .checked_add(meta.len())
                .ok_or("目录过大，无法计算空间")?;
            files += 1;
        } else if !meta.is_dir() {
            return Err("包含不支持的特殊文件".into());
        }
    }
    Ok((bytes, files))
}

pub fn sensitive(path: &Path) -> Option<&'static str> {
    if let Some(reason) = crate::engine::safety::protected_reason(path) {
        return Some(reason);
    }
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();
    if matches!(
        name.as_str(),
        "package.json"
            | "cargo.toml"
            | "pom.xml"
            | "build.gradle"
            | "build.gradle.kts"
            | "pyproject.toml"
            | "cmakelists.txt"
            | ".idea"
            | ".vs"
    ) {
        return Some("检测到开发项目，迁移可能破坏绝对路径依赖");
    }
    None
}
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ManifestItem {
    pub relative: std::path::PathBuf,
    pub bytes: u64,
    pub sha256: String,
}
fn digest(path: &Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut file = fs::File::open(path).map_err(|e| format!("无法读取 {}：{e}", path.display()))?;
    let mut hash = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
pub fn manifest(
    source: &Path,
    mut progress: impl FnMut(&Path, u64),
) -> Result<Vec<ManifestItem>, String> {
    let mut result = Vec::new();
    for entry in WalkDir::new(source).follow_links(false) {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_type().is_symlink() {
            return Err("目录在检查后出现链接，已停止迁移".into());
        }
        if !entry.file_type().is_file() {
            continue;
        }
        if let Some(reason) = sensitive(entry.path()) {
            return Err(reason.into());
        }
        progress(entry.path(), result.len() as u64);
        let meta = entry.metadata().map_err(|e| e.to_string())?;
        let sha256 = digest(entry.path())?;
        let after = fs::metadata(entry.path()).map_err(|e| e.to_string())?;
        if meta.len() != after.len() || meta.modified().ok() != after.modified().ok() {
            return Err("文件正在变化，请关闭相关应用后重试".into());
        }
        result.push(ManifestItem {
            relative: entry
                .path()
                .strip_prefix(source)
                .map_err(|e| e.to_string())?
                .to_owned(),
            bytes: meta.len(),
            sha256,
        });
    }
    Ok(result)
}
pub fn verify(
    target: &Path,
    manifest: &[ManifestItem],
    mut progress: impl FnMut(&Path, u64),
) -> Result<(), String> {
    for (index, item) in manifest.iter().enumerate() {
        let path = target.join(&item.relative);
        progress(&path, index as u64);
        let meta = fs::symlink_metadata(&path)
            .map_err(|e| format!("目标文件缺失 {}：{e}", path.display()))?;
        if !meta.is_file()
            || meta.file_type().is_symlink()
            || meta.len() != item.bytes
            || digest(&path)? != item.sha256
        {
            return Err(format!(
                "迁移后内容校验失败：{}。请保留源目录、目标目录和记录。",
                path.display()
            ));
        }
    }
    Ok(())
}
pub fn require_space(bytes: u64, available: u64) -> Result<(), String> {
    let required = bytes
        .checked_add((bytes / 20).max(256 * 1024 * 1024))
        .ok_or("目录过大")?;
    if required > available {
        Err("目标盘空间不足（需额外预留至少 256 MB 或 5% 空间）".into())
    } else {
        Ok(())
    }
}
pub fn require_new_target(target: &Path) -> Result<(), String> {
    match fs::symlink_metadata(target) {
        Ok(_) => Err(
            "目标目录已存在。为避免合并或覆盖文件，请选择其他目标盘，或先重命名已有目录。".into(),
        ),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("无法检查目标目录：{e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn temp() -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!(
            "migration-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&p).unwrap();
        p
    }
    #[test]
    fn counts_nested_and_empty_files_without_mutation() {
        let p = temp();
        fs::create_dir(p.join("nested")).unwrap();
        fs::write(p.join("a"), b"hello").unwrap();
        fs::write(p.join("nested/b"), b"").unwrap();
        assert_eq!(scan_source(&p).unwrap(), (5, 2));
        assert_eq!(fs::read(p.join("a")).unwrap(), b"hello");
        fs::remove_dir_all(p).unwrap();
    }
    #[test]
    fn never_overwrites_existing_targets() {
        let p = temp();
        assert!(require_new_target(&p).is_err());
        assert!(require_new_target(&p.join("new")).is_ok());
        fs::remove_dir_all(p).unwrap();
    }
    #[test]
    fn rejects_insufficient_space_and_overflow() {
        assert!(require_space(1, 1).is_err());
        assert!(require_space(u64::MAX, u64::MAX).is_err());
        assert!(require_space(100, 1024 * 1024 * 1024).is_ok());
    }
    #[cfg(unix)]
    #[test]
    fn rejects_symlinks_and_missing_sources() {
        let p = temp();
        std::os::unix::fs::symlink("/", p.join("link")).unwrap();
        assert!(scan_source(&p).is_err());
        assert!(scan_source(&p.join("missing")).is_err());
        fs::remove_dir_all(p).unwrap();
    }
}

#[cfg(test)]
mod audit_tests {
    use super::*;
    #[test]
    fn manifest_detects_same_size_content_corruption() {
        let root = crate::engine::test_support::root();
        let source = root.join("source");
        let target = root.join("target");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&target).unwrap();
        fs::write(source.join("photo.txt"), b"abc").unwrap();
        let plan = manifest(&source, |_, _| {}).unwrap();
        fs::write(target.join("photo.txt"), b"abc").unwrap();
        assert!(verify(&target, &plan, |_, _| {}).is_ok());
        fs::write(target.join("photo.txt"), b"bad").unwrap();
        assert!(verify(&target, &plan, |_, _| {}).is_err());
        assert_eq!(fs::read(source.join("photo.txt")).unwrap(), b"abc");
    }
    #[test]
    fn project_and_vm_files_are_rejected_before_moving() {
        let root = crate::engine::test_support::root();
        fs::write(root.join("Cargo.toml"), b"project").unwrap();
        assert!(scan_source(&root).unwrap_err().contains("开发项目"));
        fs::remove_file(root.join("Cargo.toml")).unwrap();
        fs::write(root.join("ext4.vhdx"), b"disk").unwrap();
        assert!(scan_source(&root).is_err());
    }
}
