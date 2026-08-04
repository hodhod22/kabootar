//! wgpu desktop presenter — GPU-accelerated frame presentation for Kabootar shell.

#[cfg(all(feature = "shell", feature = "gpu"))]
pub fn run() -> Result<(), String> {
    use winit::event_loop::EventLoop;
    use winit::window::WindowBuilder;

    let event_loop = EventLoop::new().map_err(|e| format!("event loop: {e}"))?;
    let window = WindowBuilder::new()
        .with_title("Kabootar OS (GPU)")
        .with_inner_size(winit::dpi::LogicalSize::new(960, 540))
        .build(&event_loop)
        .map_err(|e| format!("window: {e}"))?;
    run_with_gpu(event_loop, window)
}

#[cfg(all(feature = "shell", feature = "gpu"))]
pub fn run_with_gpu(
    event_loop: winit::event_loop::EventLoop<()>,
    window: winit::window::Window,
) -> Result<(), String> {
    use crate::runtime::frame_buffer;
    use std::sync::Arc;
    use winit::event::{Event, WindowEvent};
    use winit::window::Window;

    struct GpuPresenter {
        surface: wgpu::Surface<'static>,
        device: wgpu::Device,
        queue: wgpu::Queue,
        config: wgpu::SurfaceConfiguration,
        pipeline: wgpu::RenderPipeline,
        bind_group_layout: wgpu::BindGroupLayout,
        sampler: wgpu::Sampler,
        texture: wgpu::Texture,
        texture_view: wgpu::TextureView,
        bind_group: wgpu::BindGroup,
        frame_w: u32,
        frame_h: u32,
    }

    impl GpuPresenter {
        fn new(window: Arc<Window>) -> Result<Self, String> {
            let size = window.inner_size();
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
                backends: wgpu::Backends::all(),
                ..Default::default()
            });
            let surface = instance
                .create_surface(window.clone())
                .map_err(|e| format!("wgpu surface: {e}"))?;
            let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            }))
            .ok_or("no wgpu adapter for surface")?;
            let (device, queue) = pollster::block_on(adapter.request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("kabootar-shell"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            ))
            .map_err(|e| format!("wgpu device: {e}"))?;
            let caps = surface.get_capabilities(&adapter);
            let format = caps
                .formats
                .iter()
                .copied()
                .find(|f| f.is_srgb())
                .unwrap_or(caps.formats[0]);
            let want_immediate =
                crate::runtime::os::display_vsync_mode().eq_ignore_ascii_case("immediate");
            let present_mode = if want_immediate
                && caps
                    .present_modes
                    .contains(&wgpu::PresentMode::Immediate)
            {
                wgpu::PresentMode::Immediate
            } else {
                wgpu::PresentMode::Fifo
            };
            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                width: size.width.max(1),
                height: size.height.max(1),
                present_mode,
                alpha_mode: caps.alpha_modes[0],
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            };
            surface.configure(&device, &config);

            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("kabootar-blit"),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(SHADER)),
            });
            let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("kabootar-blit-layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("kabootar-blit-pipeline-layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });
            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("kabootar-blit-pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });
            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("kabootar-sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });
            let (texture, texture_view) = Self::make_texture(&device, 1, 1)?;
            let bind_group = Self::make_bind_group(
                &device,
                &bind_group_layout,
                &texture_view,
                &sampler,
            );
            Ok(Self {
                surface,
                device,
                queue,
                config,
                pipeline,
                bind_group_layout,
                sampler,
                texture,
                texture_view,
                bind_group,
                frame_w: 1,
                frame_h: 1,
            })
        }

        fn make_texture(
            device: &wgpu::Device,
            width: u32,
            height: u32,
        ) -> Result<(wgpu::Texture, wgpu::TextureView), String> {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("kabootar-shell-frame"),
                size: wgpu::Extent3d {
                    width: width.max(1),
                    height: height.max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let view = texture.create_view(&Default::default());
            Ok((texture, view))
        }

        fn make_bind_group(
            device: &wgpu::Device,
            layout: &wgpu::BindGroupLayout,
            view: &wgpu::TextureView,
            sampler: &wgpu::Sampler,
        ) -> wgpu::BindGroup {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("kabootar-blit-bind"),
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                ],
            })
        }

        fn resize(&mut self, width: u32, height: u32) {
            if width > 0 && height > 0 {
                self.config.width = width;
                self.config.height = height;
                self.surface.configure(&self.device, &self.config);
            }
        }

        fn upload_rgba(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<(), String> {
            let expected = (width * height * 4) as usize;
            if rgba.len() < expected {
                return Err("frame rgba too small".into());
            }
            if width != self.frame_w || height != self.frame_h {
                let (tex, view) = Self::make_texture(&self.device, width, height)?;
                self.texture = tex;
                self.texture_view = view;
                self.bind_group = Self::make_bind_group(
                    &self.device,
                    &self.bind_group_layout,
                    &self.texture_view,
                    &self.sampler,
                );
                self.frame_w = width;
                self.frame_h = height;
            }
            let unpadded = width * 4;
            let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
            let bpr = ((unpadded + align - 1) / align) * align;
            let data = if bpr == unpadded {
                rgba[..expected].to_vec()
            } else {
                let mut out = vec![0u8; (bpr * height) as usize];
                for y in 0..height as usize {
                    let src = y * unpadded as usize;
                    let dst = y * bpr as usize;
                    out[dst..dst + unpadded as usize]
                        .copy_from_slice(&rgba[src..src + unpadded as usize]);
                }
                out
            };
            self.queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &self.texture,
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
            Ok(())
        }

        fn present(&mut self) -> Result<(), String> {
            let output = self
                .surface
                .get_current_texture()
                .map_err(|e| format!("surface frame: {e}"))?;
            let view = output
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder =
                self.device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("kabootar-present"),
                    });
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("kabootar-blit-pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.125,
                                g: 0.129,
                                b: 0.141,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.bind_group, &[]);
                pass.draw(0..3, 0..1);
            }
            self.queue.submit(std::iter::once(encoder.finish()));
            output.present();
            Ok(())
        }

        fn blit_from_buffer(&mut self) -> Result<(), String> {
            let (w, h, rgba) = frame_buffer::last_frame_pixels().ok_or("no compositor frame")?;
            self.upload_rgba(w.max(1) as u32, h.max(1) as u32, &rgba)?;
            self.present()
        }
    }

    const SHADER: &str = r#"
struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(2.0, 1.0),
        vec2<f32>(0.0, -1.0),
    );
    var out: VertexOutput;
    out.pos = vec4<f32>(positions[i], 0.0, 1.0);
    out.uv = uvs[i];
    return out;
}

@group(0) @binding(0) var frame_tex: texture_2d<f32>;
@group(0) @binding(1) var frame_samp: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(frame_tex, frame_samp, in.uv);
}
"#;

    let window = Arc::new(window);
    let mut presenter = GpuPresenter::new(window.clone())?;
    presenter.blit_from_buffer()?;
    let mut cursor = (0.0_f64, 0.0_f64);

    event_loop
        .run(move |event, elwt| {
            match event {
                Event::WindowEvent { event, window_id } => {
                    if window_id != window.id() {
                        return;
                    }
                    match event {
                        WindowEvent::CloseRequested => elwt.exit(),
                        WindowEvent::Resized(size) => {
                            presenter.resize(size.width, size.height);
                        }
                        WindowEvent::RedrawRequested => {
                            let _ = presenter.blit_from_buffer();
                        }
                        WindowEvent::CursorMoved { position, .. } => {
                            cursor = (position.x, position.y);
                            crate::runtime::game::pointer_move(position.x, position.y);
                        }
                        WindowEvent::MouseInput {
                            state: winit::event::ElementState::Pressed,
                            button: winit::event::MouseButton::Left,
                            ..
                        } => {
                            let (x, y) = cursor;
                            let _ = crate::shell::shell_pointer_click(x, y);
                            window.request_redraw();
                        }
                        WindowEvent::MouseInput {
                            state: winit::event::ElementState::Released,
                            button: winit::event::MouseButton::Left,
                            ..
                        } => {
                            let (x, y) = cursor;
                            crate::runtime::game::pointer_up(x, y);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        })
        .map_err(|e| format!("loop: {e}"))?;
    Ok(())
}
