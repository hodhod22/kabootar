//! Kabootar OS GPU / display driver — framebuffer + optional wgpu backend.

use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone)]
pub struct GpuMode {
    pub width: u32,
    pub height: u32,
    pub bpp: u8,
}

#[derive(Debug, Clone)]
pub struct GpuDriverInfo {
    pub device: String,
    pub backend: String,
    pub available: bool,
    pub vram_mb: u32,
    pub mode: GpuMode,
    pub present_count: u64,
    pub last_texture: Option<u64>,
}

pub struct GpuDriver {
    next_texture: AtomicU64,
    mode: GpuMode,
    present_count: u64,
    last_texture: Option<u64>,
}

impl Default for GpuDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl GpuDriver {
    pub fn new() -> Self {
        #[cfg(feature = "gpu")]
        crate::runtime::render::probe_gpu();

        Self {
            next_texture: AtomicU64::new(1),
            mode: GpuMode {
                width: 1280,
                height: 720,
                bpp: 32,
            },
            present_count: 0,
            last_texture: None,
        }
    }

    pub fn info(&self) -> GpuDriverInfo {
        let (available, device, backend) = gpu_probe();
        GpuDriverInfo {
            device,
            backend,
            available,
            vram_mb: if available { 256 } else { 64 },
            mode: self.mode.clone(),
            present_count: self.present_count,
            last_texture: self.last_texture,
        }
    }

    pub fn set_mode(&mut self, width: u32, height: u32) -> Result<(), String> {
        if width == 0 || height == 0 {
            return Err("gpu mode requires non-zero width and height".into());
        }
        if width > 8192 || height > 8192 {
            return Err("gpu mode exceeds maximum 8192×8192".into());
        }
        self.mode.width = width;
        self.mode.height = height;
        Ok(())
    }

    pub fn present(&mut self, rgba: &[u8]) -> Result<u64, String> {
        let w = self.mode.width;
        let h = self.mode.height;
        let need = (w * h * 4) as usize;
        if rgba.len() < need {
            return Err(format!("gpu present: rgba too small ({} < {need})", rgba.len()));
        }
        self.present_count += 1;

        #[cfg(feature = "gpu")]
        {
            if let Ok(tex) = crate::runtime::render::upload_rgba(w, h, rgba) {
                self.last_texture = Some(tex);
                return Ok(tex);
            }
        }

        let id = self.next_texture.fetch_add(1, Ordering::SeqCst);
        self.last_texture = Some(id);
        Ok(id)
    }

    pub fn present_bytes(&mut self, byte_count: usize) -> Result<u64, String> {
        let w = self.mode.width;
        let h = self.mode.height;
        let frame = (0..(w * h))
            .flat_map(|i| {
                let v = ((i % 256) as u8).wrapping_add(byte_count as u8);
                [v, v, v, 255]
            })
            .collect::<Vec<_>>();
        self.present(&frame)
    }
}

fn gpu_probe() -> (bool, String, String) {
    let info = crate::runtime::render::gpu_info_map();
    let available = info
        .get("available")
        .map(|s| s == "true")
        .unwrap_or(false);
    let device = info
        .get("device")
        .cloned()
        .unwrap_or_else(|| "Kabootar Virtual GPU".into());
    let backend = info
        .get("backend")
        .cloned()
        .unwrap_or_else(|| "cpu-fallback".into());
    (available, device, backend)
}
