//! Block I/O scheduler — NVMe vs rotational disk ordering.

use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub enum BlockOp {
    Read { path: String, len: usize },
    Write { path: String, len: usize },
}

pub struct BlockIoScheduler {
    queue: VecDeque<BlockOp>,
    nvme_mode: bool,
}

impl Default for BlockIoScheduler {
    fn default() -> Self {
        Self {
            queue: VecDeque::new(),
            nvme_mode: true,
        }
    }
}

impl BlockIoScheduler {
    pub fn enqueue_write(&mut self, path: &str, len: usize) {
        self.queue.push_back(BlockOp::Write {
            path: path.to_string(),
            len,
        });
    }

    pub fn enqueue_read(&mut self, path: &str, len: usize) {
        self.queue.push_front(BlockOp::Read {
            path: path.to_string(),
            len,
        });
    }

    pub fn next(&mut self) -> Option<BlockOp> {
        self.queue.pop_front()
    }

    pub fn pending(&self) -> usize {
        self.queue.len()
    }
}
