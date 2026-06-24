//! Pager — swap in/out between RAM and pagefile.

use std::collections::HashMap;

pub struct Pager {
    pagefile: HashMap<(u64, u64), Vec<u8>>,
    swap_slots: u64,
}

impl Default for Pager {
    fn default() -> Self {
        Self {
            pagefile: HashMap::new(),
            swap_slots: 0,
        }
    }
}

impl Pager {
    pub fn swap_out(&mut self, pid: u64, virt: u64, data: Vec<u8>) {
        self.pagefile.insert((pid, virt), data);
        self.swap_slots += 1;
    }

    pub fn swap_in(&mut self, pid: u64, virt: u64) -> Result<bool, String> {
        let page = virt & !4095;
        Ok(self.pagefile.remove(&(pid, page)).is_some())
    }

    pub fn swapped_pages(&self) -> usize {
        self.pagefile.len()
    }
}
