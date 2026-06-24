//! Cache coherence — L1/L2/L3 invalidation on shared mappings.

use std::collections::HashSet;

pub struct CacheCoherence {
    invalidated: HashSet<u64>,
    flushes: u64,
}

impl Default for CacheCoherence {
    fn default() -> Self {
        Self {
            invalidated: HashSet::new(),
            flushes: 0,
        }
    }
}

impl CacheCoherence {
    pub fn invalidate_line(&mut self, addr: u64) {
        let line = addr & !63;
        self.invalidated.insert(line);
        self.flushes += 1;
    }

    pub fn flush_all(&mut self) {
        self.invalidated.clear();
        self.flushes += 1;
    }

    pub fn flush_count(&self) -> u64 {
        self.flushes
    }
}
