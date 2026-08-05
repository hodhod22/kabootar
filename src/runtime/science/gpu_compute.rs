//! WGSL compute matmul kernel (SC4b) — optional `gpu` feature; CPU fallback always.

/// Try device matmul via wgpu compute. Returns `(data, kernel_id)` or None.
pub fn try_matmul_compute(
    m: usize,
    k: usize,
    n: usize,
    a: &[f64],
    b: &[f64],
) -> Option<(Vec<f64>, &'static str)> {
    #[cfg(feature = "gpu")]
    {
        return gpu_impl::matmul(m, k, n, a, b);
    }
    #[cfg(not(feature = "gpu"))]
    {
        let _ = (m, k, n, a, b);
        None
    }
}

#[cfg(feature = "gpu")]
mod gpu_impl {
    use std::sync::{Mutex, OnceLock};
    use wgpu::util::DeviceExt;

    struct ComputeCtx {
        device: wgpu::Device,
        queue: wgpu::Queue,
        pipeline: wgpu::ComputePipeline,
        bind_layout: wgpu::BindGroupLayout,
    }

    static CTX: OnceLock<Mutex<Option<ComputeCtx>>> = OnceLock::new();

    const WGSL: &str = r#"
struct Dims { m: u32, k: u32, n: u32, _pad: u32 }
@group(0) @binding(0) var<storage, read> A: array<f32>;
@group(0) @binding(1) var<storage, read> B: array<f32>;
@group(0) @binding(2) var<storage, read_write> C: array<f32>;
@group(0) @binding(3) var<uniform> dims: Dims;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    let j = gid.y;
    if (i >= dims.m || j >= dims.n) { return; }
    var s: f32 = 0.0;
    var t: u32 = 0u;
    loop {
        if (t >= dims.k) { break; }
        s = s + A[i * dims.k + t] * B[t * dims.n + j];
        t = t + 1u;
    }
    C[i * dims.n + j] = s;
}
"#;

    fn ensure() -> Option<()> {
        let slot = CTX.get_or_init(|| Mutex::new(None));
        let mut g = slot.lock().ok()?;
        if g.is_some() {
            return Some(());
        }
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))?;
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("kab-science-compute"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                memory_hints: Default::default(),
            },
            None,
        ))
        .ok()?;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("kab-matmul-wgsl"),
            source: wgpu::ShaderSource::Wgsl(WGSL.into()),
        });
        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("kab-matmul-bgl"),
            entries: &[
                storage_entry(0, true),
                storage_entry(1, true),
                storage_entry(2, false),
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("kab-matmul-pl"),
            bind_group_layouts: &[&bind_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("kab-matmul-pipe"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        *g = Some(ComputeCtx {
            device,
            queue,
            pipeline,
            bind_layout,
        });
        Some(())
    }

    fn storage_entry(binding: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }
    }

    pub fn matmul(
        m: usize,
        k: usize,
        n: usize,
        a: &[f64],
        b: &[f64],
    ) -> Option<(Vec<f64>, &'static str)> {
        if a.len() != m * k || b.len() != k * n || m == 0 || n == 0 || k == 0 {
            return None;
        }
        if m * n > 4096 || k > 512 {
            return None;
        }
        ensure()?;
        let slot = CTX.get()?;
        let g = slot.lock().ok()?;
        let ctx = g.as_ref()?;

        let a_f: Vec<f32> = a.iter().map(|x| *x as f32).collect();
        let b_f: Vec<f32> = b.iter().map(|x| *x as f32).collect();
        let c_bytes = ((m * n * 4) as u64).max(4);

        let a_buf = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("a"),
            contents: cast_u8(&a_f),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let b_buf = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("b"),
            contents: cast_u8(&b_f),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let c_buf = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("c"),
            size: c_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let dims = [m as u32, k as u32, n as u32, 0u32];
        let dim_buf = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("dims"),
            contents: cast_u8(&dims),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let bind = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("kab-matmul-bg"),
            layout: &ctx.bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: a_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: b_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: c_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: dim_buf.as_entire_binding(),
                },
            ],
        });
        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("kab-matmul-enc"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("kab-matmul-pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&ctx.pipeline);
            pass.set_bind_group(0, &bind, &[]);
            let gx = (m as u32).div_ceil(8).max(1);
            let gy = (n as u32).div_ceil(8).max(1);
            pass.dispatch_workgroups(gx, gy, 1);
        }
        let read_buf = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("c-read"),
            size: c_bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        encoder.copy_buffer_to_buffer(&c_buf, 0, &read_buf, 0, c_bytes);
        ctx.queue.submit(Some(encoder.finish()));
        let slice = read_buf.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        ctx.device.poll(wgpu::Maintain::Wait);
        rx.recv().ok()?.ok()?;
        let data = slice.get_mapped_range();
        let out: Vec<f64> = data
            .chunks_exact(4)
            .take(m * n)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]) as f64)
            .collect();
        drop(data);
        read_buf.unmap();
        Some((out, "wgpu-compute-matmul_f32_v1"))
    }

    fn cast_u8<T>(v: &[T]) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v))
        }
    }
}
