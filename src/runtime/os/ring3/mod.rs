//! Ring 3 — shell, init, libc, subsystem compatibility.

mod init;
mod libc;
mod shell;
mod subsystem;

pub use init::InitProcess;
pub use libc::Libc;
pub use shell::Shell;
pub use subsystem::{OsSubsystem, SubsystemKind};

pub struct Userland {
    pub init: InitProcess,
    pub shell: Shell,
    pub libc: Libc,
    pub subsystems: Vec<OsSubsystem>,
}

impl Default for Userland {
    fn default() -> Self {
        Self {
            init: InitProcess::default(),
            shell: Shell::default(),
            libc: Libc::default(),
            subsystems: vec![
                OsSubsystem::new(SubsystemKind::Posix),
                OsSubsystem::new(SubsystemKind::Wsl),
            ],
        }
    }
}

impl Userland {
    pub fn run_command(&mut self, line: &str) -> String {
        self.shell.exec(line)
    }
}
