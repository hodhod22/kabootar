//! Kabootar OS — sandboxed kernel with virtual filesystem.

mod display;
mod drivers;
mod features_api;
mod fsys;
mod iosys;
mod kcore;
mod mm;
mod netstack;
mod os_api;
mod proc2;
mod ring3;
pub mod host_bridge;
pub mod hotplug;
pub mod native_hw;
mod kernel;
mod memory;
mod permissions;
mod persist;
mod process;
mod sauce;
mod sauce_api;
mod scheduler;
mod subsys;
mod syscall;
mod vfs;
mod window;
mod xcut;

pub use display::DisplaySurface;
pub use drivers::{
    device_list_value, gpu_info_value, net_ifaces_value, DeviceDescriptor, DeviceManager,
    DriverKind,
};
pub use host_bridge::HostBridge;
pub use hotplug::HotplugEvent;
pub use kernel::Kernel;
pub use memory::MemoryManager;
pub use permissions::{Capability, PermissionSet, NET_CONNECT, PERM_ADMIN, HOTPLUG};
pub use process::{ProcessEntry, ProcessState};
pub use subsys::{KernelSubsystems, SharedSubsystems};
pub use vfs::{VfsEntryKind, VfsStat};
pub use window::OsWindow;

use crate::value::{Environment, Value};
use std::sync::{Arc, Mutex};
use display::DisplayServer;
use memory::MemoryManager as MemMgr;
use process::ProcessTable;
use scheduler::Scheduler;
use vfs::VirtualFs;
use window::WindowManager;

#[derive(Clone)]
pub struct OsHandle {
    pub kernel: Kernel,
    vfs: Arc<Mutex<VirtualFs>>,
    processes: Arc<Mutex<ProcessTable>>,
    windows: Arc<Mutex<WindowManager>>,
    display: Arc<Mutex<DisplayServer>>,
    memory: Arc<Mutex<MemMgr>>,
    scheduler: Arc<Mutex<Scheduler>>,
    devices: Arc<Mutex<DeviceManager>>,
    permissions: Arc<Mutex<PermissionSet>>,
    subject: Arc<Mutex<u64>>,
    pub(crate) subsys: SharedSubsystems,
}

impl std::fmt::Debug for OsHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OsHandle")
            .field("kernel", &self.kernel)
            .finish_non_exhaustive()
    }
}

impl OsHandle {
    pub fn new() -> Self {
        Self {
            kernel: Kernel::new(),
            vfs: Arc::new(Mutex::new(VirtualFs::default())),
            processes: Arc::new(Mutex::new(ProcessTable::default())),
            windows: Arc::new(Mutex::new(WindowManager::default())),
            display: Arc::new(Mutex::new(DisplayServer::default())),
            memory: Arc::new(Mutex::new(MemMgr::default())),
            scheduler: Arc::new(Mutex::new(Scheduler::default())),
            devices: Arc::new(Mutex::new(DeviceManager::default())),
            permissions: Arc::new(Mutex::new(PermissionSet::default())),
            subject: Arc::new(Mutex::new(1)),
            subsys: subsys::new_shared(),
        }
    }

    pub fn subject(&self) -> Result<u64, String> {
        self.subject
            .lock()
            .map(|g| *g)
            .map_err(|_| "subject lock poisoned".into())
    }

    pub fn set_subject(&self, pid: u64) -> Result<(), String> {
        let pt = self
            .processes
            .lock()
            .map_err(|_| "process lock poisoned".to_string())?;
        if !pt.list().iter().any(|p| p.pid == pid) {
            return Err(format!("unknown process subject: {pid}"));
        }
        drop(pt);
        let mut sub = self
            .subject
            .lock()
            .map_err(|_| "subject lock poisoned".to_string())?;
        *sub = pid;
        Ok(())
    }

    fn with_perms<F, T>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&PermissionSet) -> Result<T, String>,
    {
        let perms = self
            .permissions
            .lock()
            .map_err(|_| "permissions lock poisoned".to_string())?;
        f(&perms)
    }

    fn with_perms_mut<F, T>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&mut PermissionSet) -> Result<T, String>,
    {
        let mut perms = self
            .permissions
            .lock()
            .map_err(|_| "permissions lock poisoned".to_string())?;
        f(&mut perms)
    }

    fn require_cap(&self, required: &str) -> Result<(), String> {
        let pid = self.subject()?;
        self.with_perms(|p| p.require(pid, required))
    }

    fn require_admin(&self) -> Result<(), String> {
        self.require_cap(PERM_ADMIN)
    }

    fn require_vfs(&self, path: &str, write: bool) -> Result<(), String> {
        let pid = self.subject()?;
        let base = if write { "vfs:write" } else { "vfs:read" };
        self.with_perms(|p| {
            if p.is_allowed(pid, &format!("{base}:*")) {
                return Ok(());
            }
            let mut cur = path.to_string();
            loop {
                if p.is_allowed(pid, &format!("{base}:{cur}")) {
                    return Ok(());
                }
                if cur == "/" || cur.is_empty() {
                    break;
                }
                match cur.rfind('/') {
                    Some(0) => cur = "/".into(),
                    Some(i) => cur.truncate(i),
                    None => break,
                }
            }
            p.require(pid, &format!("{base}:{path}"))
        })
    }

    fn require_device(&self, device_id: &str) -> Result<(), String> {
        self.require_cap(&permissions::device_cap(device_id))
    }

    fn require_device_ioctl(&self, device_id: &str, op: &str) -> Result<(), String> {
        let pid = self.subject()?;
        self.with_perms(|p| {
            if p.is_allowed(pid, &permissions::device_cap(device_id)) {
                return Ok(());
            }
            p.require(
                pid,
                &permissions::device_ioctl_cap(device_id, op),
            )
        })
    }

    fn with_devices<F, T>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&mut DeviceManager) -> Result<T, String>,
    {
        let mut dm = self
            .devices
            .lock()
            .map_err(|_| "device manager lock poisoned".to_string())?;
        f(&mut dm)
    }

    pub fn name(&self) -> &str {
        &self.kernel.name
    }

    pub fn version(&self) -> &str {
        &self.kernel.version
    }

    fn with_vfs<F, T>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&mut VirtualFs) -> Result<T, String>,
    {
        let mut vfs = self
            .vfs
            .lock()
            .map_err(|_| "OS filesystem lock poisoned".to_string())?;
        f(&mut vfs)
    }

    pub fn with_subsys<F, T>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&mut KernelSubsystems) -> Result<T, String>,
    {
        let mut g = self
            .subsys
            .lock()
            .map_err(|_| "kernel subsystems lock poisoned".to_string())?;
        f(&mut g)
    }

    pub fn info(&self) -> String {
        self.kernel.info()
    }

    pub fn capabilities(&self) -> Vec<&'static str> {
        Kernel::active_capabilities()
    }

    pub fn mkdir(&self, path: &str) -> Result<(), String> {
        self.require_vfs(path, true)?;
        self.with_vfs(|vfs| vfs.mkdir(path))
    }

    pub fn write(&self, path: &str, content: String) -> Result<(), String> {
        self.require_vfs(path, true)?;
        self.with_vfs(|vfs| vfs.write(path, content.clone()))?;
        self.with_subsys(|s| {
            let _ = s.fsys.write_with_journal(path, &content);
            s.sauce.state.write_data(content.len() as u64);
            if let Some(app) = path.strip_prefix("/apps/").and_then(|p| p.split('/').next()) {
                if !app.is_empty() {
                    s.sauce.ai.record_launch(app, 12);
                }
            }
            s.xcut.log.record("vfs_write", path);
            Ok(())
        })
    }

    pub fn read(&self, path: &str) -> Result<String, String> {
        self.require_vfs(path, false)?;
        if let Some(cached) = self.with_subsys(|s| Ok(s.fsys.read_cached(path)))? {
            if let Ok(text) = String::from_utf8(cached) {
                return Ok(text);
            }
        }
        let content = self.with_vfs(|vfs| vfs.read(path))?;
        let _ = self.with_subsys(|s| {
            s.fsys.page_cache.put(path, content.as_bytes());
            Ok(())
        });
        Ok(content)
    }

    pub fn exists(&self, path: &str) -> Result<bool, String> {
        self.require_vfs(path, false)?;
        self.with_vfs(|vfs| Ok(vfs.exists(path)))
    }

    pub fn stat(&self, path: &str) -> Result<VfsStat, String> {
        self.require_vfs(path, false)?;
        self.with_vfs(|vfs| vfs.stat(path))
    }

    pub fn list(&self, dir: &str) -> Result<Vec<String>, String> {
        self.require_vfs(dir, false)?;
        self.with_vfs(|vfs| Ok(vfs.list(dir)))
    }

    pub fn delete(&self, path: &str) -> Result<(), String> {
        self.require_vfs(path, true)?;
        self.with_vfs(|vfs| vfs.delete(path))
    }

    pub fn rename(&self, from: &str, to: &str) -> Result<(), String> {
        self.require_vfs(from, true)?;
        self.require_vfs(to, true)?;
        self.with_vfs(|vfs| vfs.rename(from, to))
    }

    pub fn copy_path(&self, from: &str, to: &str) -> Result<(), String> {
        self.require_vfs(from, false)?;
        self.require_vfs(to, true)?;
        self.with_vfs(|vfs| vfs.copy(from, to))
    }

    pub fn mount_host(&self, vfs_path: &str, host_root: &str) -> Result<(), String> {
        self.require_admin()?;
        self.with_vfs(|vfs| vfs.mount_host(vfs_path, host_root))
    }

    pub fn unmount(&self, vfs_path: &str) -> Result<(), String> {
        self.require_admin()?;
        self.with_vfs(|vfs| vfs.unmount(vfs_path))
    }

    pub fn list_mounts(&self) -> Result<Vec<(String, String)>, String> {
        self.with_vfs(|vfs| {
            Ok(vfs
                .list_mounts()
                .into_iter()
                .filter_map(|m| match m.kind {
                    vfs::MountKind::Host { host_root } => Some((
                        m.vfs_path,
                        host_root.to_string_lossy().into_owned(),
                    )),
                    vfs::MountKind::Virtual => None,
                })
                .collect())
        })
    }

    pub fn spawn(&self, name: &str) -> Result<u64, String> {
        let parent = self.subject()?;
        let pid = {
            let mut pt = self
                .processes
                .lock()
                .map_err(|_| "OS process lock poisoned".to_string())?;
            pt.spawn(name)
        };
        self.with_perms_mut(|p| {
            p.inherit_from(parent, pid);
            Ok(())
        })?;
        self.with_subsys(|s| {
            s.proc2.spawn_thread(pid, name);
            s.sauce.state.register_app();
            s.sauce.ai.record_launch(name, 8);
            s.xcut.log.record("spawn", &format!("pid={pid} {name}"));
            Ok(())
        })?;
        Ok(pid)
    }

    /// Golden-image restore: reset OS partition, keep apps/data stats, reload VFS template.
    pub fn golden_restore(&self) -> Result<u64, String> {
        let start = std::time::Instant::now();
        self.with_vfs(|vfs| vfs.restore_golden())?;
        let ms = start.elapsed().as_millis().max(1) as u64;
        let reported = self.with_subsys(|s| Ok(s.sauce.state.golden_restore(ms)))?;
        self.with_subsys(|s| {
            s.xcut.log.record("golden_restore", &format!("{reported}ms"));
            Ok(())
        })?;
        Ok(reported)
    }

    pub fn boot_ms(&self) -> u64 {
        static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        START
            .get_or_init(std::time::Instant::now)
            .elapsed()
            .as_millis() as u64
    }

    pub fn process_list(&self) -> Result<Vec<ProcessEntry>, String> {
        let pt = self
            .processes
            .lock()
            .map_err(|_| "OS process lock poisoned".to_string())?;
        Ok(pt.list())
    }

    pub fn window_create(&self, title: &str, width: i64, height: i64) -> Result<u64, String> {
        let mut wm = self
            .windows
            .lock()
            .map_err(|_| "OS window lock poisoned".to_string())?;
        Ok(wm.create(title, width, height))
    }

    pub fn window_list(&self) -> Result<Vec<OsWindow>, String> {
        let wm = self
            .windows
            .lock()
            .map_err(|_| "OS window lock poisoned".to_string())?;
        Ok(wm.list())
    }

    pub fn window_bind_tab(&self, window_id: u64, tab_id: u64) -> Result<bool, String> {
        let mut wm = self
            .windows
            .lock()
            .map_err(|_| "OS window lock poisoned".to_string())?;
        Ok(wm.bind_tab(window_id, tab_id))
    }

    pub fn display_register(&self, window_id: u64, title: &str, w: i64, h: i64) -> Result<u64, String> {
        let mut ds = self.display.lock().map_err(|_| "display lock poisoned".to_string())?;
        Ok(ds.register(window_id, title, w, h))
    }

    pub fn display_present(&self, window_id: u64, bytes: usize) -> Result<bool, String> {
        let allow = self.with_subsys(|s| Ok(s.sauce.energy.should_repaint()))?;
        if !allow {
            return Ok(false);
        }
        self.with_subsys(|s| {
            s.sauce.energy.mark_repaint();
            Ok(())
        })?;
        let mut ds = self.display.lock().map_err(|_| "display lock poisoned".to_string())?;
        Ok(ds.present(window_id, bytes))
    }

    pub fn mem_alloc(&self, size: usize, label: &str) -> Result<u64, String> {
        let mut mm = self.memory.lock().map_err(|_| "memory lock poisoned".to_string())?;
        mm.alloc(size, label)
    }

    pub fn mem_free(&self, id: u64) -> Result<bool, String> {
        let mut mm = self.memory.lock().map_err(|_| "memory lock poisoned".to_string())?;
        Ok(mm.free(id))
    }

    pub fn mem_write(&self, id: u64, offset: usize, data: &[u8]) -> Result<usize, String> {
        let mut mm = self.memory.lock().map_err(|_| "memory lock poisoned".to_string())?;
        mm.write(id, offset, data)
    }

    pub fn mem_read(&self, id: u64, offset: usize, len: usize) -> Result<Vec<u8>, String> {
        let mut mm = self.memory.lock().map_err(|_| "memory lock poisoned".to_string())?;
        mm.read(id, offset, len)
    }

    pub fn mem_stats(&self) -> Result<(usize, usize, usize), String> {
        let mm = self.memory.lock().map_err(|_| "memory lock poisoned".to_string())?;
        Ok(mm.stats())
    }

    pub fn sched_enqueue(&self, name: &str) -> Result<u64, String> {
        let mut sc = self.scheduler.lock().map_err(|_| "scheduler lock poisoned".to_string())?;
        Ok(sc.enqueue(name))
    }

    pub fn vfs_save(&self, path: &str) -> Result<(), String> {
        let vfs = self.vfs.lock().map_err(|_| "vfs lock poisoned".to_string())?;
        persist::save_vfs(&vfs, path)?;
        drop(vfs);
        let mut vfs = self.vfs.lock().map_err(|_| "vfs lock poisoned".to_string())?;
        vfs.record_snapshot(path);
        Ok(())
    }

    pub fn vfs_snapshot_list(&self) -> Result<Vec<String>, String> {
        let vfs = self.vfs.lock().map_err(|_| "vfs lock poisoned".to_string())?;
        Ok(vfs.snapshot_list())
    }

    pub fn vfs_load(&self, path: &str) -> Result<(), String> {
        let loaded = persist::load_vfs(path)?;
        let mut vfs = self.vfs.lock().map_err(|_| "vfs lock poisoned".to_string())?;
        let (dirs, files, mounts) = loaded.export_snapshot();
        vfs.import_snapshot(dirs, files, mounts);
        drop(vfs);
        self.with_subsys(|s| {
            s.fsys.reset_after_vfs_snapshot();
            Ok(())
        })?;
        Ok(())
    }

    pub fn dev_list(&self) -> Result<Vec<DeviceDescriptor>, String> {
        self.with_devices(|dm| Ok(dm.list_devices().to_vec()))
    }

    pub fn dev_open(&self, device_id: &str) -> Result<u64, String> {
        self.require_device(device_id)?;
        self.with_devices(|dm| dm.open(device_id))
    }

    pub fn dev_close(&self, handle: u64) -> Result<(), String> {
        self.with_devices(|dm| dm.close(handle))
    }

    pub fn dev_ioctl(
        &self,
        handle: u64,
        op: &str,
        args: &[Value],
    ) -> Result<Value, String> {
        let device_id = self.with_devices(|dm| {
            dm.handle_info(handle)
                .map(|h| h.device_id.clone())
        })?;
        self.require_device_ioctl(&device_id, op)?;
        if op == "connect" {
            self.require_cap(NET_CONNECT)?;
        }
        self.with_devices(|dm| dm.ioctl(handle, op, args))
    }

    pub fn gpu_info(&self) -> Result<Value, String> {
        self.with_devices(|dm| Ok(gpu_info_value(&dm.gpu.info())))
    }

    pub fn net_interfaces(&self) -> Result<Value, String> {
        self.with_devices(|dm| Ok(net_ifaces_value(dm.net.interfaces())))
    }

    pub fn net_connect(&self, host: &str, port: u16) -> Result<u64, String> {
        self.require_cap(NET_CONNECT)?;
        self.with_devices(|dm| dm.net.connect(host, port))
    }

    pub fn net_listen(&self, host: &str, port: u16) -> Result<u64, String> {
        self.require_cap(NET_CONNECT)?;
        self.with_devices(|dm| dm.net.listen(host, port))
    }

    pub fn net_accept(&self, listener: u64) -> Result<u64, String> {
        self.require_cap(NET_CONNECT)?;
        self.with_devices(|dm| dm.net.accept(listener))
    }

    pub fn net_poll(&self, sockets: &[u64]) -> Result<Value, String> {
        self.require_cap(NET_CONNECT)?;
        self.with_devices(|dm| {
            let events = dm.net.poll(sockets);
            Ok(Value::Array(
                events
                    .into_iter()
                    .map(|e| {
                        let mut m = std::collections::HashMap::new();
                        m.insert("socket".into(), Value::Number(e.socket as i64));
                        m.insert("kind".into(), Value::String(e.kind));
                        Value::Object(m)
                    })
                    .collect(),
            ))
        })
    }

    pub fn net_udp_bind(&self, host: &str, port: u16) -> Result<u64, String> {
        self.require_cap(NET_CONNECT)?;
        self.with_devices(|dm| dm.net.udp_bind(host, port))
    }

    pub fn perm_grant(&self, pid: u64, cap: &str) -> Result<(), String> {
        self.require_admin()?;
        self.with_perms_mut(|p| p.grant(pid, cap))
    }

    pub fn perm_revoke(&self, pid: u64, cap: &str) -> Result<bool, String> {
        self.require_admin()?;
        self.with_perms_mut(|p| p.revoke(pid, cap))
    }

    pub fn perm_list(&self, pid: u64) -> Result<Vec<String>, String> {
        self.with_perms(|p| Ok(p.list(pid)))
    }

    pub fn perm_check(&self, pid: u64, cap: &str) -> Result<bool, String> {
        self.with_perms(|p| Ok(p.is_allowed(pid, cap)))
    }

    pub fn hotplug_poll(&self) -> Vec<HotplugEvent> {
        hotplug::drain()
    }

    pub fn perm_clear(&self, pid: u64) -> Result<(), String> {
        self.require_admin()?;
        self.with_perms_mut(|p| {
            p.clear(pid);
            Ok(())
        })
    }

    pub fn hotplug_register(
        &self,
        vendor: &str,
        product: &str,
        class: &str,
    ) -> Result<String, String> {
        self.require_cap(HOTPLUG)?;
        let usb_class = match class.trim().to_ascii_lowercase().as_str() {
            "hid" => drivers::UsbClass::Hid,
            "mass-storage" | "storage" => drivers::UsbClass::MassStorage,
            "serial" | "cdc-acm" => drivers::UsbClass::CdcAcm,
            _ => return Err(format!("unknown usb class: {class}")),
        };
        self.with_devices(|dm| Ok(dm.hotplug_register(vendor, product, usb_class)))
    }

    pub fn host_info(&self) -> Result<Value, String> {
        self.with_devices(|dm| {
            Ok(Value::Object(
                dm.host_info()
                    .into_iter()
                    .map(|(k, v)| (k, Value::String(v)))
                    .collect(),
            ))
        })
    }

    pub fn hw_refresh(&self) -> Result<Value, String> {
        self.with_devices(|dm| Ok(Value::Number(dm.refresh_hw() as i64)))
    }

    pub fn usb_devices(&self) -> Result<Value, String> {
        self.with_devices(|dm| {
            Ok(Value::Array(
                dm.usb
                    .list()
                    .iter()
                    .map(|u| {
                        let mut m = std::collections::HashMap::new();
                        m.insert("id".into(), Value::String(u.id.clone()));
                        m.insert("vendor".into(), Value::String(u.vendor.clone()));
                        m.insert("product".into(), Value::String(u.product.clone()));
                        m.insert("class".into(), Value::String(u.class.as_str().into()));
                        m.insert("bus".into(), Value::Number(u.bus as i64));
                        m.insert("address".into(), Value::Number(u.address as i64));
                        Value::Object(m)
                    })
                    .collect(),
            ))
        })
    }

    pub fn audio_devices(&self) -> Result<Value, String> {
        self.with_devices(|dm| {
            Ok(Value::Array(
                dm.audio
                    .list()
                    .iter()
                    .map(|a| {
                        let mut m = std::collections::HashMap::new();
                        m.insert("id".into(), Value::String(a.id.clone()));
                        m.insert("name".into(), Value::String(a.name.clone()));
                        m.insert(
                            "direction".into(),
                            Value::String(a.direction.as_str().into()),
                        );
                        m.insert("channels".into(), Value::Number(a.channels as i64));
                        m.insert("sample_rate".into(), Value::Number(a.sample_rate as i64));
                        Value::Object(m)
                    })
                    .collect(),
            ))
        })
    }
}

fn get_os(env: &Environment) -> Result<OsHandle, String> {
    let os = env.get("os").ok_or("OS handle not available")?;
    let Value::OsHandle(handle) = os else {
        return Err("OS handle not available".into());
    };
    Ok(handle)
}

/// Resolve the sandboxed OS handle from the global environment.
pub fn os_handle(env: &Environment) -> Result<OsHandle, String> {
    get_os(env)
}

fn os_info_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let _ = args;
    Ok(Value::String(get_os(env)?.info()))
}

fn os_caps_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let _ = args;
    let caps = get_os(env)?
        .capabilities()
        .into_iter()
        .map(|c| Value::String(c.to_string()))
        .collect();
    Ok(Value::Array(caps))
}

fn os_read_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let path = expect_string(args, 0, "os_read()")?;
    Ok(Value::String(get_os(env)?.read(&path)?))
}

fn os_write_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let path = expect_string(args, 0, "os_write()")?;
    let content = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => crate::value::format_value(other),
        None => String::new(),
    };
    get_os(env)?.write(&path, content)?;
    Ok(Value::Null)
}

fn os_mkdir_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let path = expect_string(args, 0, "os_mkdir()")?;
    get_os(env)?.mkdir(&path)?;
    Ok(Value::Null)
}

fn os_stat_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let path = expect_string(args, 0, "os_stat()")?;
    let stat = get_os(env)?.stat(&path)?;
    let kind = match stat.kind {
        VfsEntryKind::File => "file",
        VfsEntryKind::Directory => "dir",
    };
    Ok(Value::Array(vec![
        Value::String(kind.to_string()),
        Value::Number(stat.size as i64),
        Value::Number(stat.mtime as i64),
        Value::Bool(stat.readonly),
    ]))
}

fn os_rename_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let from = expect_string(args, 0, "os_rename()")?;
    let to = expect_string(args, 1, "os_rename()")?;
    get_os(env)?.rename(&from, &to)?;
    Ok(Value::Null)
}

fn os_copy_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let from = expect_string(args, 0, "os_copy()")?;
    let to = expect_string(args, 1, "os_copy()")?;
    get_os(env)?.copy_path(&from, &to)?;
    Ok(Value::Null)
}

fn os_mount_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let vfs_path = expect_string(args, 0, "os_mount()")?;
    let host_root = expect_string(args, 1, "os_mount()")?;
    get_os(env)?.mount_host(&vfs_path, &host_root)?;
    Ok(Value::Null)
}

fn os_unmount_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let vfs_path = expect_string(args, 0, "os_unmount()")?;
    get_os(env)?.unmount(&vfs_path)?;
    Ok(Value::Null)
}

fn os_mounts_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let mounts = get_os(env)?.list_mounts()?;
    Ok(Value::Array(
        mounts
            .into_iter()
            .map(|(vfs, host)| {
                let mut m = std::collections::HashMap::new();
                m.insert("vfs".into(), Value::String(vfs));
                m.insert("host".into(), Value::String(host));
                Value::Object(m)
            })
            .collect(),
    ))
}

fn os_exists_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let path = expect_string(args, 0, "os_exists()")?;
    Ok(Value::Bool(get_os(env)?.exists(&path)?))
}

fn os_list_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let dir = args
        .first()
        .map(|v| match v {
            Value::String(s) => s.clone(),
            _ => "/".to_string(),
        })
        .unwrap_or_else(|| "/".to_string());
    let files = get_os(env)?.list(&dir)?;
    Ok(Value::Array(files.into_iter().map(Value::String).collect()))
}

fn os_delete_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let path = expect_string(args, 0, "os_delete()")?;
    get_os(env)?.delete(&path)?;
    Ok(Value::Null)
}

fn os_spawn_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let name = expect_string(args, 0, "os_spawn()")?;
    Ok(Value::Number(get_os(env)?.spawn(&name)? as i64))
}

fn os_process_list_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let list = get_os(env)?.process_list()?;
    Ok(Value::Array(
        list.into_iter()
            .map(|p| {
                let mut m = std::collections::HashMap::new();
                m.insert("pid".into(), Value::Number(p.pid as i64));
                m.insert("name".into(), Value::String(p.name));
                m.insert(
                    "state".into(),
                    Value::String(match p.state {
                        ProcessState::Running => "running",
                        ProcessState::Stopped => "stopped",
                    }.into()),
                );
                Value::Object(m)
            })
            .collect(),
    ))
}

fn os_window_create_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let title = expect_string(args, 0, "os_window_create()")?;
    let width = args
        .get(1)
        .and_then(|v| match v {
            Value::Number(n) => Some(*n),
            _ => None,
        })
        .unwrap_or(800);
    let height = args
        .get(2)
        .and_then(|v| match v {
            Value::Number(n) => Some(*n),
            _ => None,
        })
        .unwrap_or(600);
    Ok(Value::Number(
        get_os(env)?.window_create(&title, width, height)? as i64,
    ))
}

fn os_window_list_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let list = get_os(env)?.window_list()?;
    Ok(Value::Array(
        list.into_iter()
            .map(|w| {
                let mut m = std::collections::HashMap::new();
                m.insert("id".into(), Value::Number(w.id as i64));
                m.insert("title".into(), Value::String(w.title));
                m.insert("width".into(), Value::Number(w.width));
                m.insert("height".into(), Value::Number(w.height));
                m.insert("focused".into(), Value::Bool(w.focused));
                if let Some(tab) = w.browser_tab_id {
                    m.insert("tab".into(), Value::Number(tab as i64));
                }
                Value::Object(m)
            })
            .collect(),
    ))
}

fn os_window_bind_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let win = match args.first() {
        Some(Value::Number(n)) => *n as u64,
        _ => return Err("os_window_bind() expects window id".into()),
    };
    let tab = match args.get(1) {
        Some(Value::Number(n)) => *n as u64,
        _ => return Err("os_window_bind() expects tab id".into()),
    };
    Ok(Value::Bool(get_os(env)?.window_bind_tab(win, tab)?))
}

fn os_display_register_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let win = match args.first() {
        Some(Value::Number(n)) => *n as u64,
        _ => return Err("os_display_register() expects window id".into()),
    };
    let title = expect_string(args, 1, "os_display_register()")?;
    let w = args.get(2).and_then(|v| match v { Value::Number(n) => Some(*n), _ => None }).unwrap_or(1280);
    let h = args.get(3).and_then(|v| match v { Value::Number(n) => Some(*n), _ => None }).unwrap_or(720);
    Ok(Value::Number(get_os(env)?.display_register(win, &title, w, h)? as i64))
}

fn os_mem_alloc_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let size = match args.first() {
        Some(Value::Number(n)) => *n as usize,
        _ => return Err("os_mem_alloc() expects size".into()),
    };
    let label = args.get(1).and_then(|v| match v { Value::String(s) => Some(s.as_str()), _ => None }).unwrap_or("anon");
    Ok(Value::Number(get_os(env)?.mem_alloc(size, label)? as i64))
}

fn os_mem_stats_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let (regions, used, limit) = get_os(env)?.mem_stats()?;
    Ok(Value::Array(vec![
        Value::Number(regions as i64),
        Value::Number(used as i64),
        Value::Number(limit as i64),
    ]))
}

fn os_mem_free_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let id = value_handle(args, 0, "os_mem_free()")?;
    Ok(Value::Bool(get_os(env)?.mem_free(id)?))
}

fn os_mem_write_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let id = value_handle(args, 0, "os_mem_write()")?;
    let offset = args
        .get(1)
        .and_then(|v| match v {
            Value::Number(n) if *n >= 0 => Some(*n as usize),
            _ => None,
        })
        .unwrap_or(0);
    let data: Vec<u8> = match args.get(2) {
        Some(Value::Array(vals)) => vals
            .iter()
            .map(|v| match v {
                Value::Number(n) => Ok(*n as u8),
                _ => Err("os_mem_write expects byte array".to_string()),
            })
            .collect::<Result<Vec<_>, _>>()?,
        Some(Value::String(s)) => s.as_bytes().to_vec(),
        _ => Vec::new(),
    };
    let n = get_os(env)?.mem_write(id, offset, &data)?;
    Ok(Value::Number(n as i64))
}

fn os_mem_read_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let id = value_handle(args, 0, "os_mem_read()")?;
    let offset = args
        .get(1)
        .and_then(|v| match v {
            Value::Number(n) if *n >= 0 => Some(*n as usize),
            _ => None,
        })
        .unwrap_or(0);
    let len = args
        .get(2)
        .and_then(|v| match v {
            Value::Number(n) if *n > 0 => Some(*n as usize),
            _ => None,
        })
        .unwrap_or(64);
    let buf = get_os(env)?.mem_read(id, offset, len)?;
    Ok(Value::Array(
        buf.into_iter().map(|b| Value::Number(b as i64)).collect(),
    ))
}

fn os_sched_enqueue_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let name = expect_string(args, 0, "os_sched_enqueue()")?;
    Ok(Value::Number(get_os(env)?.sched_enqueue(&name)? as i64))
}

fn os_vfs_save_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let path = expect_string(args, 0, "os_vfs_save()")?;
    get_os(env)?.vfs_save(&path)?;
    Ok(Value::Null)
}

fn os_vfs_load_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let path = expect_string(args, 0, "os_vfs_load()")?;
    get_os(env)?.vfs_load(&path)?;
    Ok(Value::Null)
}

fn os_syscall_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let name = expect_string(args, 0, "os_syscall()")?;
    let sc = syscall::Syscall::from_name(&name).ok_or_else(|| format!("Unknown syscall: {name}"))?;
    let os = get_os(env)?;
    match sc {
        syscall::Syscall::Info => Ok(Value::String(os.info())),
        syscall::Syscall::Read => {
            let path = expect_string(args, 1, "os_syscall(read)")?;
            Ok(Value::String(os.read(&path)?))
        }
        syscall::Syscall::Write => {
            let path = expect_string(args, 1, "os_syscall(write)")?;
            let content = args.get(2).and_then(|v| match v { Value::String(s) => Some(s.clone()), _ => None }).unwrap_or_default();
            os.write(&path, content)?;
            Ok(Value::Null)
        }
        syscall::Syscall::Spawn => {
            let n = expect_string(args, 1, "os_syscall(spawn)")?;
            Ok(Value::Number(os.spawn(&n)? as i64))
        }
        syscall::Syscall::Paint | syscall::Syscall::Present => Ok(Value::String("delegated-to-kbrowser".into())),
        syscall::Syscall::Sleep => Ok(Value::Number(0)),
        syscall::Syscall::DevList => Ok(device_list_value(&os.dev_list()?)),
        syscall::Syscall::DevOpen => {
            let id = expect_string(args, 1, "os_syscall(dev_open)")?;
            Ok(Value::Number(os.dev_open(&id)? as i64))
        }
        syscall::Syscall::DevClose => {
            let h = value_handle(args, 1, "os_syscall(dev_close)")?;
            os.dev_close(h)?;
            Ok(Value::Null)
        }
        syscall::Syscall::DevIoctl => {
            let h = value_handle(args, 1, "os_syscall(dev_ioctl)")?;
            let op = expect_string(args, 2, "os_syscall(dev_ioctl)")?;
            let extra = args.get(3..).unwrap_or(&[]);
            os.dev_ioctl(h, &op, extra)
        }
        syscall::Syscall::GpuInfo => os.gpu_info(),
        syscall::Syscall::NetIfaces => os.net_interfaces(),
        syscall::Syscall::NetConnect => {
            let host = expect_string(args, 1, "os_syscall(net_connect)")?;
            let port = args
                .get(2)
                .and_then(|v| match v {
                    Value::Number(n) if *n > 0 => Some(*n as u16),
                    _ => None,
                })
                .unwrap_or(80);
            Ok(Value::Number(os.net_connect(&host, port)? as i64))
        }
        syscall::Syscall::NetListen => {
            let host = args
                .get(1)
                .and_then(|v| match v {
                    Value::String(s) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| "0.0.0.0".into());
            let port = args
                .get(2)
                .and_then(|v| match v {
                    Value::Number(n) if *n > 0 => Some(*n as u16),
                    _ => None,
                })
                .unwrap_or(8080);
            Ok(Value::Number(os.net_listen(&host, port)? as i64))
        }
        syscall::Syscall::NetAccept => {
            let sock = value_handle(args, 1, "os_syscall(net_accept)")?;
            Ok(Value::Number(os.net_accept(sock)? as i64))
        }
        syscall::Syscall::NetPoll => {
            let socks: Vec<u64> = match args.get(1) {
                Some(Value::Array(vals)) => vals
                    .iter()
                    .filter_map(|v| match v {
                        Value::Number(n) if *n >= 0 => Some(*n as u64),
                        _ => None,
                    })
                    .collect(),
                Some(Value::Number(n)) if *n >= 0 => vec![*n as u64],
                _ => Vec::new(),
            };
            os.net_poll(&socks)
        }
        syscall::Syscall::NetUdpBind => {
            let host = args
                .get(1)
                .and_then(|v| match v {
                    Value::String(s) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| "0.0.0.0".into());
            let port = args
                .get(2)
                .and_then(|v| match v {
                    Value::Number(n) if *n > 0 => Some(*n as u16),
                    _ => None,
                })
                .unwrap_or(9);
            Ok(Value::Number(os.net_udp_bind(&host, port)? as i64))
        }
        syscall::Syscall::Rename => {
            let from = expect_string(args, 1, "os_syscall(rename)")?;
            let to = expect_string(args, 2, "os_syscall(rename)")?;
            os.rename(&from, &to)?;
            Ok(Value::Null)
        }
        syscall::Syscall::Copy => {
            let from = expect_string(args, 1, "os_syscall(copy)")?;
            let to = expect_string(args, 2, "os_syscall(copy)")?;
            os.copy_path(&from, &to)?;
            Ok(Value::Null)
        }
        syscall::Syscall::Mount => {
            let vfs_path = expect_string(args, 1, "os_syscall(mount)")?;
            let host_root = expect_string(args, 2, "os_syscall(mount)")?;
            os.mount_host(&vfs_path, &host_root)?;
            Ok(Value::Null)
        }
        syscall::Syscall::Unmount => {
            let vfs_path = expect_string(args, 1, "os_syscall(unmount)")?;
            os.unmount(&vfs_path)?;
            Ok(Value::Null)
        }
        syscall::Syscall::Mounts => {
            let mounts = os.list_mounts()?;
            Ok(Value::Array(
                mounts
                    .into_iter()
                    .map(|(vfs, host)| {
                        let mut m = std::collections::HashMap::new();
                        m.insert("vfs".into(), Value::String(vfs));
                        m.insert("host".into(), Value::String(host));
                        Value::Object(m)
                    })
                    .collect(),
            ))
        }
        syscall::Syscall::MemFree => {
            let id = value_handle(args, 1, "os_syscall(mem_free)")?;
            Ok(Value::Bool(os.mem_free(id)?))
        }
        syscall::Syscall::MemRead => {
            let id = value_handle(args, 1, "os_syscall(mem_read)")?;
            let offset = args
                .get(2)
                .and_then(|v| match v {
                    Value::Number(n) if *n >= 0 => Some(*n as usize),
                    _ => None,
                })
                .unwrap_or(0);
            let len = args
                .get(3)
                .and_then(|v| match v {
                    Value::Number(n) if *n > 0 => Some(*n as usize),
                    _ => None,
                })
                .unwrap_or(64);
            let buf = os.mem_read(id, offset, len)?;
            Ok(Value::Array(
                buf.into_iter().map(|b| Value::Number(b as i64)).collect(),
            ))
        }
        syscall::Syscall::MemWrite => {
            let id = value_handle(args, 1, "os_syscall(mem_write)")?;
            let offset = args
                .get(2)
                .and_then(|v| match v {
                    Value::Number(n) if *n >= 0 => Some(*n as usize),
                    _ => None,
                })
                .unwrap_or(0);
            let data: Vec<u8> = match args.get(3) {
                Some(Value::Array(vals)) => vals
                    .iter()
                    .map(|v| match v {
                        Value::Number(n) => Ok(*n as u8),
                        _ => Err("mem_write expects byte array".to_string()),
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                Some(Value::String(s)) => s.as_bytes().to_vec(),
                _ => Vec::new(),
            };
            let n = os.mem_write(id, offset, &data)?;
            Ok(Value::Number(n as i64))
        }
        syscall::Syscall::UsbList => os.usb_devices(),
        syscall::Syscall::AudioList => os.audio_devices(),
        syscall::Syscall::PermGrant => {
            let pid = value_handle(args, 1, "os_syscall(perm_grant)")?;
            let cap = expect_string(args, 2, "os_syscall(perm_grant)")?;
            os.perm_grant(pid, &cap)?;
            Ok(Value::Null)
        }
        syscall::Syscall::PermRevoke => {
            let pid = value_handle(args, 1, "os_syscall(perm_revoke)")?;
            let cap = expect_string(args, 2, "os_syscall(perm_revoke)")?;
            Ok(Value::Bool(os.perm_revoke(pid, &cap)?))
        }
        syscall::Syscall::PermList => {
            let pid = value_handle(args, 1, "os_syscall(perm_list)")?;
            let list = os.perm_list(pid)?;
            Ok(Value::Array(list.into_iter().map(Value::String).collect()))
        }
        syscall::Syscall::PermCheck => {
            let pid = value_handle(args, 1, "os_syscall(perm_check)")?;
            let cap = expect_string(args, 2, "os_syscall(perm_check)")?;
            Ok(Value::Bool(os.perm_check(pid, &cap)?))
        }
        syscall::Syscall::SetSubject => {
            let pid = value_handle(args, 1, "os_syscall(set_subject)")?;
            os.set_subject(pid)?;
            Ok(Value::Null)
        }
        syscall::Syscall::HotplugPoll => os_hotplug_poll_native(&[], env),
        syscall::Syscall::HotplugRegister => {
            let vendor = expect_string(args, 1, "os_syscall(hotplug_register)")?;
            let product = expect_string(args, 2, "os_syscall(hotplug_register)")?;
            let class = args
                .get(3)
                .and_then(|v| match v {
                    Value::String(s) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| "hid".into());
            Ok(Value::String(os.hotplug_register(&vendor, &product, &class)?))
        }
        syscall::Syscall::HostInfo => os.host_info(),
    }
}

fn os_syscalls_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(Value::Array(
        syscall::list_syscalls().into_iter().map(|s| Value::String(s.into())).collect(),
    ))
}

fn expect_string(args: &[Value], index: usize, name: &str) -> Result<String, String> {
    match args.get(index) {
        Some(Value::String(s)) => Ok(s.clone()),
        _ => Err(format!("{} expects a string path", name)),
    }
}

fn value_handle(args: &[Value], index: usize, name: &str) -> Result<u64, String> {
    match args.get(index) {
        Some(Value::Number(n)) if *n >= 0 => Ok(*n as u64),
        _ => Err(format!("{name} expects handle number")),
    }
}

fn os_dev_list_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let list = get_os(env)?.dev_list()?;
    Ok(device_list_value(&list))
}

fn os_dev_open_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let id = expect_string(args, 0, "os_dev_open()")?;
    Ok(Value::Number(get_os(env)?.dev_open(&id)? as i64))
}

fn os_dev_close_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let h = value_handle(args, 0, "os_dev_close()")?;
    get_os(env)?.dev_close(h)?;
    Ok(Value::Null)
}

fn os_dev_ioctl_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let h = value_handle(args, 0, "os_dev_ioctl()")?;
    let op = expect_string(args, 1, "os_dev_ioctl()")?;
    let extra = args.get(2..).unwrap_or(&[]);
    get_os(env)?.dev_ioctl(h, &op, extra)
}

fn os_gpu_info_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    get_os(env)?.gpu_info()
}

fn os_net_interfaces_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    get_os(env)?.net_interfaces()
}

fn os_net_connect_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let host = expect_string(args, 0, "os_net_connect()")?;
    let port = args
        .get(1)
        .and_then(|v| match v {
            Value::Number(n) if *n > 0 => Some(*n as u16),
            _ => None,
        })
        .unwrap_or(80);
    Ok(Value::Number(get_os(env)?.net_connect(&host, port)? as i64))
}

fn os_net_listen_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let host = args
        .get(0)
        .and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_else(|| "0.0.0.0".into());
    let port = args
        .get(1)
        .and_then(|v| match v {
            Value::Number(n) if *n > 0 => Some(*n as u16),
            _ => None,
        })
        .unwrap_or(8080);
    Ok(Value::Number(get_os(env)?.net_listen(&host, port)? as i64))
}

fn os_net_accept_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let sock = value_handle(args, 0, "os_net_accept()")?;
    Ok(Value::Number(get_os(env)?.net_accept(sock)? as i64))
}

fn os_net_poll_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let socks: Vec<u64> = match args.first() {
        Some(Value::Array(vals)) => vals
            .iter()
            .filter_map(|v| match v {
                Value::Number(n) if *n >= 0 => Some(*n as u64),
                _ => None,
            })
            .collect(),
        Some(Value::Number(n)) if *n >= 0 => vec![*n as u64],
        _ => Vec::new(),
    };
    get_os(env)?.net_poll(&socks)
}

fn os_net_udp_bind_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let host = args
        .get(0)
        .and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_else(|| "0.0.0.0".into());
    let port = args
        .get(1)
        .and_then(|v| match v {
            Value::Number(n) if *n > 0 => Some(*n as u16),
            _ => None,
        })
        .unwrap_or(9);
    Ok(Value::Number(get_os(env)?.net_udp_bind(&host, port)? as i64))
}

fn os_usb_devices_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    get_os(env)?.usb_devices()
}

fn os_audio_devices_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    get_os(env)?.audio_devices()
}

fn os_subject_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    Ok(Value::Number(get_os(env)?.subject()? as i64))
}

fn os_set_subject_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let pid = value_handle(args, 0, "os_set_subject()")?;
    get_os(env)?.set_subject(pid)?;
    Ok(Value::Null)
}

fn os_perm_grant_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let pid = value_handle(args, 0, "os_perm_grant()")?;
    let cap = expect_string(args, 1, "os_perm_grant()")?;
    get_os(env)?.perm_grant(pid, &cap)?;
    Ok(Value::Null)
}

fn os_perm_revoke_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let pid = value_handle(args, 0, "os_perm_revoke()")?;
    let cap = expect_string(args, 1, "os_perm_revoke()")?;
    Ok(Value::Bool(get_os(env)?.perm_revoke(pid, &cap)?))
}

fn os_perm_list_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let pid = value_handle(args, 0, "os_perm_list()")?;
    let list = get_os(env)?.perm_list(pid)?;
    Ok(Value::Array(list.into_iter().map(Value::String).collect()))
}

fn os_perm_check_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let pid = value_handle(args, 0, "os_perm_check()")?;
    let cap = expect_string(args, 1, "os_perm_check()")?;
    Ok(Value::Bool(get_os(env)?.perm_check(pid, &cap)?))
}

fn os_perm_clear_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let pid = value_handle(args, 0, "os_perm_clear()")?;
    get_os(env)?.perm_clear(pid)?;
    Ok(Value::Null)
}

fn os_hotplug_poll_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let _ = env;
    let events = hotplug::drain();
    Ok(Value::Array(
        events
            .into_iter()
            .map(|e| {
                let mut m = std::collections::HashMap::new();
                m.insert("action".into(), Value::String(e.action));
                m.insert("device_id".into(), Value::String(e.device_id));
                m.insert("kind".into(), Value::String(e.kind));
                m.insert("name".into(), Value::String(e.name));
                m.insert("vendor".into(), Value::String(e.vendor));
                Value::Object(m)
            })
            .collect(),
    ))
}

fn os_hotplug_register_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let vendor = expect_string(args, 0, "os_hotplug_register()")?;
    let product = expect_string(args, 1, "os_hotplug_register()")?;
    let class = args
        .get(2)
        .and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_else(|| "hid".into());
    Ok(Value::String(
        get_os(env)?.hotplug_register(&vendor, &product, &class)?,
    ))
}

fn os_host_info_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    get_os(env)?.host_info()
}

fn os_hw_refresh_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    get_os(env)?.hw_refresh()
}

/// Read a text file through the sandboxed OS (used by Deno parity aliases).
pub fn read_text_file(env: &mut Environment, path: &str) -> Result<String, String> {
    get_os(env)?.read(path)
}

/// Write a text file through the sandboxed OS (used by Deno parity aliases).
pub fn write_text_file(env: &mut Environment, path: &str, content: &str) -> Result<(), String> {
    get_os(env)?.write(path, content.to_string())
}

/// Current working directory on the host (native only).
pub fn host_cwd() -> Result<String, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        return std::env::current_dir()
            .map(|p| p.display().to_string())
            .map_err(|e| format!("cwd: {e}"));
    }
    #[cfg(target_arch = "wasm32")]
    {
        Err("cwd() is not available on wasm32".into())
    }
}

/// Change host working directory (native only).
pub fn host_chdir(path: &str) -> Result<(), String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        return std::env::set_current_dir(path).map_err(|e| format!("chdir: {e}"));
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = path;
        Err("chdir() is not available on wasm32".into())
    }
}

/// Canonicalize a host path (native only).
pub fn host_realpath(path: &str) -> Result<String, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let p = std::path::Path::new(path);
        let canon = p
            .canonicalize()
            .map_err(|e| format!("realpath({path}): {e}"))?;
        return Ok(canon.display().to_string());
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = path;
        Err("realpath() is not available on wasm32".into())
    }
}

/// Create a symbolic link on the host filesystem (native only).
pub fn host_symlink(target: &str, link_path: &str) -> Result<(), String> {
    #[cfg(all(not(target_arch = "wasm32"), unix))]
    {
        return std::os::unix::fs::symlink(target, link_path)
            .map_err(|e| format!("symlink({target} -> {link_path}): {e}"));
    }
    #[cfg(all(not(target_arch = "wasm32"), windows))]
    {
        return std::os::windows::fs::symlink_file(target, link_path)
            .map_err(|e| format!("symlink({target} -> {link_path}): {e}"));
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (target, link_path);
        Err("symlink() is not available on wasm32".into())
    }
    #[cfg(all(not(target_arch = "wasm32"), not(unix), not(windows)))]
    {
        let _ = (target, link_path);
        Err("symlink() is not supported on this platform".into())
    }
}

/// Create a hard link on the host filesystem (native only).
pub fn host_link(old: &str, new: &str) -> Result<(), String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        return std::fs::hard_link(old, new).map_err(|e| format!("link({old} -> {new}): {e}"));
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (old, new);
        Err("link() is not available on wasm32".into())
    }
}

/// Spawn a sandboxed process by name (Deno `run` parity).
pub fn spawn_process(env: &mut Environment, name: &str) -> Result<u64, String> {
    get_os(env)?.spawn(name)
}

pub fn os_globals(env: &mut Environment) {
    env.set("os".to_string(), Value::OsHandle(OsHandle::new()));
    env.set("os_info".to_string(), Value::NativeFunction(os_info_native));
    env.set("os_caps".to_string(), Value::NativeFunction(os_caps_native));
    env.set("os_read".to_string(), Value::NativeFunction(os_read_native));
    env.set("os_write".to_string(), Value::NativeFunction(os_write_native));
    env.set("os_mkdir".to_string(), Value::NativeFunction(os_mkdir_native));
    env.set("os_stat".to_string(), Value::NativeFunction(os_stat_native));
    env.set("os_exists".to_string(), Value::NativeFunction(os_exists_native));
    env.set("os_list".to_string(), Value::NativeFunction(os_list_native));
    env.set("os_delete".to_string(), Value::NativeFunction(os_delete_native));
    env.set("os_rename".to_string(), Value::NativeFunction(os_rename_native));
    env.set("os_copy".to_string(), Value::NativeFunction(os_copy_native));
    env.set("os_mount".to_string(), Value::NativeFunction(os_mount_native));
    env.set("os_unmount".to_string(), Value::NativeFunction(os_unmount_native));
    env.set("os_mounts".to_string(), Value::NativeFunction(os_mounts_native));
    env.set("os_spawn".to_string(), Value::NativeFunction(os_spawn_native));
    env.set(
        "os_process_list".to_string(),
        Value::NativeFunction(os_process_list_native),
    );
    env.set(
        "os_window_create".to_string(),
        Value::NativeFunction(os_window_create_native),
    );
    env.set(
        "os_window_list".to_string(),
        Value::NativeFunction(os_window_list_native),
    );
    env.set(
        "os_window_bind".to_string(),
        Value::NativeFunction(os_window_bind_native),
    );
    env.set("os_display_register".to_string(), Value::NativeFunction(os_display_register_native));
    env.set("os_mem_alloc".to_string(), Value::NativeFunction(os_mem_alloc_native));
    env.set("os_mem_free".to_string(), Value::NativeFunction(os_mem_free_native));
    env.set("os_mem_read".to_string(), Value::NativeFunction(os_mem_read_native));
    env.set("os_mem_write".to_string(), Value::NativeFunction(os_mem_write_native));
    env.set("os_mem_stats".to_string(), Value::NativeFunction(os_mem_stats_native));
    env.set("os_sched_enqueue".to_string(), Value::NativeFunction(os_sched_enqueue_native));
    env.set("os_vfs_save".to_string(), Value::NativeFunction(os_vfs_save_native));
    env.set("os_vfs_load".to_string(), Value::NativeFunction(os_vfs_load_native));
    env.set("os_syscall".to_string(), Value::NativeFunction(os_syscall_native));
    env.set("os_syscalls".to_string(), Value::NativeFunction(os_syscalls_native));
    env.set("os_dev_list".to_string(), Value::NativeFunction(os_dev_list_native));
    env.set("os_dev_open".to_string(), Value::NativeFunction(os_dev_open_native));
    env.set("os_dev_close".to_string(), Value::NativeFunction(os_dev_close_native));
    env.set("os_dev_ioctl".to_string(), Value::NativeFunction(os_dev_ioctl_native));
    env.set("os_gpu_info".to_string(), Value::NativeFunction(os_gpu_info_native));
    env.set("os_net_interfaces".to_string(), Value::NativeFunction(os_net_interfaces_native));
    env.set("os_net_connect".to_string(), Value::NativeFunction(os_net_connect_native));
    env.set("os_net_listen".to_string(), Value::NativeFunction(os_net_listen_native));
    env.set("os_net_accept".to_string(), Value::NativeFunction(os_net_accept_native));
    env.set("os_net_poll".to_string(), Value::NativeFunction(os_net_poll_native));
    env.set("os_net_udp_bind".to_string(), Value::NativeFunction(os_net_udp_bind_native));
    env.set("os_usb_devices".to_string(), Value::NativeFunction(os_usb_devices_native));
    env.set("os_audio_devices".to_string(), Value::NativeFunction(os_audio_devices_native));
    env.set("os_subject".to_string(), Value::NativeFunction(os_subject_native));
    env.set("os_set_subject".to_string(), Value::NativeFunction(os_set_subject_native));
    env.set("os_perm_grant".to_string(), Value::NativeFunction(os_perm_grant_native));
    env.set("os_perm_revoke".to_string(), Value::NativeFunction(os_perm_revoke_native));
    env.set("os_perm_list".to_string(), Value::NativeFunction(os_perm_list_native));
    env.set("os_perm_check".to_string(), Value::NativeFunction(os_perm_check_native));
    env.set("os_perm_clear".to_string(), Value::NativeFunction(os_perm_clear_native));
    env.set("os_hotplug_poll".to_string(), Value::NativeFunction(os_hotplug_poll_native));
    env.set(
        "os_hotplug_register".to_string(),
        Value::NativeFunction(os_hotplug_register_native),
    );
    env.set("os_host_info".to_string(), Value::NativeFunction(os_host_info_native));
    env.set("os_hw_refresh".to_string(), Value::NativeFunction(os_hw_refresh_native));
    os_api::register_architecture_globals(env);
    sauce_api::register_sauce_globals(env);
    features_api::register_features_globals(env);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_handle_rename_copy() {
        let os = OsHandle::new();
        os.write("/a.txt", "hello".into()).unwrap();
        os.copy_path("/a.txt", "/b.txt").unwrap();
        os.rename("/b.txt", "/c.txt").unwrap();
        assert_eq!(os.read("/c.txt").unwrap(), "hello");
    }

    #[test]
    fn os_handle_vfs_roundtrip() {
        let os = OsHandle::new();
        os.mkdir("/apps").unwrap();
        os.write("/apps/a.txt", "hello".into()).unwrap();
        assert_eq!(os.read("/apps/a.txt").unwrap(), "hello");
        assert!(os.exists("/apps").unwrap());
        assert_eq!(os.list("/apps").unwrap(), vec!["a.txt"]);
    }
}
