//! Page cache — RAM cache for frequently accessed files.

use std::collections::HashMap;

pub struct PageCache {
    pages: HashMap<String, Vec<u8>>,
    hits: u64,
    misses: u64,
    max_pages: usize,
}

impl Default for PageCache {
    fn default() -> Self {
        Self {
            pages: HashMap::new(),
            hits: 0,
            misses: 0,
            max_pages: 256,
        }
    }
}

impl PageCache {
    pub fn get(&mut self, path: &str) -> Option<Vec<u8>> {
        if let Some(data) = self.pages.get(path) {
            self.hits += 1;
            return Some(data.clone());
        }
        self.misses += 1;
        None
    }

    pub fn put(&mut self, path: &str, data: &[u8]) {
        if self.pages.len() >= self.max_pages {
            if let Some(k) = self.pages.keys().next().cloned() {
                self.pages.remove(&k);
            }
        }
        self.pages.insert(path.to_string(), data.to_vec());
    }

    pub fn stats(&self) -> (u64, u64, usize) {
        (self.hits, self.misses, self.pages.len())
    }

    pub fn clear(&mut self) {
        self.pages.clear();
        self.hits = 0;
        self.misses = 0;
    }
}
