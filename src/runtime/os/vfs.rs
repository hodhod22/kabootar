//! Virtual filesystem for Kabootar OS — in-memory tree + host mounts.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VfsEntryKind {
    File,
    Directory,
}

#[derive(Debug, Clone)]
pub struct VfsStat {
    pub kind: VfsEntryKind,
    pub size: usize,
    pub mtime: u64,
    pub readonly: bool,
    pub mount: Option<String>,
}

#[derive(Debug, Clone)]
pub enum MountKind {
    Virtual,
    Host { host_root: PathBuf },
}

#[derive(Debug, Clone)]
pub struct MountPoint {
    pub vfs_path: String,
    pub kind: MountKind,
}

#[derive(Debug, Clone)]
struct FileEntry {
    content: String,
    mtime: u64,
    readonly: bool,
}

pub struct VirtualFs {
    files: HashMap<String, FileEntry>,
    dirs: HashSet<String>,
    mounts: Vec<MountPoint>,
    snapshots: Vec<String>,
}

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl Default for VirtualFs {
    fn default() -> Self {
        let mut vfs = Self {
            files: HashMap::new(),
            dirs: HashSet::from(["/".to_string()]),
            mounts: Vec::new(),
            snapshots: Vec::new(),
        };
        let _ = vfs.restore_golden();
        vfs
    }
}

impl VirtualFs {
    pub fn normalize(path: &str) -> String {
        let path = path.replace('\\', "/");
        if path.is_empty() || path == "/" {
            "/".to_string()
        } else if path.starts_with('/') {
            path
        } else {
            format!("/{}", path)
        }
    }

    fn parent_dir(path: &str) -> Option<String> {
        let path = Self::normalize(path);
        if path == "/" {
            return None;
        }
        let parent = path.rsplit_once('/').map(|(p, _)| p).unwrap_or("/");
        if parent.is_empty() {
            Some("/".to_string())
        } else {
            Some(parent.to_string())
        }
    }

    fn longest_mount(&self, path: &str) -> Option<&MountPoint> {
        let path = Self::normalize(path);
        self.mounts
            .iter()
            .filter(|m| {
                path == m.vfs_path
                    || path.starts_with(&format!("{}/", m.vfs_path.trim_end_matches('/')))
            })
            .max_by_key(|m| m.vfs_path.len())
    }

    fn host_rel_path(mount: &str, path: &str) -> Result<PathBuf, String> {
        let path = Self::normalize(path);
        let mount = Self::normalize(mount);
        let rest = path
            .strip_prefix(&mount)
            .unwrap_or("")
            .trim_start_matches('/');
        let mut out = PathBuf::new();
        for comp in rest.split('/').filter(|s| !s.is_empty()) {
            if comp == ".." {
                return Err("path traversal denied".into());
            }
            out.push(comp);
        }
        Ok(out)
    }

    fn host_abs(&self, mount: &MountPoint, path: &str) -> Result<PathBuf, String> {
        let MountKind::Host { host_root } = &mount.kind else {
            return Err("not a host mount".into());
        };
        let rel = Self::host_rel_path(&mount.vfs_path, path)?;
        let joined = host_root.join(rel);
        let canon_host = host_root
            .canonicalize()
            .unwrap_or_else(|_| host_root.clone());
        let canon_joined = joined
            .canonicalize()
            .unwrap_or(joined.clone());
        if !canon_joined.starts_with(&canon_host) {
            return Err("path escapes host mount".into());
        }
        Ok(joined)
    }

    fn ensure_parent_dirs(&mut self, path: &str) -> Result<(), String> {
        let Some(parent) = Self::parent_dir(path) else {
            return Ok(());
        };
        if let Some(m) = self.longest_mount(path) {
            if matches!(m.kind, MountKind::Host { .. }) {
                return Ok(());
            }
        }
        if parent != "/" && !self.dirs.contains(&parent) {
            return Err(format!("Directory not found: {}", parent));
        }
        Ok(())
    }

    pub fn mount_host(&mut self, vfs_path: &str, host_root: &str) -> Result<(), String> {
        let vfs_path = Self::normalize(vfs_path);
        if vfs_path == "/" {
            return Err("cannot mount at root".into());
        }
        let host = PathBuf::from(host_root);
        if !host.exists() {
            std::fs::create_dir_all(&host).map_err(|e| format!("host mount mkdir: {e}"))?;
        }
        if self.mounts.iter().any(|m| m.vfs_path == vfs_path) {
            return Err(format!("mount already exists: {vfs_path}"));
        }
        self.mounts.push(MountPoint {
            vfs_path: vfs_path.clone(),
            kind: MountKind::Host {
                host_root: host.canonicalize().unwrap_or(host),
            },
        });
        self.dirs.insert(vfs_path);
        Ok(())
    }

    pub fn unmount(&mut self, vfs_path: &str) -> Result<(), String> {
        let vfs_path = Self::normalize(vfs_path);
        let before = self.mounts.len();
        self.mounts.retain(|m| m.vfs_path != vfs_path);
        if self.mounts.len() == before {
            return Err(format!("mount not found: {vfs_path}"));
        }
        Ok(())
    }

    pub fn list_mounts(&self) -> Vec<MountPoint> {
        self.mounts.clone()
    }

    pub fn mkdir(&mut self, path: &str) -> Result<(), String> {
        let path = Self::normalize(path);
        if path == "/" {
            return Ok(());
        }
        if let Some(m) = self.longest_mount(&path) {
            if let MountKind::Host { .. } = &m.kind {
                let hp = self.host_abs(m, &path)?;
                return std::fs::create_dir_all(&hp).map_err(|e| format!("host mkdir: {e}"));
            }
        }
        self.ensure_parent_dirs(&path)?;
        self.dirs.insert(path);
        Ok(())
    }

    pub fn write(&mut self, path: &str, content: String) -> Result<(), String> {
        let path = Self::normalize(path);
        if let Some(m) = self.longest_mount(&path) {
            if let MountKind::Host { .. } = &m.kind {
                let hp = self.host_abs(m, &path)?;
                if let Some(parent) = hp.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| format!("host write mkdir: {e}"))?;
                }
                return std::fs::write(&hp, &content).map_err(|e| format!("host write: {e}"));
            }
        }
        self.ensure_parent_dirs(&path)?;
        if self
            .files
            .get(&path)
            .is_some_and(|f| f.readonly)
        {
            return Err(format!("Read-only file: {path}"));
        }
        self.files.insert(
            path,
            FileEntry {
                content,
                mtime: now_epoch(),
                readonly: false,
            },
        );
        Ok(())
    }

    pub fn read(&self, path: &str) -> Result<String, String> {
        let path = Self::normalize(path);
        if let Some(m) = self.longest_mount(&path) {
            if let MountKind::Host { .. } = &m.kind {
                let hp = self.host_abs(m, &path)?;
                return std::fs::read_to_string(&hp).map_err(|e| format!("host read: {e}"));
            }
        }
        self.files
            .get(&path)
            .map(|f| f.content.clone())
            .ok_or_else(|| format!("File not found: {}", path))
    }

    pub fn exists(&self, path: &str) -> bool {
        let path = Self::normalize(path);
        if let Some(m) = self.longest_mount(&path) {
            if let MountKind::Host { .. } = &m.kind {
                return self.host_abs(m, &path).map(|p| p.exists()).unwrap_or(false);
            }
        }
        self.files.contains_key(&path) || self.dirs.contains(&path)
    }

    pub fn stat(&self, path: &str) -> Result<VfsStat, String> {
        let path = Self::normalize(path);
        let mount = self.longest_mount(&path).map(|m| m.vfs_path.clone());
        if let Some(m) = self.longest_mount(&path) {
            if let MountKind::Host { .. } = &m.kind {
                let hp = self.host_abs(m, &path)?;
                if hp.is_file() {
                    let meta = hp.metadata().map_err(|e| format!("host stat: {e}"))?;
                    return Ok(VfsStat {
                        kind: VfsEntryKind::File,
                        size: meta.len() as usize,
                        mtime: meta
                            .modified()
                            .ok()
                            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                            .map(|d| d.as_secs())
                            .unwrap_or(0),
                        readonly: meta.permissions().readonly(),
                        mount,
                    });
                }
                if hp.is_dir() {
                    let n = std::fs::read_dir(&hp)
                        .map(|rd| rd.count())
                        .unwrap_or(0);
                    return Ok(VfsStat {
                        kind: VfsEntryKind::Directory,
                        size: n,
                        mtime: 0,
                        readonly: false,
                        mount,
                    });
                }
            }
        }
        if let Some(f) = self.files.get(&path) {
            return Ok(VfsStat {
                kind: VfsEntryKind::File,
                size: f.content.len(),
                mtime: f.mtime,
                readonly: f.readonly,
                mount,
            });
        }
        if self.dirs.contains(&path) {
            return Ok(VfsStat {
                kind: VfsEntryKind::Directory,
                size: self.list(&path).len(),
                mtime: 0,
                readonly: false,
                mount,
            });
        }
        Err(format!("Path not found: {}", path))
    }

    pub fn list(&self, dir: &str) -> Vec<String> {
        let dir = Self::normalize(dir);
        if let Some(m) = self.longest_mount(&dir) {
            if let MountKind::Host { .. } = &m.kind {
                if let Ok(hp) = self.host_abs(m, &dir) {
                    if let Ok(rd) = std::fs::read_dir(hp) {
                        let mut names: Vec<String> = rd.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).collect();
                        names.sort();
                        return names;
                    }
                }
                return Vec::new();
            }
        }
        let prefix = if dir == "/" {
            "/".to_string()
        } else {
            format!("{}/", dir.trim_end_matches('/'))
        };

        let mut names: Vec<String> = self
            .files
            .keys()
            .chain(self.dirs.iter())
            .filter(|path| path.starts_with(&prefix) && **path != dir)
            .filter_map(|path| {
                let rest = path.strip_prefix(&prefix)?;
                if rest.contains('/') {
                    None
                } else {
                    Some(rest.to_string())
                }
            })
            .collect();
        names.sort();
        names.dedup();
        names
    }

    pub fn delete(&mut self, path: &str) -> Result<(), String> {
        let path = Self::normalize(path);
        if let Some(m) = self.longest_mount(&path) {
            if let MountKind::Host { .. } = &m.kind {
                let hp = self.host_abs(m, &path)?;
                if hp.is_dir() {
                    return std::fs::remove_dir(&hp).map_err(|e| format!("host rmdir: {e}"));
                }
                return std::fs::remove_file(&hp).map_err(|e| format!("host unlink: {e}"));
            }
        }
        if self.files.remove(&path).is_some() {
            return Ok(());
        }
        if self.dirs.contains(&path) {
            if path == "/" {
                return Err("Cannot delete root directory".into());
            }
            let prefix = format!("{}/", path.trim_end_matches('/'));
            if self.files.keys().any(|p| p.starts_with(&prefix)) {
                return Err(format!("Directory not empty: {}", path));
            }
            if self.dirs.iter().any(|d| d.starts_with(&prefix) && d != &path) {
                return Err(format!("Directory not empty: {}", path));
            }
            self.dirs.remove(&path);
            return Ok(());
        }
        Err(format!("Path not found: {}", path))
    }

    pub fn rename(&mut self, from: &str, to: &str) -> Result<(), String> {
        let from = Self::normalize(from);
        let to = Self::normalize(to);
        if self.longest_mount(&from).is_some() || self.longest_mount(&to).is_some() {
            return Err("rename across mounts not supported".into());
        }
        if self.files.contains_key(&from) {
            let entry = self.files.remove(&from).unwrap();
            self.ensure_parent_dirs(&to)?;
            self.files.insert(to, entry);
            return Ok(());
        }
        if self.dirs.contains(&from) {
            if from == "/" {
                return Err("cannot rename root".into());
            }
            let prefix = format!("{}/", from.trim_end_matches('/'));
            let to_prefix = format!("{}/", to.trim_end_matches('/'));
            self.ensure_parent_dirs(&to)?;
            let mut new_dirs = HashSet::new();
            for d in self.dirs.iter() {
                if d == &from {
                    new_dirs.insert(to.clone());
                } else if d.starts_with(&prefix) {
                    new_dirs.insert(d.replacen(&from, &to, 1));
                } else {
                    new_dirs.insert(d.clone());
                }
            }
            let mut new_files = HashMap::new();
            for (k, v) in self.files.drain() {
                let nk = if k == from {
                    to.clone()
                } else if k.starts_with(&prefix) {
                    k.replacen(&prefix, &to_prefix, 1)
                } else {
                    k
                };
                new_files.insert(nk, v);
            }
            self.dirs = new_dirs;
            self.files = new_files;
            return Ok(());
        }
        Err(format!("Path not found: {from}"))
    }

    pub fn copy(&mut self, from: &str, to: &str) -> Result<(), String> {
        let from = Self::normalize(from);
        let to = Self::normalize(to);
        if self.longest_mount(&from).is_some() || self.longest_mount(&to).is_some() {
            let content = self.read(&from)?;
            return self.write(&to, content);
        }
        if let Some(f) = self.files.get(&from).cloned() {
            self.ensure_parent_dirs(&to)?;
            self.files.insert(
                to,
                FileEntry {
                    content: f.content,
                    mtime: now_epoch(),
                    readonly: false,
                },
            );
            return Ok(());
        }
        Err(format!("Path not found: {from}"))
    }

    pub fn chmod_readonly(&mut self, path: &str, readonly: bool) -> Result<(), String> {
        let path = Self::normalize(path);
        if let Some(f) = self.files.get_mut(&path) {
            f.readonly = readonly;
            return Ok(());
        }
        Err(format!("File not found: {path}"))
    }

    pub fn export_snapshot(&self) -> (Vec<String>, Vec<(String, String, u64, bool)>, Vec<(String, String)>) {
        let mut dirs: Vec<_> = self.dirs.iter().cloned().collect();
        dirs.sort();
        let mut files: Vec<_> = self
            .files
            .iter()
            .map(|(k, v)| (k.clone(), v.content.clone(), v.mtime, v.readonly))
            .collect();
        files.sort_by(|a, b| a.0.cmp(&b.0));
        let mounts: Vec<_> = self
            .mounts
            .iter()
            .filter_map(|m| {
                if let MountKind::Host { host_root } = &m.kind {
                    Some((
                        m.vfs_path.clone(),
                        host_root.to_string_lossy().into_owned(),
                    ))
                } else {
                    None
                }
            })
            .collect();
        (dirs, files, mounts)
    }

    pub fn import_snapshot(
        &mut self,
        dirs: Vec<String>,
        files: Vec<(String, String, u64, bool)>,
        mounts: Vec<(String, String)>,
    ) {
        self.dirs = dirs.into_iter().collect();
        if !self.dirs.contains("/") {
            self.dirs.insert("/".to_string());
        }
        self.files = files
            .into_iter()
            .map(|(k, v, mtime, readonly)| {
                (
                    k,
                    FileEntry {
                        content: v,
                        mtime,
                        readonly,
                    },
                )
            })
            .collect();
        self.mounts = mounts
            .into_iter()
            .filter_map(|(vfs, host)| {
                let host_root = PathBuf::from(host);
                if host_root.exists() {
                    Some(MountPoint {
                        vfs_path: vfs,
                        kind: MountKind::Host {
                            host_root: host_root.canonicalize().unwrap_or(host_root),
                        },
                    })
                } else {
                    None
                }
            })
            .collect();
    }

    /// Legacy KVF1 import (no metadata).
    pub fn import_snapshot_v1(&mut self, dirs: Vec<String>, files: Vec<(String, String)>) {
        let ts = now_epoch();
        self.import_snapshot(
            dirs,
            files
                .into_iter()
                .map(|(k, v)| (k, v, ts, false))
                .collect(),
            Vec::new(),
        );
    }

    pub fn record_snapshot(&mut self, path: &str) {
        self.snapshots.push(path.to_string());
        if self.snapshots.len() > 32 {
            self.snapshots.remove(0);
        }
    }

    pub fn snapshot_list(&self) -> Vec<String> {
        self.snapshots.clone()
    }

    /// Reset OS partition to golden template (apps/data mounts preserved).
    pub fn restore_golden(&mut self) -> Result<(), String> {
        let mounts = self.mounts.clone();
        *self = VirtualFs {
            files: HashMap::new(),
            dirs: HashSet::from(["/".to_string()]),
            mounts,
            snapshots: self.snapshots.clone(),
        };
        for d in ["/system", "/apps", "/data", "/efi"] {
            self.mkdir(d)?;
        }
        self.write(
            "/system/README",
            "Kabootar OS golden image — state-separated OS partition".into(),
        )?;
        self.write(
            "/efi/golden.img",
            "readonly golden snapshot anchor".into(),
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mkdir_and_stat() {
        let mut vfs = VirtualFs::default();
        vfs.mkdir("/apps").unwrap();
        vfs.write("/apps/note.txt", "hi".into()).unwrap();
        let stat = vfs.stat("/apps/note.txt").unwrap();
        assert_eq!(stat.kind, VfsEntryKind::File);
        assert_eq!(stat.size, 2);
        assert!(!stat.readonly);
        let dir_stat = vfs.stat("/apps").unwrap();
        assert_eq!(dir_stat.kind, VfsEntryKind::Directory);
    }

    #[test]
    fn rename_and_copy() {
        let mut vfs = VirtualFs::default();
        vfs.write("/a.txt", "one".into()).unwrap();
        vfs.copy("/a.txt", "/b.txt").unwrap();
        vfs.rename("/b.txt", "/c.txt").unwrap();
        assert_eq!(vfs.read("/c.txt").unwrap(), "one");
    }
}
