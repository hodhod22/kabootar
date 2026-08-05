//! WGSL compute kernels (SC4b) — matmul + conv2d; optional `gpu` feature; CPU fallback always.

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

/// Try device conv2d (stride=1, pad=0). X [C,H,W], W [O,C,Kh,Kw], B [O].
pub fn try_conv2d_compute(
    cin: usize,
    hin: usize,
    win: usize,
    cout: usize,
    kh: usize,
    kw: usize,
    x: &[f64],
    w: &[f64],
    bias: &[f64],
) -> Option<(Vec<f64>, Vec<usize>, &'static str)> {
    #[cfg(feature = "gpu")]
    {
        return gpu_impl::conv2d(cin, hin, win, cout, kh, kw, x, w, bias);
    }
    #[cfg(not(feature = "gpu"))]
    {
        let _ = (cin, hin, win, cout, kh, kw, x, w, bias);
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
        matmul_pipeline: wgpu::ComputePipeline,
        matmul_bind: wgpu::BindGroupLayout,
        conv_pipeline: wgpu::ComputePipeline,
        conv_bind: wgpu::BindGroupLayout,
    }

    static CTX: OnceLock<Mutex<Option<ComputeCtx>>> = OnceLock::new();

    const WGSL_MATMUL: &str = r#"
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

    const WGSL_CONV2D: &str = r#"
struct Dims {
  cin: u32, hin: u32, win: u32,
  cout: u32, kh: u32, kw: u32,
  hout: u32, wout: u32,
}
@group(0) @binding(0) var<storage, read> X: array<f32>;
@group(0) @binding(1) var<storage, read> W: array<f32>;
@group(0) @binding(2) var<storage, read> Bias: array<f32>;
@group(0) @binding(3) var<storage, read_write> Y: array<f32>;
@group(0) @binding(4) var<uniform> dims: Dims;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let oc = gid.z;
    let oh = gid.y;
    let ow = gid.x;
    if (oc >= dims.cout || oh >= dims.hout || ow >= dims.wout) { return; }
    var s: f32 = Bias[oc];
    var ic: u32 = 0u;
    loop {
        if (ic >= dims.cin) { break; }
        var khi: u32 = 0u;
        loop {
            if (khi >= dims.kh) { break; }
            var kwi: u32 = 0u;
            loop {
                if (kwi >= dims.kw) { break; }
                let xi = ic * dims.hin * dims.win + (oh + khi) * dims.win + (ow + kwi);
                let wi = oc * (dims.cin * dims.kh * dims.kw)
                    + ic * (dims.kh * dims.kw)
                    + khi * dims.kw
                    + kwi;
                s = s + X[xi] * W[wi];
                kwi = kwi + 1u;
            }
            khi = khi + 1u;
        }
        ic = ic + 1u;
    }
    Y[oc * dims.hout * dims.wout + oh * dims.wout + ow] = s;
}
"#;

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

    fn uniform_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }
    }

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

        let matmul_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("kab-matmul-wgsl"),
            source: wgpu::ShaderSource::Wgsl(WGSL_MATMUL.into()),
        });
        let matmul_bind = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("kab-matmul-bgl"),
            entries: &[
                storage_entry(0, true),
                storage_entry(1, true),
                storage_entry(2, false),
                uniform_entry(3),
            ],
        });
        let matmul_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("kab-matmul-pl"),
            bind_group_layouts: &[&matmul_bind],
            push_constant_ranges: &[],
        });
        let matmul_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("kab-matmul-pipe"),
            layout: Some(&matmul_pl),
            module: &matmul_module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let conv_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("kab-conv2d-wgsl"),
            source: wgpu::ShaderSource::Wgsl(WGSL_CONV2D.into()),
        });
        let conv_bind = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("kab-conv2d-bgl"),
            entries: &[
                storage_entry(0, true),
                storage_entry(1, true),
                storage_entry(2, true),
                storage_entry(3, false),
                uniform_entry(4),
            ],
        });
        let conv_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("kab-conv2d-pl"),
            bind_group_layouts: &[&conv_bind],
            push_constant_ranges: &[],
        });
        let conv_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("kab-conv2d-pipe"),
            layout: Some(&conv_pl),
            module: &conv_module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        *g = Some(ComputeCtx {
            device,
            queue,
            matmul_pipeline,
            matmul_bind,
            conv_pipeline,
            conv_bind,
        });
        Some(())
    }

    fn cast_u8<T>(v: &[T]) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v))
        }
    }

    fn readback(
        ctx: &ComputeCtx,
        src: &wgpu::Buffer,
        n_f32: usize,
    ) -> Option<Vec<f64>> {
        let bytes = ((n_f32 * 4) as u64).max(4);
        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("kab-readback-enc"),
            });
        let read_buf = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("read"),
            size: bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        encoder.copy_buffer_to_buffer(src, 0, &read_buf, 0, bytes);
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
            .take(n_f32)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]) as f64)
            .collect();
        drop(data);
        read_buf.unmap();
        Some(out)
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
            layout: &ctx.matmul_bind,
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
            pass.set_pipeline(&ctx.matmul_pipeline);
            pass.set_bind_group(0, &bind, &[]);
            let gx = (m as u32).div_ceil(8).max(1);
            let gy = (n as u32).div_ceil(8).max(1);
            pass.dispatch_workgroups(gx, gy, 1);
        }
        ctx.queue.submit(Some(encoder.finish()));
        let out = readback(ctx, &c_buf, m * n)?;
        Some((out, "wgpu-compute-matmul_f32_v1"))
    }

    pub fn conv2d(
        cin: usize,
        hin: usize,
        win: usize,
        cout: usize,
        kh: usize,
        kw: usize,
        x: &[f64],
        w: &[f64],
        bias: &[f64],
    ) -> Option<(Vec<f64>, Vec<usize>, &'static str)> {
        if cin == 0 || hin == 0 || win == 0 || cout == 0 || kh == 0 || kw == 0 {
            return None;
        }
        if hin < kh || win < kw {
            return None;
        }
        let hout = hin + 1 - kh;
        let wout = win + 1 - kw;
        if x.len() != cin * hin * win
            || w.len() != cout * cin * kh * kw
            || bias.len() != cout
        {
            return None;
        }
        if cout * hout * wout > 4096 || cin * kh * kw > 512 {
            return None;
        }
        ensure()?;
        let slot = CTX.get()?;
        let g = slot.lock().ok()?;
        let ctx = g.as_ref()?;

        let x_f: Vec<f32> = x.iter().map(|v| *v as f32).collect();
        let w_f: Vec<f32> = w.iter().map(|v| *v as f32).collect();
        let b_f: Vec<f32> = bias.iter().map(|v| *v as f32).collect();
        let y_n = cout * hout * wout;
        let y_bytes = ((y_n * 4) as u64).max(4);

        let x_buf = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("x"),
            contents: cast_u8(&x_f),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let w_buf = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("w"),
            contents: cast_u8(&w_f),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let b_buf = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bias"),
            contents: cast_u8(&b_f),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let y_buf = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("y"),
            size: y_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let dims = [
            cin as u32,
            hin as u32,
            win as u32,
            cout as u32,
            kh as u32,
            kw as u32,
            hout as u32,
            wout as u32,
        ];
        let dim_buf = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("conv-dims"),
            contents: cast_u8(&dims),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let bind = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("kab-conv2d-bg"),
            layout: &ctx.conv_bind,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: x_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: w_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: b_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: y_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: dim_buf.as_entire_binding(),
                },
            ],
        });
        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("kab-conv2d-enc"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("kab-conv2d-pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&ctx.conv_pipeline);
            pass.set_bind_group(0, &bind, &[]);
            let gx = (wout as u32).div_ceil(8).max(1);
            let gy = (hout as u32).div_ceil(8).max(1);
            let gz = (cout as u32).max(1);
            pass.dispatch_workgroups(gx, gy, gz);
        }
        ctx.queue.submit(Some(encoder.finish()));
        let out = readback(ctx, &y_buf, y_n)?;
        Some((out, vec![cout, hout, wout], "wgpu-compute-conv2d_f32_v1"))
    }
}
