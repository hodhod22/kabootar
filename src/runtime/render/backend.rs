//! Render backend selection — CPU raster or GPU texture upload (wgpu).

use std::sync::atomic::{AtomicU8, Ordering};

const CPU: u8 = 0;
const GPU: u8 = 1;

static BACKEND: AtomicU8 = AtomicU8::new(CPU);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderBackend {
    Cpu,
    Gpu,
}

impl RenderBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Gpu => "gpu",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "cpu" => Some(Self::Cpu),
            "gpu" => Some(Self::Gpu),
            _ => None,
        }
    }
}

pub fn active_backend() -> RenderBackend {
    match BACKEND.load(Ordering::SeqCst) {
        GPU => RenderBackend::Gpu,
        _ => RenderBackend::Cpu,
    }
}

pub fn set_backend(backend: RenderBackend) {
    BACKEND.store(
        match backend {
            RenderBackend::Cpu => CPU,
            RenderBackend::Gpu => GPU,
        },
        Ordering::SeqCst,
    );
}

pub fn resolve_backend(requested: RenderBackend, gpu_available: bool) -> RenderBackend {
    match requested {
        RenderBackend::Gpu if gpu_available => RenderBackend::Gpu,
        _ => RenderBackend::Cpu,
    }
}
