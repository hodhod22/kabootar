//! Write-ahead journaling for crash recovery.

use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct JournalEntry {
    pub seq: u64,
    pub path: String,
    pub bytes: usize,
    pub payload: String,
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
            payload: data.to_string(),
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
        if self.committed == 0 {
            return self.log.iter().cloned().collect();
        }
        self.log
            .iter()
            .filter(|e| e.seq <= self.committed)
            .cloned()
            .collect()
    }

    /// Drop committed entries (checkpoint / truncate WAL).
    pub fn checkpoint(&mut self) -> u64 {
        let committed = self.committed;
        self.log.retain(|e| e.seq > committed);
        committed
    }

    pub fn len(&self) -> usize {
        self.log.len()
    }
}
