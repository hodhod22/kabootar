//! HAL — hardware abstraction (x86 / ARM / RISC-V).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuArch {
    X86_64,
    Arm64,
    RiscV64,
    Wasm,
    Unknown,
}

impl CpuArch {
    pub fn detect() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            return CpuArch::X86_64;
        }
        #[cfg(target_arch = "aarch64")]
        {
            return CpuArch::Arm64;
        }
        #[cfg(target_arch = "riscv64")]
        {
            return CpuArch::RiscV64;
        }
        #[cfg(target_arch = "wasm32")]
        {
            return CpuArch::Wasm;
        }
        #[cfg(not(any(
            target_arch = "x86_64",
            target_arch = "aarch64",
            target_arch = "riscv64",
            target_arch = "wasm32"
        )))]
        {
            CpuArch::Unknown
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            CpuArch::X86_64 => "x86_64",
            CpuArch::Arm64 => "arm64",
            CpuArch::RiscV64 => "riscv64",
            CpuArch::Wasm => "wasm32",
            CpuArch::Unknown => "unknown",
        }
    }
}

pub struct Hal {
    pub arch: CpuArch,
    timer_ticks: u64,
    irq_mask: u32,
}

impl Default for Hal {
    fn default() -> Self {
        Self {
            arch: CpuArch::detect(),
            timer_ticks: 0,
            irq_mask: 0,
        }
    }
}

impl Hal {
    pub fn advance_timer(&mut self) {
        self.timer_ticks += 1;
    }

    pub fn read_timer(&self) -> u64 {
        self.timer_ticks
    }

    pub fn halt_cpu(&self) -> String {
        format!("hal_halt({})", self.arch.as_str())
    }

    pub fn enable_irq(&mut self, irq: u8) {
        self.irq_mask |= 1u32 << (irq % 32);
    }
}
