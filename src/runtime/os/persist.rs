//! Persist Kabootar OS VFS to disk (KVF1 + KVF2).

use crate::runtime::os::vfs::VirtualFs;
use std::fs;
use std::path::Path;

pub fn save_vfs(vfs: &VirtualFs, path: &str) -> Result<(), String> {
    let (dirs, files, mounts) = vfs.export_snapshot();
    let mut lines = vec!["KVF2".to_string()];
    for (vfs_path, host) in mounts {
        lines.push(format!("M:{vfs_path}:{host}"));
    }
    for d in dirs {
        lines.push(format!("D:{d}"));
    }
    for (f, content, mtime, readonly) in files {
        lines.push(format!("F:{f}:{mtime}:{readonly}"));
        lines.push(content);
    }
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| format!("vfs mkdir: {e}"))?;
        }
    }
    fs::write(path, lines.join("\n")).map_err(|e| format!("vfs save: {e}"))
}

pub fn load_vfs(path: &str) -> Result<VirtualFs, String> {
    let data = fs::read_to_string(path).map_err(|e| format!("vfs load: {e}"))?;
    let mut lines = data.lines();
    let header = lines.next().unwrap_or("");
    match header {
        "KVF2" => load_kvf2(lines),
        "KVF1" => load_kvf1(lines),
        _ => Err("Invalid VFS snapshot format".into()),
    }
}

fn load_kvf1(lines: std::str::Lines<'_>) -> Result<VirtualFs, String> {
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    let mut pending: Option<String> = None;
    for line in lines {
        if let Some(fp) = pending.take() {
            files.push((fp, line.to_string()));
            continue;
        }
        if let Some(d) = line.strip_prefix("D:") {
            dirs.push(d.to_string());
        } else if let Some(f) = line.strip_prefix("F:") {
            pending = Some(f.to_string());
        }
    }
    let mut vfs = VirtualFs::default();
    vfs.import_snapshot_v1(dirs, files);
    Ok(vfs)
}

fn load_kvf2(lines: std::str::Lines<'_>) -> Result<VirtualFs, String> {
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    let mut mounts = Vec::new();
    let mut pending: Option<(String, u64, bool)> = None;
    for line in lines {
        if let Some((fp, mtime, readonly)) = pending.take() {
            files.push((fp, line.to_string(), mtime, readonly));
            continue;
        }
        if let Some(m) = line.strip_prefix("M:") {
            if let Some((vfs, host)) = m.split_once(':') {
                mounts.push((vfs.to_string(), host.to_string()));
            }
            continue;
        }
        if let Some(d) = line.strip_prefix("D:") {
            dirs.push(d.to_string());
            continue;
        }
        if let Some(f) = line.strip_prefix("F:") {
            let parts: Vec<_> = f.splitn(3, ':').collect();
            if parts.len() >= 3 {
                let mtime = parts[1].parse().unwrap_or(0);
                let readonly = parts[2] == "true";
                pending = Some((parts[0].to_string(), mtime, readonly));
            } else {
                pending = Some((f.to_string(), 0, false));
            }
        }
    }
    let mut vfs = VirtualFs::default();
    vfs.import_snapshot(dirs, files, mounts);
    Ok(vfs)
}
