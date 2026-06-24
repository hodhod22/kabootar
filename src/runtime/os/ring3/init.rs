//! Init process — PID 1 parent of all user processes.

#[derive(Debug, Clone)]
pub struct InitProcess {
    pub pid: u64,
    pub name: String,
    pub children: Vec<u64>,
}

impl Default for InitProcess {
    fn default() -> Self {
        Self {
            pid: 1,
            name: "kabootar-init".into(),
            children: Vec::new(),
        }
    }
}

impl InitProcess {
    pub fn adopt(&mut self, child_pid: u64) {
        self.children.push(child_pid);
    }
}
