//! Kabootar kernel subsystem registry — all 7 parts + cross-cutting.

use super::fsys::FsSubsystem;
use super::iosys::IoSubsystem;
use super::kcore::KernelCore;
use super::mm::MemorySubsystem;
use super::netstack::NetStackSubsystem;
use super::proc2::ProcessSubsystem;
use super::ring3::Userland;
use super::sauce::SauceSubsystem;
use super::xcut::CrosscutSubsystem;
use std::sync::{Arc, Mutex};

/// Complete kernel architecture state (Ring 0 + subsystems).
pub struct KernelSubsystems {
    pub kcore: KernelCore,
    pub mm: MemorySubsystem,
    pub proc2: ProcessSubsystem,
    pub iosys: IoSubsystem,
    pub fsys: FsSubsystem,
    pub netstack: NetStackSubsystem,
    pub ring3: Userland,
    pub xcut: CrosscutSubsystem,
    pub sauce: SauceSubsystem,
}

impl Default for KernelSubsystems {
    fn default() -> Self {
        Self {
            kcore: KernelCore::default(),
            mm: MemorySubsystem::default(),
            proc2: ProcessSubsystem::default(),
            iosys: IoSubsystem::default(),
            fsys: FsSubsystem::default(),
            netstack: NetStackSubsystem::default(),
            ring3: Userland::default(),
            xcut: CrosscutSubsystem::default(),
            sauce: SauceSubsystem::default(),
        }
    }
}

impl KernelSubsystems {
    pub fn architecture_map(&self) -> std::collections::HashMap<String, String> {
        let mut m = std::collections::HashMap::new();
        m.insert("part1_microkernel".into(), "ipc+address-spaces".into());
        m.insert("part1_executive".into(), "objects+io".into());
        m.insert("part1_hal".into(), self.kcore.hal.arch.as_str().into());
        m.insert("part1_scheduler".into(), "cfs".into());
        m.insert("part1_dispatcher".into(), "context-switch".into());
        m.insert("part2_vmm".into(), format!("{} pages", self.mm.vmm.mapped_pages()));
        m.insert("part2_pager".into(), format!("{} swapped", self.mm.pager.swapped_pages()));
        m.insert("part2_cache".into(), format!("{} flushes", self.mm.cache.flush_count()));
        m.insert("part3_threads".into(), self.proc2.threads.count().to_string());
        m.insert("part3_signals".into(), "dispatcher".into());
        m.insert("part3_jobs".into(), self.proc2.jobs.len().to_string());
        m.insert("part4_drivers".into(), self.iosys.framework.list().len().to_string());
        m.insert("part4_pnp".into(), "usb+pci".into());
        m.insert("part4_irq".into(), self.iosys.irq.handled_count().to_string());
        m.insert("part4_dma".into(), self.iosys.dma.len().to_string());
        m.insert("part5_journal".into(), "wal".into());
        m.insert("part5_block_io".into(), self.fsys.block_io.pending().to_string());
        m.insert(
            "part5_page_cache".into(),
            format!("{:?}", self.fsys.page_cache.stats()),
        );
        m.insert("part6_stack".into(), self.netstack.stack.packet_count().to_string());
        m.insert("part6_qos".into(), "traffic-control".into());
        m.insert("part7_init".into(), self.ring3.init.name.clone());
        m.insert("part7_shell".into(), "kabootar-sh".into());
        m.insert("part7_libc".into(), "kabootar-libc".into());
        m.insert("part7_subsystems".into(), "posix+wsl".into());
        m.insert("xcut_security".into(), "srm+acl".into());
        m.insert("xcut_error".into(), format!("{} crashes", self.xcut.error.crash_count()));
        m.insert("xcut_log".into(), self.xcut.log.total().to_string());
        m.insert(
            "xcut_power".into(),
            format!("{:?}", self.xcut.power.state as u8),
        );
        m
    }
}

pub type SharedSubsystems = Arc<Mutex<KernelSubsystems>>;

pub fn new_shared() -> SharedSubsystems {
    Arc::new(Mutex::new(KernelSubsystems::default()))
}
