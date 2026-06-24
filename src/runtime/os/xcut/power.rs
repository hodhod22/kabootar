//! Power management — ACPI C/P/S states.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcpiState {
    Running,
    Idle,
    Sleep,
    Hibernate,
    Off,
}

pub struct PowerManager {
    pub state: AcpiState,
    pub c_state: u8,
    pub p_state: u8,
    wake_on_lan: bool,
}

impl Default for PowerManager {
    fn default() -> Self {
        Self {
            state: AcpiState::Running,
            c_state: 0,
            p_state: 0,
            wake_on_lan: false,
        }
    }
}

impl PowerManager {
    pub fn set_c_state(&mut self, c: u8) {
        self.c_state = c;
        if c > 0 {
            self.state = AcpiState::Idle;
        }
    }

    pub fn suspend(&mut self) {
        self.state = AcpiState::Sleep;
    }

    pub fn resume(&mut self) {
        self.state = AcpiState::Running;
    }

    pub fn enable_wol(&mut self, on: bool) {
        self.wake_on_lan = on;
    }
}
