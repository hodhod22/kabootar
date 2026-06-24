//! Virtual Memory Manager — page tables and address translation.

use std::collections::HashMap;

pub const PAGE_SIZE: u64 = 4096;

#[derive(Debug, Clone)]
pub struct PageEntry {
    pub virt: u64,
    pub phys: u64,
    pub perms: u8,
}

#[derive(Debug, Clone, Default)]
pub struct PageTable {
    pages: HashMap<u64, PageEntry>,
}

pub struct Vmm {
    tables: HashMap<u64, PageTable>,
    next_phys: u64,
}

impl Default for Vmm {
    fn default() -> Self {
        let mut v = Self {
            tables: HashMap::new(),
            next_phys: 0x1000_0000,
        };
        v.tables.insert(1, PageTable::default());
        v
    }
}

impl Vmm {
    pub fn map(&mut self, pid: u64, virt: u64, phys: u64, perms: u8) -> Result<(), String> {
        let table = self.tables.entry(pid).or_default();
        let page = virt & !(PAGE_SIZE - 1);
        table.pages.insert(
            page,
            PageEntry {
                virt: page,
                phys,
                perms,
            },
        );
        Ok(())
    }

    pub fn translate(&self, pid: u64, virt: u64) -> Result<u64, String> {
        let page = virt & !(PAGE_SIZE - 1);
        let off = virt & (PAGE_SIZE - 1);
        let table = self.tables.get(&pid).ok_or("no page table for pid")?;
        let entry = table.pages.get(&page).ok_or("page fault")?;
        Ok(entry.phys + off)
    }

    pub fn alloc_phys(&mut self) -> u64 {
        let p = self.next_phys;
        self.next_phys += PAGE_SIZE;
        p
    }

    pub fn mapped_pages(&self) -> usize {
        self.tables.values().map(|t| t.pages.len()).sum()
    }
}
