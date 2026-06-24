//! wgpu GPU compositor — uploads RGBA frames to GPU textures (layer 2).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub available: bool,
    pub device: String,
    pub backend: String,
    pub uploads: u64,
}

static INFO: OnceLock<Mutex<GpuInfo>> = OnceLock::new();

fn info_slot() -> &'static Mutex<GpuInfo> {
    INFO.get_or_init(|| {
        Mutex::new(GpuInfo {
            available: false,
            device: "none".into(),
            backend: "cpu".into(),
            uploads: 0,
        })
    })
}

pub fn gpu_available() -> bool {
    info_slot().lock().map(|g| g.available).unwrap_or(false)
}

pub fn gpu_info() -> GpuInfo {
    info_slot()
        .lock()
        .map(|g| g.clone())
        .unwrap_or_else(|_| GpuInfo {
            available: false,
            device: "none".into(),
            backend: "cpu".into(),
            uploads: 0,
        })
}

pub fn gpu_info_map() -> HashMap<String, String> {
    let g = gpu_info();
    let mut m = HashMap::new();
    m.insert("available".into(), g.available.to_string());
    m.insert("device".into(), g.device);
    m.insert("backend".into(), g.backend);
    m.insert("uploads".into(), g.uploads.to_string());
    m
}

#[cfg(feature = "gpu")]
mod imp {
    use super::{info_slot, GpuInfo};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Mutex, OnceLock};

    struct GpuContext {
        device: wgpu::Device,
        queue: wgpu::Queue,
        textures: Vec<wgpu::Texture>,
    }

    static CTX: OnceLock<Mutex<Option<GpuContext>>> = OnceLock::new();
    static NEXT_ID: AtomicU64 = AtomicU64::new(1);

    fn ctx_slot() -> &'static Mutex<Option<GpuContext>> {
        CTX.get_or_init(|| Mutex::new(None))
    }

    fn ensure_context() -> Result<(), String> {
        let mut guard = ctx_slot().lock().map_err(|_| "gpu lock poisoned".to_string())?;
        if guard.is_some() {
            return Ok(());
        }
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .ok_or("no wgpu adapter")?;

        let device_name = adapter.get_info().name.clone();
        let backend_name = format!("{:?}", adapter.get_info().backend);

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("kabootar-gpu"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        ))
        .map_err(|e| format!("wgpu device: {e}"))?;

        *guard = Some(GpuContext {
            device,
            queue,
            textures: Vec::new(),
        });

        if let Ok(mut info) = info_slot().lock() {
            info.available = true;
            info.device = device_name;
            info.backend = backend_name;
        }
        Ok(())
    }

    fn pad_rgba(width: u32, height: u32, rgba: &[u8]) -> (Vec<u8>, u32) {
        let unpadded = width * 4;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bpr = ((unpadded + align - 1) / align) * align;
        if padded_bpr == unpadded {
            return (rgba.to_vec(), unpadded);
        }
        let mut out = vec![0u8; (padded_bpr * height) as usize];
        for y in 0..height as usize {
            let src = y * unpadded as usize;
            let dst = y * padded_bpr as usize;
            out[dst..dst + unpadded as usize].copy_from_slice(&rgba[src..src + unpadded as usize]);
        }
        (out, padded_bpr)
    }

    pub fn upload_rgba(width: u32, height: u32, rgba: &[u8]) -> Result<u64, String> {
        let expected = (width * height * 4) as usize;
        if rgba.len() < expected {
            return Err(format!("rgba buffer too small: {} < {expected}", rgba.len()));
        }
        ensure_context()?;
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        let (data, bpr) = pad_rgba(width, height, rgba);
        let mut guard = ctx_slot().lock().map_err(|_| "gpu lock poisoned".to_string())?;
        let ctx = guard.as_mut().ok_or("gpu context missing")?;
        let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("kabootar-frame"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        ctx.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(bpr),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        ctx.textures.push(texture);
        if let Ok(mut info) = info_slot().lock() {
            info.uploads += 1;
        }
        Ok(id)
    }

    pub fn with_gpu<F, T>(f: F) -> Result<T, String>
    where
        F: FnOnce(&wgpu::Device, &wgpu::Queue) -> Result<T, String>,
    {
        ensure_context()?;
        let guard = ctx_slot().lock().map_err(|_| "gpu lock poisoned".to_string())?;
        let ctx = guard.as_ref().ok_or("gpu context missing")?;
        f(&ctx.device, &ctx.queue)
    }
}

#[cfg(feature = "gpu")]
pub use imp::{upload_rgba, with_gpu};

#[cfg(not(feature = "gpu"))]
pub fn upload_rgba(_width: u32, _height: u32, _rgba: &[u8]) -> Result<u64, String> {
    Err("gpu feature disabled — rebuild with --features gpu".into())
}

#[cfg(feature = "gpu")]
pub fn probe_gpu() {
    let _ = imp::upload_rgba(1, 1, &[32, 33, 36, 255]);
}

#[cfg(not(feature = "gpu"))]
pub fn probe_gpu() {}
