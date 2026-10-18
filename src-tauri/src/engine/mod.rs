//! engine 模块聚合 + 磁盘枚举。

pub mod autoclean;
pub mod safety;
pub mod task;
pub mod cleaner;
pub mod duplicates;
pub mod history;
pub mod migration;
pub mod junk;
pub mod large_files;
pub mod platform;
pub mod scanner;
pub mod settings;
pub mod types;

use sysinfo::Disks;

use types::DiskInfo;

/// 枚举所有挂载磁盘
pub fn list_disks() -> Vec<DiskInfo> {
    let disks = Disks::new_with_refreshed_list();
    let mut out: Vec<DiskInfo> = disks
        .list()
        .iter()
        .map(|d| {
            let mount = d.mount_point().to_string_lossy().to_string();
            let total = d.total_space();
            let available = d.available_space();
            let is_system = mount == "/" || mount.eq_ignore_ascii_case("C:\\");
            DiskInfo {
                name: if is_system {
                    if cfg!(target_os = "macos") {
                        "Macintosh HD".to_string()
                    } else {
                        "系统 (C:)".to_string()
                    }
                } else {
                    d.name().to_string_lossy().to_string()
                },
                mount_point: mount,
                total_bytes: total,
                used_bytes: total.saturating_sub(available),
                is_system,
            }
        })
        .collect();
    out.sort_by(|a, b| b.is_system.cmp(&a.is_system));
    out
}

#[cfg(test)]
pub mod test_support {
    thread_local! { static ROOT: std::path::PathBuf = {
        let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let p=std::env::temp_dir().join(format!("diskcleanup-test-{}-{stamp}",std::process::id()));
        std::fs::create_dir_all(&p).unwrap(); p.canonicalize().unwrap()
    }; }
    pub fn root()->std::path::PathBuf { ROOT.with(Clone::clone) }
}
