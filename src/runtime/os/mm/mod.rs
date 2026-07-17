//! Memory Management Unit — VMM, pager, cache coherence, allocator.

mod allocator;
mod cache;
mod pager;
mod vmm;

pub use allocator::HeapAllocator;
pub use cache::CacheCoherence;
pub use pager::Pager;
pub use vmm::{PageEntry, Vmm, PAGE_SIZE};

use std::sync::atomic::{AtomicU64, Ordering};

pub struct MemorySubsystem {
    pub vmm: Vmm,
    pub pager: Pager,
    pub cache: CacheCoherence,
    pub allocator: HeapAllocator,
    pub page_faults: AtomicU64,
}

impl Default for MemorySubsystem {
    fn default() -> Self {
        Self {
            vmm: Vmm::default(),
            pager: Pager::default(),
            cache: CacheCoherence::default(),
            allocator: HeapAllocator::default(),
            page_faults: AtomicU64::new(0),
        }
    }
}

impl MemorySubsystem {
    pub fn map_page(&mut self, pid: u64, virt: u64, phys: u64, perms: u8) -> Result<(), String> {
        self.vmm.map(pid, virt, phys, perms)?;
        self.cache.invalidate_line(virt);
        Ok(())
    }

    pub fn translate(&mut self, pid: u64, virt: u64) -> Result<u64, String> {
        match self.vmm.translate(pid, virt) {
            Ok(p) => Ok(p),
            Err(e) => {
                self.page_faults.fetch_add(1, Ordering::SeqCst);
                if self.pager.swap_in(pid, virt)? {
                    self.vmm.translate(pid, virt)
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Demand-zero page fault: map a fresh page if missing.
    pub fn fault(&mut self, pid: u64, virt: u64) -> Result<u64, String> {
        match self.vmm.translate(pid, virt) {
            Ok(p) => Ok(p),
            Err(_) => {
                self.page_faults.fetch_add(1, Ordering::SeqCst);
                let phys = self.vmm.alloc_phys();
                let page = virt & !(PAGE_SIZE - 1);
                self.map_page(pid, page, phys, 7)?;
                Ok(phys + (virt & (PAGE_SIZE - 1)))
            }
        }
    }

    /// Anonymous mmap — map `len` bytes starting at `virt` (page-aligned).
    pub fn mmap(&mut self, pid: u64, virt: u64, len: u64, perms: u8) -> Result<u64, String> {
        if len == 0 {
            return Err("os_mm_mmap: len must be > 0".into());
        }
        let start = virt & !(PAGE_SIZE - 1);
        let pages = (len + PAGE_SIZE - 1) / PAGE_SIZE;
        for i in 0..pages {
            let v = start + i * PAGE_SIZE;
            let phys = self.vmm.alloc_phys();
            self.map_page(pid, v, phys, perms)?;
        }
        Ok(start)
    }

    /// Share a page COW from src_pid to dst_pid (same phys, both marked cow).
    pub fn cow_share(&mut self, src_pid: u64, dst_pid: u64, virt: u64) -> Result<u64, String> {
        let mut entry = self.vmm.entry(src_pid, virt)?;
        entry.cow = true;
        self.vmm.map_entry(src_pid, entry.clone())?;
        self.vmm.map_entry(dst_pid, entry.clone())?;
        Ok(entry.phys)
    }

    /// Break COW on write: allocate private phys if page is shared.
    pub fn cow_break(&mut self, pid: u64, virt: u64) -> Result<u64, String> {
        let entry = self.vmm.entry(pid, virt)?;
        if !entry.cow {
            return Ok(entry.phys);
        }
        let phys = self.vmm.alloc_phys();
        self.vmm.map_entry(
            pid,
            PageEntry {
                virt: entry.virt,
                phys,
                perms: entry.perms,
                cow: false,
            },
        )?;
        self.cache.invalidate_line(virt);
        Ok(phys)
    }

    pub fn malloc(&mut self, pid: u64, size: usize) -> Result<u64, String> {
        self.allocator.alloc(pid, size)
    }

    pub fn free(&mut self, pid: u64, addr: u64) -> bool {
        self.allocator.free(pid, addr)
    }

    pub fn stats(&self) -> (usize, usize, u64, u64) {
        (
            self.vmm.mapped_pages(),
            self.pager.swapped_pages(),
            self.page_faults.load(Ordering::SeqCst),
            self.allocator.total_allocated(),
        )
    }
}
