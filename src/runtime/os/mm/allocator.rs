//! Heap/stack allocator — per-process user allocations.

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct StackFrame {
    pub base: u64,
    pub size: u64,
}

pub struct HeapAllocator {
    heaps: HashMap<u64, HashMap<u64, usize>>,
    stacks: HashMap<u64, StackFrame>,
    next_addr: u64,
    total: u64,
}

impl Default for HeapAllocator {
    fn default() -> Self {
        Self {
            heaps: HashMap::new(),
            stacks: HashMap::new(),
            next_addr: 0x4000_0000,
            total: 0,
        }
    }
}

impl HeapAllocator {
    pub fn alloc(&mut self, pid: u64, size: usize) -> Result<u64, String> {
        if size == 0 {
            return Err("alloc size must be > 0".into());
        }
        let addr = self.next_addr;
        self.next_addr += (size as u64 + 15) & !15;
        self.heaps
            .entry(pid)
            .or_default()
            .insert(addr, size);
        self.total += size as u64;
        Ok(addr)
    }

    pub fn free(&mut self, pid: u64, addr: u64) -> bool {
        if let Some(h) = self.heaps.get_mut(&pid) {
            if let Some(sz) = h.remove(&addr) {
                self.total = self.total.saturating_sub(sz as u64);
                return true;
            }
        }
        false
    }

    pub fn alloc_stack(&mut self, tid: u64, size: u64) -> StackFrame {
        let frame = StackFrame {
            base: 0x7fff_0000 - tid * 0x10000,
            size,
        };
        self.stacks.insert(tid, frame.clone());
        frame
    }

    pub fn total_allocated(&self) -> u64 {
        self.total
    }
}
