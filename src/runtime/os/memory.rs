//! Kabootar OS memory regions — guarded heap with bounds checking.

use std::collections::HashMap;

const GUARD_BYTE: u8 = 0xDE;
const GUARD_LEN: usize = 8;

#[derive(Debug)]
struct Region {
    id: u64,
    label: String,
    storage: Vec<u8>,
    payload_off: usize,
    payload_len: usize,
}

#[derive(Debug, Clone)]
pub struct MemoryRegion {
    pub id: u64,
    pub size: usize,
    pub label: String,
}

pub struct MemoryManager {
    next_id: u64,
    regions: HashMap<u64, Region>,
    total_allocated: usize,
    pub limit: usize,
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self {
            next_id: 1,
            regions: HashMap::new(),
            total_allocated: 0,
            limit: 64 * 1024 * 1024,
        }
    }
}

impl MemoryManager {
    fn check_guards(r: &Region) -> Result<(), String> {
        let start = r.payload_off.saturating_sub(GUARD_LEN);
        let end = r.payload_off + r.payload_len;
        for i in start..r.payload_off {
            if r.storage.get(i) != Some(&GUARD_BYTE) {
                return Err(format!("memory guard corrupted (head) in region {}", r.id));
            }
        }
        for i in end..end + GUARD_LEN {
            if r.storage.get(i) != Some(&GUARD_BYTE) {
                return Err(format!("memory guard corrupted (tail) in region {}", r.id));
            }
        }
        Ok(())
    }

    pub fn alloc(&mut self, size: usize, label: &str) -> Result<u64, String> {
        if size == 0 {
            return Err("alloc size must be > 0".into());
        }
        if self.total_allocated.saturating_add(size) > self.limit {
            return Err("OS memory limit exceeded".into());
        }
        let id = self.next_id;
        self.next_id += 1;
        let payload_off = GUARD_LEN;
        let total = GUARD_LEN + size + GUARD_LEN;
        let mut storage = vec![GUARD_BYTE; total];
        storage[payload_off..payload_off + size].fill(0);
        self.regions.insert(
            id,
            Region {
                id,
                label: label.to_string(),
                storage,
                payload_off,
                payload_len: size,
            },
        );
        self.total_allocated += size;
        Ok(id)
    }

    pub fn free(&mut self, id: u64) -> bool {
        if let Some(mut r) = self.regions.remove(&id) {
            for b in &mut r.storage {
                *b = 0;
            }
            self.total_allocated = self.total_allocated.saturating_sub(r.payload_len);
            true
        } else {
            false
        }
    }

    pub fn write(&mut self, id: u64, offset: usize, data: &[u8]) -> Result<usize, String> {
        let r = self
            .regions
            .get_mut(&id)
            .ok_or_else(|| format!("invalid memory region: {id}"))?;
        Self::check_guards(r)?;
        if offset.saturating_add(data.len()) > r.payload_len {
            return Err(format!(
                "memory write out of bounds: offset {offset} + {} > {}",
                data.len(),
                r.payload_len
            ));
        }
        let start = r.payload_off + offset;
        r.storage[start..start + data.len()].copy_from_slice(data);
        Ok(data.len())
    }

    pub fn read(&mut self, id: u64, offset: usize, len: usize) -> Result<Vec<u8>, String> {
        let r = self
            .regions
            .get(&id)
            .ok_or_else(|| format!("invalid memory region: {id}"))?;
        Self::check_guards(r)?;
        if offset.saturating_add(len) > r.payload_len {
            return Err(format!(
                "memory read out of bounds: offset {offset} + {len} > {}",
                r.payload_len
            ));
        }
        let start = r.payload_off + offset;
        Ok(r.storage[start..start + len].to_vec())
    }

    pub fn region_info(&self, id: u64) -> Option<MemoryRegion> {
        self.regions.get(&id).map(|r| MemoryRegion {
            id: r.id,
            size: r.payload_len,
            label: r.label.clone(),
        })
    }

    pub fn stats(&self) -> (usize, usize, usize) {
        (self.regions.len(), self.total_allocated, self.limit)
    }
}
