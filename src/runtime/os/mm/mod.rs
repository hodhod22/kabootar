//! Memory Management Unit — VMM, pager, cache coherence, allocator.

mod allocator;
mod cache;
mod pager;
mod vmm;

pub use allocator::{HeapAllocator, StackFrame};
pub use cache::CacheCoherence;
pub use pager::Pager;
pub use vmm::{PageTable, Vmm};

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
