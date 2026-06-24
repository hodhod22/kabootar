//! Process & thread management — PID table, threads, signals, job objects.

mod job;
mod signal;
mod thread;

pub use job::JobObject;
pub use signal::{Signal, SignalHandler};
pub use thread::{Thread, ThreadPool};

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct ProcessSubsystem {
    pub threads: ThreadPool,
    pub signals: SignalHandler,
    pub jobs: HashMap<u64, JobObject>,
    next_job: u64,
    uid_table: HashMap<u64, u32>,
    gid_table: HashMap<u64, u32>,
}

impl Default for ProcessSubsystem {
    fn default() -> Self {
        let mut ps = Self {
            threads: ThreadPool::default(),
            signals: SignalHandler::default(),
            jobs: HashMap::new(),
            next_job: 1,
            uid_table: HashMap::new(),
            gid_table: HashMap::new(),
        };
        ps.uid_table.insert(1, 0);
        ps.gid_table.insert(1, 0);
        ps.threads.spawn_main(1);
        ps
    }
}

impl ProcessSubsystem {
    pub fn spawn_thread(&mut self, pid: u64, name: &str) -> u64 {
        self.threads.spawn(pid, name)
    }

    pub fn send_signal(&mut self, pid: u64, sig: Signal) -> bool {
        self.signals.deliver(pid, sig)
    }

    pub fn create_job(&mut self, name: &str, cpu_quota: u32) -> u64 {
        let id = self.next_job;
        self.next_job += 1;
        self.jobs.insert(
            id,
            JobObject {
                id,
                name: name.to_string(),
                members: Vec::new(),
                cpu_quota,
            },
        );
        id
    }

    pub fn job_add(&mut self, job_id: u64, pid: u64) -> Result<(), String> {
        let job = self
            .jobs
            .get_mut(&job_id)
            .ok_or_else(|| format!("job not found: {job_id}"))?;
        job.members.push(pid);
        Ok(())
    }

    pub fn set_uid(&mut self, pid: u64, uid: u32) {
        self.uid_table.insert(pid, uid);
    }

    pub fn uid(&self, pid: u64) -> u32 {
        self.uid_table.get(&pid).copied().unwrap_or(65534)
    }
}
