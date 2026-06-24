//! Filesystem subsystem — journal, block I/O scheduler, page cache.

mod block_io;
mod journal;
mod page_cache;

pub use block_io::BlockIoScheduler;
pub use journal::Journal;
pub use page_cache::PageCache;

pub struct FsSubsystem {
    pub journal: Journal,
    pub block_io: BlockIoScheduler,
    pub page_cache: PageCache,
}

impl Default for FsSubsystem {
    fn default() -> Self {
        Self {
            journal: Journal::default(),
            block_io: BlockIoScheduler::default(),
            page_cache: PageCache::default(),
        }
    }
}

impl FsSubsystem {
    pub fn write_with_journal(&mut self, path: &str, data: &str) -> Result<u64, String> {
        let seq = self.journal.append(path, data)?;
        self.page_cache.put(path, data.as_bytes());
        self.block_io.enqueue_write(path, data.len());
        Ok(seq)
    }

    pub fn read_cached(&mut self, path: &str) -> Option<Vec<u8>> {
        self.page_cache.get(path)
    }

    pub fn reset_after_vfs_snapshot(&mut self) {
        self.page_cache.clear();
        self.journal = Journal::default();
        self.block_io = BlockIoScheduler::default();
    }
}
