//! Kabootar OS task scheduler — cooperative micro-scheduler.

use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct OsTask {
    pub id: u64,
    pub name: String,
    pub ticks: u64,
}

pub struct Scheduler {
    next_id: u64,
    queue: VecDeque<OsTask>,
    running: Option<u64>,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self {
            next_id: 1,
            queue: VecDeque::new(),
            running: None,
        }
    }
}

impl Scheduler {
    pub fn enqueue(&mut self, name: &str) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.queue.push_back(OsTask {
            id,
            name: name.to_string(),
            ticks: 0,
        });
        id
    }

    pub fn tick(&mut self) -> Option<OsTask> {
        if let Some(front) = self.queue.front_mut() {
            front.ticks += 1;
            self.running = Some(front.id);
            return Some(front.clone());
        }
        None
    }

    pub fn complete(&mut self, id: u64) -> bool {
        if self.queue.front().map(|t| t.id) == Some(id) {
            self.queue.pop_front();
            self.running = self.queue.front().map(|t| t.id);
            return true;
        }
        false
    }

    pub fn pending(&self) -> usize {
        self.queue.len()
    }
}
