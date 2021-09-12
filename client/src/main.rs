use crate::camera::Camera;
use crate::character::CharacterSet;
use crate::character::DrawCharacterSet;
use crate::fpscounter::FpsCounter;
use crate::instance::{Instance, InstanceRaw};
use crate::model::Vertex;
use ::network::{Connection, Packet};
use cgmath::{InnerSpace, Rotation3, Zero};
use log::*;
use std::time::{Duration, Instant};
use wgpu::util::DeviceExt;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

mod camera;
mod character;
mod fpscounter;
mod instance;
mod model;
mod texture;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct LightUniform {
    position: [f32; 3],
    _padding: u32,
    color: [f32; 3],
}

struct Player {
    position: cgmath::Point3<f32>,
    moving_forward: bool,
    moving_backward: bool,
    moving_left: bool,
    moving_right: bool,
}

impl Player {
    fn new() -> Self {
        Self {
            position: cgmath::Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            moving_forward: false,
            moving_backward: false,
            moving_left: false,
            moving_right: false,
        }
    }

    fn update(&mut self, camera: &Camera) {
        let at = camera.at();
        let at = cgmath::Vector3 {
            x: at.x,
            y: 0.0,
            z: at.z,
        };
        let side_vector = at.cross(cgmath::Vector3::unit_y());

        let forward = if self.moving_forward { 1.0 } else { 0.0 };
        let backward = if self.moving_backward { 1.0 } else { 0.0 };
        let left = if self.moving_left { 1.0 } else { 0.0 };
        let right = if self.moving_right { 1.0 } else { 0.0 };

        let movement = at * forward - at * backward - side_vector * left + side_vector * right;
        let speed = 0.002;
        self.position += speed * movement;
    }
}

struct State {
    surface: wgpu::Surface,
    surface_config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    bg_color: wgpu::Color,
    render_pipeline2: wgpu::RenderPipeline,
    camera: Camera,
    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,
    depth_texture: texture::Texture,
    depth_bind_group: wgpu::BindGroup,
    obj_model: model::Model,

    light_uniform: LightUniform,
    light_buffer: wgpu::Buffer,
    light_bind_group: wgpu::BindGroup,

    network: Connection,
    character_set: CharacterSet,
    player: Player,
    last_pos_update: Instant,
    cursor_locked: bool,
    fps: FpsCounter,
}

impl State {
    // Creating some of the wgpu types requires async code
    async fn new(window: &Window, network: Connection) -> Self {
        let size = window.inner_size();

        // GPU hande
        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let backend = wgpu::util::backend_bits_from_env().unwrap_or(wgpu::Backends::PRIMARY);
        let adapter = wgpu::util::initialize_adapter_from_env_or_default(&instance, backend)
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_preferred_format(&adapter).unwrap(),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Mailbox,
        };
        surface.configure(&device, &surface_config);

        let depth_texture = texture::Texture::create_depth_texture(&device, &size, "depth_texture");

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                        ty: wgpu::BindingType::Sampler {
                            comparison: false,
                            filtering: true,
                        },
                        count: None,
                    },
                ],
                label: Some("Texture bind group layout"),
            });
        let depth_texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Depth,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler {
                            comparison: true,
                            filtering: true,
                        },
                        count: None,
                    },
                ],
                label: Some("Texture bind group layout"),
            });
        let depth_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &depth_texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&depth_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&depth_texture.sampler),
                },
            ],
            label: Some("Depth bind group descriptor"),
        });

        let light_uniform = LightUniform {
            position: [2.0, 2.0, 2.0],
            _padding: 0,
            color: [1.0, 1.0, 1.0],
        };
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light VB"),
            contents: bytemuck::cast_slice(&[light_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let light_bind_group_layout =
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
                label: None,
            });

        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &light_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_buffer.as_entire_binding(),
            }],
            label: None,
        });

        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let instances = (0..10)
            .flat_map(|z| {
                (0..10).map(move |x| {
                    let position = cgmath::Vector3 {
                        x: 3.0 * x as f32,
                        y: 0.0,
                        z: 3.0 * z as f32,
                    } - cgmath::Vector3 {
                        x: 15.0,
                        y: 0.0,
                        z: 15.0,
                    };
                    let rotation = if position.is_zero() {
                        // Special case for the object at (0,0,0)
                        cgmath::Quaternion::from_axis_angle(
                            cgmath::Vector3::unit_z(),
                            cgmath::Deg(0.0),
                        )
                    } else {
                        cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(45.0))
                    };
                    Instance { position, rotation }
                })
            })
            .collect::<Vec<_>>();
        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let camera = Camera::new(&device, &size).unwrap();

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &texture_bind_group_layout,
                    &camera.bind_group_layout,
                    &depth_texture_bind_group_layout,
                    &light_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let render_pipeline2 = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "main",
                buffers: &[model::ModelVertex::desc(), InstanceRaw::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "colored",
                targets: &[wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                clamp_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
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
        });

        let res_dir = std::path::Path::new(".").join("res");
        let obj_model = model::Model::load(
            &device,
            &queue,
            &texture_bind_group_layout,
            res_dir.join("cube.obj"),
        )
        .unwrap();

        let character_set = CharacterSet::new(&device, &queue, &texture_bind_group_layout);
        let player = Player::new();

        Self {
            surface,
            device,
            queue,
            size,
            bg_color: wgpu::Color {
                r: 0.1,
                g: 0.2,
                b: 0.3,
                a: 1.0,
            },
            render_pipeline2,
            camera,
            instances,
            instance_buffer,
            depth_texture,
            depth_bind_group,
            obj_model,
            light_uniform,
            light_buffer,
            light_bind_group,
            network,
            character_set,
            surface_config,
            player,
            last_pos_update: Instant::now(),
            cursor_locked: false,
            fps: FpsCounter::new(),
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_config);

            self.depth_texture =
                texture::Texture::create_depth_texture(&self.device, &self.size, "depth texture");
        }
    }

    fn input(&mut self, event: &WindowEvent, window: &winit::window::Window) -> bool {
        let processed = match event {
            WindowEvent::KeyboardInput { input, .. } => {
                if input.virtual_keycode == Some(winit::event::VirtualKeyCode::Space)
                    && input.state == winit::event::ElementState::Released
                {
                    true
                } else if input.virtual_keycode == Some(winit::event::VirtualKeyCode::Q)
                    && input.state == winit::event::ElementState::Released
                {
                    true
                } else if input.virtual_keycode == Some(winit::event::VirtualKeyCode::W) {
                    self.player.moving_forward = input.state == winit::event::ElementState::Pressed;
                    true
                } else if input.virtual_keycode == Some(winit::event::VirtualKeyCode::A) {
                    self.player.moving_left = input.state == winit::event::ElementState::Pressed;
                    true
                } else if input.virtual_keycode == Some(winit::event::VirtualKeyCode::S) {
                    self.player.moving_backward =
                        input.state == winit::event::ElementState::Pressed;
                    true
                } else if input.virtual_keycode == Some(winit::event::VirtualKeyCode::D) {
                    self.player.moving_right = input.state == winit::event::ElementState::Pressed;
                    true
                } else if input.virtual_keycode == Some(winit::event::VirtualKeyCode::Tab)
                    && self.cursor_locked
                {
                    self.cursor_locked = false;
                    window.set_cursor_grab(false).unwrap();
                    true
                } else {
                    false
                }
            }
            WindowEvent::CursorMoved { position: pos, .. } if self.cursor_locked => {
                window
                    .set_cursor_position(winit::dpi::PhysicalPosition::new(100, 100))
                    .unwrap();
                let dx = pos.x - 100.0;
                let dy = pos.y - 100.0;
                self.camera.input(dx, dy);
                self.bg_color = wgpu::Color {
                    r: pos.x / self.size.width as f64,
                    g: pos.y / self.size.height as f64,
                    b: 0.3,
                    a: 1.0,
                };
                true
            }
            WindowEvent::MouseInput { button, state, .. } => {
                if !self.cursor_locked {
                    self.cursor_locked = true;
                    window.set_cursor_grab(true).unwrap();
                    window
                        .set_cursor_position(winit::dpi::PhysicalPosition::new(100, 100))
                        .unwrap();
                    true
                } else {
                    false
                }
            }
            _ => false,
        };
        if !processed {
            //self.camera_controller.process_events(event)
            false
        } else {
            false
        }
    }

    fn update(&mut self) {
        let packets = self.network.packets().unwrap();
        for packet in packets.iter() {
            match packet {
                Packet::CreateCharacter {
                    id,
                    username,
                    position,
                    is_owned,
                } => {
                    if *is_owned {
                        // This is the player character
                        self.player.position = cgmath::Point3 {
                            x: position[0],
                            y: position[1],
                            z: position[2],
                        };
                    } else {
                        self.character_set.add(*id, position.clone());
                    }
                }
                Packet::UpdatePosition { id, position } => {
                    self.character_set.update_position(*id, position.clone());
                }
                Packet::Login { .. } => {
                    panic!("Impossible packet");
                }
            }
        }

        if self.last_pos_update.elapsed() > Duration::from_millis(50) {
            self.last_pos_update = Instant::now();
            self.network
                .send(&Packet::UpdatePosition {
                    id: 0,
                    position: self.player.position.into(),
                })
                .unwrap();
        }

        // Update the camera
        self.player.update(&self.camera);
        self.camera.set_position(&self.player.position);
        self.camera.update(&self.queue);

        // Move the light
        let old_position: cgmath::Vector3<_> = self.light_uniform.position.into();
        self.light_uniform.position =
            (cgmath::Quaternion::from_axis_angle((0.0, 1.0, 0.0).into(), cgmath::Deg(1.0))
                * old_position)
                .into();
        self.queue.write_buffer(
            &self.light_buffer,
            0,
            bytemuck::cast_slice(&[self.light_uniform]),
        );
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.fps.frame();

        let frame = self.surface.get_current_frame()?.output;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.bg_color),
                        store: true,
                    },
                }],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            render_pass.set_pipeline(&self.render_pipeline2);
            render_pass.set_bind_group(1, &self.camera.bind_group, &[]);
            render_pass.set_bind_group(2, &self.depth_bind_group, &[]);
            render_pass.set_bind_group(3, &self.light_bind_group, &[]);

            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

            use model::DrawModel;
            render_pass.draw_model_instanced(&self.obj_model, 0..self.instances.len() as u32);

            render_pass.draw_character_set(&self.queue, &mut self.character_set);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }
}

fn main() {
    env_logger::Builder::new()
        .parse_filters("warn,client=trace")
        .init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let connection = Connection::connect().unwrap();
    connection
        .send(&Packet::Login { username: [5; 20] })
        .unwrap();

    let mut state = pollster::block_on(State::new(&window, connection));

    event_loop.run(move |event, _, control_flow| match event {
        Event::RedrawRequested(_) => {
            state.update();
            match state.render() {
                Ok(_) => {}
                //Err(wgpu::SwapChainErrors::Lost) => state.resize(state.size),
                //Err(wgpu::SwapChainErrors::OutOfMemory) => *control_flow = ControlFlow::Exit,
                Err(e) => {
                    eprintln!("{:?}", e);
                    *control_flow = ControlFlow::Exit;
                }
            }
        }
        Event::MainEventsCleared => {
            window.request_redraw();
        }
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() => {
            if !state.input(event, &window) {
                match event {
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(physical_size) => {
                        info!("Resize requested to {:?}", physical_size);
                        state.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        info!(
                            "Scale factor changed to new_inner_size={:?}",
                            new_inner_size
                        );
                        state.resize(**new_inner_size);
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    });
}
