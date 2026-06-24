//! Write-ahead journaling for crash recovery.

use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct JournalEntry {
    pub seq: u64,
    pub path: String,
    pub bytes: usize,
}

pub struct Journal {
    next_seq: u64,
    log: VecDeque<JournalEntry>,
    committed: u64,
}

impl Default for Journal {
    fn default() -> Self {
        Self {
            next_seq: 1,
            log: VecDeque::new(),
            committed: 0,
        }
    }
}

impl Journal {
    pub fn append(&mut self, path: &str, data: &str) -> Result<u64, String> {
        let seq = self.next_seq;
        self.next_seq += 1;
        self.log.push_back(JournalEntry {
            seq,
            path: path.to_string(),
            bytes: data.len(),
        });
        Ok(seq)
    }

    pub fn commit(&mut self) -> u64 {
        if let Some(e) = self.log.back() {
            self.committed = e.seq;
        }
        self.committed
    }

    pub fn replay(&self) -> Vec<JournalEntry> {
        self.log.iter().cloned().collect()
    }
}
