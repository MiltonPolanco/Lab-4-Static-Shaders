use winit::{
    event::*,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};
use wgpu::util::DeviceExt;
use cgmath::prelude::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
    time: f32,
    planet_type: u32,
    render_moon: u32,
    _padding: f32,
}

impl Uniforms {
    fn new() -> Self {
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
            model: cgmath::Matrix4::identity().into(),
            time: 0.0,
            planet_type: 0,
            render_moon: 0,
            _padding: 0.0,
        }
    }
}

struct State<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    uniform_buffer: wgpu::Buffer,
    moon_uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    moon_bind_group: wgpu::BindGroup,
    uniforms: Uniforms,
    camera_angle: f32,
    planet_type: u32,
    time: f32,
    show_moon: bool,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    window: &'a Window,
}

impl<'a> State<'a> {
    async fn new(window: &'a Window) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // esfera base
        let (vertices, indices) = create_sphere(50, 50);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let num_indices = indices.len() as u32;

        let mut uniforms = Uniforms::new();
        let aspect = size.width as f32 / size.height as f32;
        
        // ajuste la camara para que se vea mejor
        let projection = cgmath::perspective(cgmath::Deg(50.0), aspect, 0.1, 100.0);
        let view = cgmath::Matrix4::look_at_rh(
            cgmath::Point3::new(0.0, 0.0, 4.0),
            cgmath::Point3::new(0.0, 0.0, 0.0),
            cgmath::Vector3::unit_y(),
        );
        uniforms.view_proj = (projection * view).into();

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Planet Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let mut moon_uniforms = uniforms.clone();
        moon_uniforms.render_moon = 1;

        let moon_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Moon Uniform Buffer"),
            contents: bytemuck::cast_slice(&[moon_uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("uniform_bind_group_layout"),
            });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("planet_uniform_bind_group"),
        });

        let moon_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: moon_uniform_buffer.as_entire_binding(),
            }],
            label: Some("moon_uniform_bind_group"),
        });

        // depth buffer para que la luna se oculte bien
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&uniform_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
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
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices,
            uniform_buffer,
            moon_uniform_buffer,
            uniform_bind_group,
            moon_bind_group,
            uniforms,
            camera_angle: 0.0,
            planet_type: 0,
            time: 0.0,
            show_moon: false,
            depth_texture,
            depth_view,
            window,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            self.depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Depth Texture"),
                size: wgpu::Extent3d {
                    width: new_size.width,
                    height: new_size.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });

            self.depth_view = self.depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
            
            let aspect = new_size.width as f32 / new_size.height as f32;
            let projection = cgmath::perspective(cgmath::Deg(50.0), aspect, 0.1, 100.0);
            let view = cgmath::Matrix4::look_at_rh(
                cgmath::Point3::new(0.0, 0.0, 4.0),
                cgmath::Point3::new(0.0, 0.0, 0.0),
                cgmath::Vector3::unit_y(),
            );
            self.uniforms.view_proj = (projection * view).into();
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => match key {
                KeyCode::Digit1 => {
                    self.planet_type = 0;
                    println!("Planeta Rocoso");
                    true
                }
                KeyCode::Digit2 => {
                    self.planet_type = 1;
                    println!("Gigante Gaseoso");
                    true
                }
                KeyCode::Digit3 => {
                    self.planet_type = 2;
                    println!("Planeta de Lava");
                    true
                }
                KeyCode::Digit4 => {
                    self.planet_type = 3;
                    println!("Planeta de Cristal");
                    true
                }
                KeyCode::Digit5 => {
                    self.planet_type = 4;
                    println!("Planeta con Océanos");
                    true
                }
                KeyCode::KeyM => {
                    self.show_moon = !self.show_moon;
                    println!("Luna: {}", if self.show_moon { "Activada" } else { "Desactivada" });
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn update(&mut self, dt: f32) {
        self.time += dt;
        self.camera_angle += dt * 0.3;

        let rotation = cgmath::Matrix4::from_angle_y(cgmath::Rad(self.time * 0.5));
        self.uniforms.model = rotation.into();
        self.uniforms.time = self.time;
        self.uniforms.planet_type = self.planet_type;
        self.uniforms.render_moon = 0;

        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[self.uniforms]));

        let mut moon_uniforms = self.uniforms;
        moon_uniforms.render_moon = 1;
        self.queue
            .write_buffer(&self.moon_uniform_buffer, 0, bytemuck::cast_slice(&[moon_uniforms]));
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.01,
                            g: 0.01,
                            b: 0.02,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            
            // dibujar planeta
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
            
            // dibujar luna si esta activa
            if self.show_moon {
                render_pass.set_bind_group(0, &self.moon_bind_group, &[]);
                render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

// funcion para generar la esfera
fn create_sphere(lat_bands: u32, long_bands: u32) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for lat in 0..=lat_bands {
        let theta = lat as f32 * std::f32::consts::PI / lat_bands as f32;
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        for long in 0..=long_bands {
            let phi = long as f32 * 2.0 * std::f32::consts::PI / long_bands as f32;
            let sin_phi = phi.sin();
            let cos_phi = phi.cos();

            let x = cos_phi * sin_theta;
            let y = cos_theta;
            let z = sin_phi * sin_theta;

            vertices.push(Vertex {
                position: [x, y, z],
                normal: [x, y, z],
            });
        }
    }

    // indices para los triangulos
    for lat in 0..lat_bands {
        for long in 0..long_bands {
            let first = lat * (long_bands + 1) + long;
            let second = first + long_bands + 1;

            indices.push(first);
            indices.push(second);
            indices.push(first + 1);

            indices.push(second);
            indices.push(second + 1);
            indices.push(first + 1);
        }
    }

    (vertices, indices)
}

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("Lab 4 - Planetas Procedurales")
        .build(&event_loop)
        .unwrap();

    let mut state = pollster::block_on(State::new(&window));
    let mut last_render_time = std::time::Instant::now();

    event_loop.run(move |event, control_flow| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window.id() => {
                if !state.input(event) {
                    match event {
                        WindowEvent::CloseRequested => control_flow.exit(),
                        WindowEvent::Resized(physical_size) => {
                            state.resize(*physical_size);
                        }
                        WindowEvent::RedrawRequested => {
                            let now = std::time::Instant::now();
                            let dt = now - last_render_time;
                            last_render_time = now;
                            state.update(dt.as_secs_f32());

                            match state.render() {
                                Ok(_) => {}
                                Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                                Err(wgpu::SurfaceError::OutOfMemory) => control_flow.exit(),
                                Err(e) => eprintln!("{:?}", e),
                            }
                        }
                        _ => {}
                    }
                }
            }
            Event::AboutToWait => {
                state.window.request_redraw();
            }
            _ => {}
        }
    }).unwrap();
}
