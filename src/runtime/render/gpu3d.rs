//! wgpu 3D render pass — frame + material bind groups, depth buffer, readback to RGBA.
//! GP0b: group 0 = view_proj; group 1 = model/color/uv_xform (+ texture/sampler).

/// One GPU 3D frame.
pub struct Gpu3dFrame {
    pub width: u32,
    pub height: u32,
    pub clear_color: [f32; 4],
    pub view_proj: [f32; 16],
    pub model: [f32; 16],
    pub draw_color: [f32; 4],
    /// xy = scale, zw = offset; default 1,1,0,0
    pub uv_transform: [f32; 4],
    pub vertices: Vec<f32>,
    pub component_count: u32,
    pub vert_count: u32,
    pub indices: Option<Vec<u16>>,
    pub index_offset: u32,
    pub index_count: u32,
    pub depth_test: bool,
    /// Optional RGBA texture (width, height, pixels). Enables textured pipeline when set.
    pub texture: Option<(u32, u32, Vec<u8>)>,
}

pub fn gpu3d_available() -> bool {
    #[cfg(feature = "gpu")]
    {
        if super::gpu_available() {
            return true;
        }
        // Lazy probe so the first WebGL 3D draw can take the GPU path without
        // requiring an explicit compositor upload first.
        super::probe_gpu();
        return super::gpu_available();
    }
    #[cfg(not(feature = "gpu"))]
    {
        false
    }
}

pub fn info_line() -> &'static str {
    if gpu3d_available() {
        "wgpu-pipeline"
    } else {
        "cpu-fallback"
    }
}

pub fn render_frame(frame: &Gpu3dFrame) -> Result<Vec<u8>, String> {
    #[cfg(feature = "gpu")]
    {
        return imp::render_frame(frame);
    }
    #[cfg(not(feature = "gpu"))]
    {
        let _ = frame;
        Err("gpu feature disabled".into())
    }
}

#[cfg(feature = "gpu")]
mod imp {
    use super::Gpu3dFrame;
    use crate::runtime::render::gpu;
    use std::sync::{Mutex, OnceLock};

    const SHADER_SOLID: &str = r#"
struct FrameUniforms {
    view_proj: mat4x4<f32>,
}

struct MaterialUniforms {
    model: mat4x4<f32>,
    color: vec4<f32>,
    uv_xform: vec4<f32>,
}

@group(0) @binding(0) var<uniform> frame: FrameUniforms;
@group(1) @binding(0) var<uniform> mat: MaterialUniforms;

struct VertexIn {
    @location(0) position: vec3<f32>,
}

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
}

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip = frame.view_proj * mat.model * vec4<f32>(in.position, 1.0);
    return out;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return mat.color;
}
"#;

    const SHADER_TEXTURED: &str = r#"
struct FrameUniforms {
    view_proj: mat4x4<f32>,
}

struct MaterialUniforms {
    model: mat4x4<f32>,
    color: vec4<f32>,
    uv_xform: vec4<f32>,
}

@group(0) @binding(0) var<uniform> frame: FrameUniforms;
@group(1) @binding(0) var<uniform> mat: MaterialUniforms;
@group(1) @binding(1) var tex: texture_2d<f32>;
@group(1) @binding(2) var samp: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
}

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip = frame.view_proj * mat.model * vec4<f32>(in.position, 1.0);
    out.uv = in.uv * mat.uv_xform.xy + mat.uv_xform.zw;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    // Match CPU sample_texture V-flip.
    let uv = vec2<f32>(in.uv.x, 1.0 - in.uv.y);
    return textureSample(tex, samp, uv) * mat.color;
}
"#;

    struct Targets {
        width: u32,
        height: u32,
        color: wgpu::Texture,
        color_view: wgpu::TextureView,
        depth: wgpu::Texture,
        depth_view: wgpu::TextureView,
    }

    struct Pipe {
        pipeline: wgpu::RenderPipeline,
        material_layout: wgpu::BindGroupLayout,
    }

    struct State {
        frame_layout: wgpu::BindGroupLayout,
        solid: Pipe,
        textured: Pipe,
        targets: Option<Targets>,
    }

    static STATE: OnceLock<Mutex<Option<State>>> = OnceLock::new();

    fn state_slot() -> &'static Mutex<Option<State>> {
        STATE.get_or_init(|| Mutex::new(None))
    }

    fn write_mat4(dst: &mut [u8], mat: &[f32; 16]) {
        for (i, f) in mat.iter().enumerate() {
            dst[i * 4..i * 4 + 4].copy_from_slice(&f.to_le_bytes());
        }
    }

    fn write_vec4(dst: &mut [u8], v: [f32; 4]) {
        for (i, f) in v.iter().enumerate() {
            dst[i * 4..i * 4 + 4].copy_from_slice(&f.to_le_bytes());
        }
    }

    fn frame_uniform_bytes(view_proj: &[f32; 16]) -> [u8; 64] {
        let mut b = [0u8; 64];
        write_mat4(&mut b, view_proj);
        b
    }

    fn material_uniform_bytes(
        model: &[f32; 16],
        color: [f32; 4],
        uv_xform: [f32; 4],
    ) -> [u8; 96] {
        let mut b = [0u8; 96];
        write_mat4(&mut b[0..64], model);
        write_vec4(&mut b[64..80], color);
        write_vec4(&mut b[80..96], uv_xform);
        b
    }

    fn make_frame_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("kabootar-3d-frame-bind"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        })
    }

    fn make_solid_material_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("kabootar-3d-solid-mat-bind"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        })
    }

    fn make_textured_material_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("kabootar-3d-tex-mat-bind"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    }

    fn make_solid_pipe(
        device: &wgpu::Device,
        frame_layout: &wgpu::BindGroupLayout,
    ) -> Pipe {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("kabootar-3d-solid"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(SHADER_SOLID)),
        });
        let material_layout = make_solid_material_layout(device);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("kabootar-3d-solid-layout"),
            bind_group_layouts: &[frame_layout, &material_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("kabootar-3d-solid-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 12,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x3,
                        offset: 0,
                        shader_location: 0,
                    }],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        Pipe {
            pipeline,
            material_layout,
        }
    }

    fn make_textured_pipe(
        device: &wgpu::Device,
        frame_layout: &wgpu::BindGroupLayout,
    ) -> Pipe {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("kabootar-3d-tex"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(SHADER_TEXTURED)),
        });
        let material_layout = make_textured_material_layout(device);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("kabootar-3d-tex-layout"),
            bind_group_layouts: &[frame_layout, &material_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("kabootar-3d-tex-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 20,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 12,
                            shader_location: 1,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        Pipe {
            pipeline,
            material_layout,
        }
    }

    fn ensure_state(device: &wgpu::Device) -> Result<(), String> {
        let mut guard = state_slot()
            .lock()
            .map_err(|_| "gpu3d lock poisoned".to_string())?;
        if guard.is_some() {
            return Ok(());
        }
        let frame_layout = make_frame_layout(device);
        let solid = make_solid_pipe(device, &frame_layout);
        let textured = make_textured_pipe(device, &frame_layout);
        *guard = Some(State {
            frame_layout,
            solid,
            textured,
            targets: None,
        });
        Ok(())
    }

    fn ensure_targets(device: &wgpu::Device, width: u32, height: u32) -> Result<(), String> {
        let mut guard = state_slot()
            .lock()
            .map_err(|_| "gpu3d lock poisoned".to_string())?;
        let state = guard.as_mut().ok_or("gpu3d pipeline missing")?;
        let need_new = state
            .targets
            .as_ref()
            .map(|t| t.width != width || t.height != height)
            .unwrap_or(true);
        if !need_new {
            return Ok(());
        }
        let color = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("kabootar-3d-color"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let color_view = color.create_view(&wgpu::TextureViewDescriptor::default());
        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("kabootar-3d-depth"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());
        state.targets = Some(Targets {
            width,
            height,
            color,
            color_view,
            depth,
            depth_view,
        });
        Ok(())
    }

    fn pack_vertices(frame: &Gpu3dFrame, textured: bool) -> Result<Vec<u8>, String> {
        let stride = frame.component_count as usize;
        if textured {
            if stride < 5 {
                return Err("gpu3d textured: need vec5 (xyz+uv)".into());
            }
            let mut out = Vec::with_capacity(frame.vert_count as usize * 20);
            for i in 0..frame.vert_count as usize {
                let base = i * stride;
                if base + 4 >= frame.vertices.len() {
                    break;
                }
                for k in 0..5 {
                    out.extend_from_slice(&frame.vertices[base + k].to_le_bytes());
                }
            }
            if out.is_empty() {
                return Err("gpu3d: no vertices".into());
            }
            return Ok(out);
        }
        if stride < 3 {
            return Err("gpu3d: need vec3 vertices".into());
        }
        let mut out = Vec::with_capacity(frame.vert_count as usize * 12);
        for i in 0..frame.vert_count as usize {
            let base = i * stride;
            if base + 2 >= frame.vertices.len() {
                break;
            }
            out.extend_from_slice(&frame.vertices[base].to_le_bytes());
            out.extend_from_slice(&frame.vertices[base + 1].to_le_bytes());
            out.extend_from_slice(&frame.vertices[base + 2].to_le_bytes());
        }
        if out.is_empty() {
            return Err("gpu3d: no vertices".into());
        }
        Ok(out)
    }

    fn readback_rgba(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture: &wgpu::Texture,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>, String> {
        let unpadded = width * 4;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bpr = ((unpadded + align - 1) / align) * align;
        let read_size = (padded_bpr * height) as u64;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("kabootar-3d-readback"),
            size: read_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("kabootar-3d-readback-enc"),
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bpr),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(std::iter::once(encoder.finish()));

        let slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        slice.map_async(wgpu::MapMode::Read, move |r| {
            tx.send(r).ok();
        });
        device.poll(wgpu::Maintain::Wait);
        rx.recv()
            .map_err(|_| "gpu3d readback channel".to_string())?
            .map_err(|e| format!("gpu3d map: {e}"))?;
        let mapped = slice.get_mapped_range();
        let mut rgba = vec![0u8; (width * height * 4) as usize];
        for y in 0..height as usize {
            let src = y * padded_bpr as usize;
            let dst = y * unpadded as usize;
            rgba[dst..dst + unpadded as usize]
                .copy_from_slice(&mapped[src..src + unpadded as usize]);
        }
        drop(mapped);
        buffer.unmap();
        Ok(rgba)
    }

    pub fn render_frame(frame: &Gpu3dFrame) -> Result<Vec<u8>, String> {
        if !gpu::gpu_available() {
            return Err("gpu not available".into());
        }
        let textured = frame.texture.is_some();
        if textured && frame.component_count < 5 {
            return Err("gpu3d textured requires vec5 vertices".into());
        }
        if !textured && frame.component_count < 3 {
            return Err("gpu3d requires vec3 vertices".into());
        }
        let width = frame.width.max(1);
        let height = frame.height.max(1);
        let vertex_bytes = pack_vertices(frame, textured)?;
        let vert_stride = if textured { 20 } else { 12 };

        gpu::with_gpu(|device, queue| {
            ensure_state(device)?;
            ensure_targets(device, width, height)?;

            let frame_buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("kabootar-3d-frame-uniforms"),
                size: 64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(
                &frame_buf,
                0,
                &frame_uniform_bytes(&frame.view_proj),
            );

            let color = [
                frame.draw_color[0],
                frame.draw_color[1],
                frame.draw_color[2],
                frame.draw_color[3],
            ];
            let mat_buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("kabootar-3d-material-uniforms"),
                size: 96,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(
                &mat_buf,
                0,
                &material_uniform_bytes(&frame.model, color, frame.uv_transform),
            );

            let vertex_buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("kabootar-3d-vbo"),
                size: vertex_bytes.len() as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&vertex_buf, 0, &vertex_bytes);

            let index_buf = if let Some(ref indices) = frame.indices {
                let start = frame.index_offset as usize;
                let end = (start + frame.index_count as usize).min(indices.len());
                let index_data: Vec<u8> = indices[start..end]
                    .iter()
                    .flat_map(|i| i.to_le_bytes())
                    .collect();
                if index_data.is_empty() {
                    None
                } else {
                    let buf = device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("kabootar-3d-ibo"),
                        size: index_data.len() as u64,
                        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                    queue.write_buffer(&buf, 0, &index_data);
                    Some((buf, (index_data.len() / 2) as u32))
                }
            } else {
                None
            };

            // Optional sample texture (keep alive for bind group).
            let sample_tex;
            let sample_view;
            let sample_samp;
            if let Some((tw, th, ref pixels)) = frame.texture {
                let expected = (tw as usize) * (th as usize) * 4;
                if pixels.len() < expected {
                    return Err("gpu3d texture pixel buffer too small".into());
                }
                sample_tex = Some(device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("kabootar-3d-sample"),
                    size: wgpu::Extent3d {
                        width: tw.max(1),
                        height: th.max(1),
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                }));
                let tex = sample_tex.as_ref().unwrap();
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: tex,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &pixels[..expected],
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(tw.max(1) * 4),
                        rows_per_image: Some(th.max(1)),
                    },
                    wgpu::Extent3d {
                        width: tw.max(1),
                        height: th.max(1),
                        depth_or_array_layers: 1,
                    },
                );
                sample_view = Some(tex.create_view(&wgpu::TextureViewDescriptor::default()));
                sample_samp = Some(device.create_sampler(&wgpu::SamplerDescriptor {
                    label: Some("kabootar-3d-sampler"),
                    mag_filter: wgpu::FilterMode::Nearest,
                    min_filter: wgpu::FilterMode::Nearest,
                    ..Default::default()
                }));
            } else {
                sample_tex = None;
                sample_view = None;
                sample_samp = None;
            }
            let _keep_tex = sample_tex;

            let mut guard = state_slot()
                .lock()
                .map_err(|_| "gpu3d lock poisoned".to_string())?;
            let state = guard.as_mut().ok_or("gpu3d pipeline missing")?;
            let targets = state.targets.as_ref().ok_or("gpu3d targets missing")?;

            let frame_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("kabootar-3d-frame-bg"),
                layout: &state.frame_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: frame_buf.as_entire_binding(),
                }],
            });

            let material_bind = if textured {
                let view = sample_view.as_ref().ok_or("gpu3d missing sample view")?;
                let samp = sample_samp.as_ref().ok_or("gpu3d missing sampler")?;
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("kabootar-3d-tex-mat-bg"),
                    layout: &state.textured.material_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: mat_buf.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(samp),
                        },
                    ],
                })
            } else {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("kabootar-3d-solid-mat-bg"),
                    layout: &state.solid.material_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: mat_buf.as_entire_binding(),
                    }],
                })
            };

            let pipeline = if textured {
                &state.textured.pipeline
            } else {
                &state.solid.pipeline
            };

            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("kabootar-3d-pass"),
            });
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("kabootar-3d"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &targets.color_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: frame.clear_color[0] as f64,
                                g: frame.clear_color[1] as f64,
                                b: frame.clear_color[2] as f64,
                                a: frame.clear_color[3] as f64,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &targets.depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, &frame_bind, &[]);
                pass.set_bind_group(1, &material_bind, &[]);
                pass.set_vertex_buffer(0, vertex_buf.slice(..));

                if let Some((ref ibo, count)) = index_buf {
                    pass.set_index_buffer(ibo.slice(..), wgpu::IndexFormat::Uint16);
                    pass.draw_indexed(0..count, 0, 0..1);
                } else {
                    let vert_count = vertex_bytes.len() / vert_stride;
                    pass.draw(0..vert_count as u32, 0..1);
                }
            }
            queue.submit(std::iter::once(encoder.finish()));
            readback_rgba(device, queue, &targets.color, width, height)
        })
    }
}
