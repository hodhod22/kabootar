//! OS subsystems — POSIX / WSL compatibility layers.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubsystemKind {
    Posix,
    Wsl,
    Win32,
}

#[derive(Debug, Clone)]
pub struct OsSubsystem {
    pub kind: SubsystemKind,
    pub active: bool,
}

impl OsSubsystem {
    pub fn new(kind: SubsystemKind) -> Self {
        Self { kind, active: true }
    }

    pub fn name(&self) -> &'static str {
        match self.kind {
            SubsystemKind::Posix => "posix",
            SubsystemKind::Wsl => "wsl",
            SubsystemKind::Win32 => "win32",
        }
    }
}
