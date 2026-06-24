//! wgpu 3D render pass — offscreen MVP + depth buffer, readback to RGBA.

/// One GPU 3D frame.
pub struct Gpu3dFrame {
    pub width: u32,
    pub height: u32,
    pub clear_color: [f32; 4],
    pub mvp: [f32; 16],
    pub draw_color: [f32; 4],
    pub vertices: Vec<f32>,
    pub component_count: u32,
    pub vert_count: u32,
    pub indices: Option<Vec<u16>>,
    pub index_offset: u32,
    pub index_count: u32,
    pub depth_test: bool,
}

pub fn gpu3d_available() -> bool {
    #[cfg(feature = "gpu")]
    {
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

    const SHADER: &str = r#"
struct Uniforms {
    mvp: mat4x4<f32>,
    color: vec4<f32>,
}

@group(0) @binding(0) var<uniform> u: Uniforms;

struct VertexIn {
    @location(0) position: vec3<f32>,
}

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
}

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip = u.mvp * vec4<f32>(in.position, 1.0);
    return out;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return u.color;
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

    struct Pipeline {
        pipeline: wgpu::RenderPipeline,
        bind_layout: wgpu::BindGroupLayout,
        targets: Option<Targets>,
    }

    static PIPELINE: OnceLock<Mutex<Option<Pipeline>>> = OnceLock::new();

    fn pipeline_slot() -> &'static Mutex<Option<Pipeline>> {
        PIPELINE.get_or_init(|| Mutex::new(None))
    }

    fn uniform_bytes(mvp: &[f32; 16], color: [f32; 4]) -> [u8; 80] {
        let mut b = [0u8; 80];
        for (i, f) in mvp.iter().enumerate() {
            b[i * 4..i * 4 + 4].copy_from_slice(&f.to_le_bytes());
        }
        for (i, f) in color.iter().enumerate() {
            b[64 + i * 4..64 + i * 4 + 4].copy_from_slice(&f.to_le_bytes());
        }
        b
    }

    fn ensure_pipeline(device: &wgpu::Device) -> Result<(), String> {
        let mut guard = pipeline_slot()
            .lock()
            .map_err(|_| "gpu3d lock poisoned".to_string())?;
        if guard.is_some() {
            return Ok(());
        }
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("kabootar-3d"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(SHADER)),
        });
        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("kabootar-3d-uniforms"),
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
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("kabootar-3d-layout"),
            bind_group_layouts: &[&bind_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("kabootar-3d-pipeline"),
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
        *guard = Some(Pipeline {
            pipeline,
            bind_layout,
            targets: None,
        });
        Ok(())
    }

    fn ensure_targets(device: &wgpu::Device, width: u32, height: u32) -> Result<(), String> {
        let mut guard = pipeline_slot()
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

    fn pack_vertices(frame: &Gpu3dFrame) -> Result<Vec<u8>, String> {
        let stride = frame.component_count as usize;
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
        if frame.component_count < 3 {
            return Err("gpu3d requires vec3 vertices".into());
        }
        let width = frame.width.max(1);
        let height = frame.height.max(1);
        let vertex_bytes = pack_vertices(frame)?;

        gpu::with_gpu(|device, queue| {
            ensure_pipeline(device)?;
            ensure_targets(device, width, height)?;

            let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("kabootar-3d-uniforms"),
                size: 80,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let color = [
                frame.draw_color[0],
                frame.draw_color[1],
                frame.draw_color[2],
                frame.draw_color[3],
            ];
            queue.write_buffer(
                &uniform_buf,
                0,
                &uniform_bytes(&frame.mvp, color),
            );

            let vertex_buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("kabootar-3d-vbo"),
                size: vertex_bytes.len() as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&vertex_buf, 0, &vertex_bytes);

            let mut guard = pipeline_slot()
                .lock()
                .map_err(|_| "gpu3d lock poisoned".to_string())?;
            let state = guard.as_mut().ok_or("gpu3d pipeline missing")?;
            let targets = state.targets.as_ref().ok_or("gpu3d targets missing")?;
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("kabootar-3d-bind"),
                layout: &state.bind_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buf.as_entire_binding(),
                }],
            });

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
                pass.set_pipeline(&state.pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.set_vertex_buffer(0, vertex_buf.slice(..));

                if let Some(ref indices) = frame.indices {
                    let start = frame.index_offset as usize;
                    let end = (start + frame.index_count as usize).min(indices.len());
                    let index_data: Vec<u8> = indices[start..end]
                        .iter()
                        .flat_map(|i| i.to_le_bytes())
                        .collect();
                    if !index_data.is_empty() {
                        let index_buf = device.create_buffer(&wgpu::BufferDescriptor {
                            label: Some("kabootar-3d-ibo"),
                            size: index_data.len() as u64,
                            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                            mapped_at_creation: false,
                        });
                        queue.write_buffer(&index_buf, 0, &index_data);
                        pass.set_index_buffer(index_buf.slice(..), wgpu::IndexFormat::Uint16);
                        pass.draw_indexed(0..(index_data.len() / 2) as u32, 0, 0..1);
                    }
                } else {
                    let vert_count = vertex_bytes.len() / 12;
                    pass.draw(0..vert_count as u32, 0..1);
                }
            }
            queue.submit(std::iter::once(encoder.finish()));
            readback_rgba(device, queue, &targets.color, width, height)
        })
    }
}
