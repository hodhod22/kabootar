//! CFS / RT scheduler — fair process prioritization.

use std::collections::BinaryHeap;
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedPolicy {
    Cfs,
    Realtime,
}

#[derive(Debug, Clone)]
pub struct SchedTask {
    pub tid: u64,
    pub pid: u64,
    pub name: String,
    pub vruntime: u64,
    pub priority: i32,
}

#[derive(Eq, PartialEq)]
struct TaskEntry {
    vruntime: u64,
    tid: u64,
}

impl Ord for TaskEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .vruntime
            .cmp(&self.vruntime)
            .then_with(|| other.tid.cmp(&self.tid))
    }
}

impl PartialOrd for TaskEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct FairScheduler {
    pub policy: SchedPolicy,
    next_tid: u64,
    runqueue: BinaryHeap<TaskEntry>,
    tasks: std::collections::HashMap<u64, SchedTask>,
    running: Option<u64>,
}

impl Default for FairScheduler {
    fn default() -> Self {
        let mut s = Self {
            policy: SchedPolicy::Cfs,
            next_tid: 1,
            runqueue: BinaryHeap::new(),
            tasks: std::collections::HashMap::new(),
            running: None,
        };
        s.enqueue_task(1, "idle");
        s
    }
}

impl FairScheduler {
    pub fn set_policy(&mut self, policy: SchedPolicy) {
        self.policy = policy;
    }

    pub fn enqueue_task(&mut self, pid: u64, name: &str) -> u64 {
        let tid = self.next_tid;
        self.next_tid += 1;
        let task = SchedTask {
            tid,
            pid,
            name: name.to_string(),
            vruntime: 0,
            priority: 0,
        };
        self.runqueue.push(TaskEntry {
            vruntime: task.vruntime,
            tid,
        });
        self.tasks.insert(tid, task);
        tid
    }

    pub fn tick(&mut self) -> Option<SchedTask> {
        let entry = self.runqueue.pop()?;
        let mut task = self.tasks.get(&entry.tid)?.clone();
        task.vruntime += match self.policy {
            SchedPolicy::Cfs => 1,
            SchedPolicy::Realtime => 0,
        };
        self.running = Some(task.tid);
        self.tasks.insert(task.tid, task.clone());
        self.runqueue.push(TaskEntry {
            vruntime: task.vruntime,
            tid: task.tid,
        });
        Some(task)
    }

    pub fn running_tid(&self) -> Option<u64> {
        self.running
    }
}
