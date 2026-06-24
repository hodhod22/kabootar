//! LRU buffer pool over pages — mmap-ready (Phase 2).

use super::pages::{Page, PageKind, PAGE_SIZE};
use std::collections::{HashMap, VecDeque};

const DEFAULT_POOL: usize = 256;

#[derive(Debug, Clone)]
pub struct BufferPool {
    pages: HashMap<u64, Page>,
    lru: VecDeque<u64>,
    capacity: usize,
    next_page_id: u64,
    pub file_path: Option<String>,
}

impl Default for BufferPool {
    fn default() -> Self {
        Self::new(DEFAULT_POOL)
    }
}

impl BufferPool {
    pub fn new(capacity: usize) -> Self {
        Self {
            pages: HashMap::new(),
            lru: VecDeque::new(),
            capacity,
            next_page_id: 1,
            file_path: None,
        }
    }

    pub fn alloc_page(&mut self, kind: PageKind) -> u64 {
        let id = self.next_page_id;
        self.next_page_id += 1;
        let mut page = Page::new(id, kind);
        page.encode_header();
        self.insert_page(page);
        id
    }

    pub fn get_page(&mut self, id: u64) -> Option<&Page> {
        self.touch(id);
        self.pages.get(&id)
    }

    pub fn get_page_mut(&mut self, id: u64) -> Option<&mut Page> {
        self.touch(id);
        if let Some(p) = self.pages.get_mut(&id) {
            p.dirty = true;
        }
        self.pages.get_mut(&id)
    }

    pub fn insert_page(&mut self, page: Page) {
        let id = page.id;
        if self.pages.len() >= self.capacity && !self.pages.contains_key(&id) {
            self.evict_one();
        }
        self.pages.insert(id, page);
        self.touch(id);
    }

    pub fn dirty_pages(&self) -> Vec<u64> {
        self.pages
            .iter()
            .filter(|(_, p)| p.dirty)
            .map(|(id, _)| *id)
            .collect()
    }

    pub fn mark_clean(&mut self, id: u64) {
        if let Some(p) = self.pages.get_mut(&id) {
            p.dirty = false;
        }
    }

    pub fn page_bytes(&self, id: u64) -> Option<Vec<u8>> {
        self.pages.get(&id).map(|p| p.data.clone())
    }

    pub fn load_page_bytes(&mut self, id: u64, bytes: Vec<u8>) -> Result<(), String> {
        if bytes.len() != PAGE_SIZE {
            return Err(format!("Expected {PAGE_SIZE} byte page"));
        }
        let (_, kind) = Page::decode_header(&bytes)?;
        let mut page = Page::new(id, kind);
        page.data = bytes;
        self.insert_page(page);
        Ok(())
    }

    fn touch(&mut self, id: u64) {
        if let Some(pos) = self.lru.iter().position(|&x| x == id) {
            self.lru.remove(pos);
        }
        self.lru.push_back(id);
    }

    fn evict_one(&mut self) {
        while let Some(id) = self.lru.pop_front() {
            if let Some(page) = self.pages.get(&id) {
                if page.pin_count == 0 && !page.dirty {
                    self.pages.remove(&id);
                    return;
                }
                self.lru.push_back(id);
            }
        }
    }
}
