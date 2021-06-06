use winit::{
    event::Event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use wgpu::util::DeviceExt;

use bytemuck::{Pod, Zeroable};
use std::time::{Duration, Instant};

use cgmath::prelude::*;

use crate::{
    camera, gui, light,
    model::{self, Vertex},
    scene, texture,
};

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct Uniforms {
    view_position: [f32; 4],
    view_proj: [[f32; 4]; 4],
}

impl Uniforms {
    fn new() -> Self {
        Self {
            view_position: [0.0; 4],
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    fn update_view_proj(&mut self, camera: &camera::Camera, projection: &camera::Projection) {
        self.view_position = camera.eye.to_homogeneous().into();
        self.view_proj = (projection.calc_matrix() * camera.calc_matrix()).into();
    }
}

struct Instance {
    position: cgmath::Vector3<f32>,
    rotation: cgmath::Quaternion<f32>,
}

impl Instance {
    fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: (cgmath::Matrix4::from_translation(self.position)
                * cgmath::Matrix4::from(self.rotation))
            .into(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceRaw {
    model: [[f32; 4]; 4],
}

impl InstanceRaw {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
            step_mode: wgpu::InputStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    // While our vertex shader only uses locations 0, and 1 now, in later tutorials we'll
                    // be using 2, 3, and 4, for Vertex. We'll start at slot 5 not conflict with them later
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a slot
                // for each vec4. We don't have to do this in code though.
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

const NUM_INSTANCES_PER_ROW: u32 = 10;
pub struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    camera: camera::Camera,
    projection: camera::Projection,
    uniforms: Uniforms,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    camera_controller: camera::CameraController,
    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,
    depth_texture: texture::Texture,
    light: light::LightW,
    light_render_pipeline: wgpu::RenderPipeline,
    scene: scene::Scene,

    pub gui: gui::Gui,
}

impl State {
    pub fn handle_event<T>(
        &mut self,
        event: &winit::event::Event<T>,
        control_flow: &mut ControlFlow,
        window: &Window,
        start_time: Instant,
        last_update_inst: &mut Instant,
        previous_frame_time: &mut Option<f32>,
    ) {
        match event {
            RedrawRequested(_) => {
                self.render(start_time, previous_frame_time, &window);
            }
            RedrawEventsCleared => {
                let target_frametime = Duration::from_secs_f64(1.0 / 60.0);
                let time_since_last_frame = last_update_inst.elapsed();
                if time_since_last_frame >= target_frametime {
                    window.request_redraw();
                    *last_update_inst = Instant::now();
                } else {
                    *control_flow = ControlFlow::WaitUntil(
                        Instant::now() + target_frametime - time_since_last_frame,
                    );
                }

                //window.request_redraw();
            }
            MainEventsCleared => {
                self.update();
            }
            WindowEvent {
                ref event,
                window_id,
            } if *window_id == window.id() => {
                if !self.input(event) {
                    match event {
                        winit::event::WindowEvent::CloseRequested => {
                            *control_flow = ControlFlow::Exit
                        }
                        winit::event::WindowEvent::KeyboardInput { input, .. } => match input {
                            winit::event::KeyboardInput {
                                state: winit::event::ElementState::Pressed,
                                virtual_keycode: Some(winit::event::VirtualKeyCode::Escape),
                                ..
                            } => *control_flow = ControlFlow::Exit,
                            _ => {}
                        },
                        winit::event::WindowEvent::Resized(physical_size) => {
                            self.resize(*physical_size);
                        }
                        winit::event::WindowEvent::ScaleFactorChanged {
                            new_inner_size, ..
                        } => {
                            self.resize(**new_inner_size);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}

impl State {
    fn create_render_pipeline(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        color_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
        vertex_layouts: &[wgpu::VertexBufferLayout],
        shader: &wgpu::ShaderModule,
        //vs_module: &wgpu::ShaderModule,
        //fs_module: &wgpu::ShaderModule,
    ) -> wgpu::RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main", // 1.
                buffers: vertex_layouts,
            },
            fragment: Some(wgpu::FragmentState {
                // 2.
                module: &shader,
                entry_point: "fs_main",
                targets: &[wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrite::ALL,
                }],
            }),

            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                //..Default::default()
                strip_index_format: None,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                clamp_depth: device.features().contains(wgpu::Features::DEPTH_CLAMPING),
                conservative: false,
            },

            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format.unwrap_or_else(|| texture::Texture::DEPTH_FORMAT),
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less, // 1.
                stencil: wgpu::StencilState::default(),     // 2.
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            // {
            //    count: 1,
            //    mask: !0,
            //    alpha_to_coverage_enabled: false,
            //},
        })
    }

    fn create_render_pipeline2(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        color_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
        vertex_layouts: &[wgpu::VertexBufferLayout],
        //shader: &wgpu::ShaderModule,
        vs_module: &wgpu::ShaderModule,
        fs_module: &wgpu::ShaderModule,
    ) -> wgpu::RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &vs_module,
                entry_point: "main", // 1.
                buffers: vertex_layouts,
            },
            fragment: Some(wgpu::FragmentState {
                // 2.
                module: &fs_module,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrite::ALL,
                }],
            }),

            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                ..Default::default()
                //strip_index_format: None,
                //cull_mode: Some(wgpu::Face::Back),
                //polygon_mode: wgpu::PolygonMode::Fill,
                //clamp_depth: device.features().contains(wgpu::Features::DEPTH_CLAMPING),
                //conservative: false,
            },

            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format.unwrap_or_else(|| texture::Texture::DEPTH_FORMAT),
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less, // 1.
                stencil: wgpu::StencilState::default(),     // 2.
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            // {
            //    count: 1,
            //    mask: !0,
            //    alpha_to_coverage_enabled: false,
            //},
        })
    }

    fn create_box_render_pipeline(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        color_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
        vertex_layouts: &[wgpu::VertexBufferLayout],
        //shader: &wgpu::ShaderModuleDescriptor,
        vertex_shader: &wgpu::ShaderModuleDescriptor,
        fragent_shader: &wgpu::ShaderModuleDescriptor,
    ) -> wgpu::RenderPipeline {
        let vs_module = device.create_shader_module(vertex_shader);
        let fs_module = device.create_shader_module(fragent_shader);
        //let shader_module = device.create_shader_module(&shader);

        Self::create_render_pipeline2(
            &device,
            &layout,
            color_format,
            depth_format,
            vertex_layouts,
            //&shader_module,
            &vs_module,
            &fs_module,
        )
    }

    fn create_light_render_pipeline(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        color_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
        vertex_layouts: &[wgpu::VertexBufferLayout],
    ) -> wgpu::RenderPipeline {
        let vs_module = device.create_shader_module(&wgpu::include_spirv!("light.vert.spv"));
        let fs_module = device.create_shader_module(&wgpu::include_spirv!("light.frag.spv"));

        Self::create_render_pipeline2(
            &device,
            &layout,
            color_format,
            depth_format,
            vertex_layouts,
            &vs_module,
            &fs_module,
        )
    }

    // Creating some of the wgpu types requires async code
    pub async fn new(
        window: &Window,
        texture_format: wgpu::TextureFormat,
        event_loop: &EventLoop<gui::Event>,
    ) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
            })
            .await
            .expect("No suitable GPU adapters found on the system!");
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .expect("Unable to find a suitable GPU adapter!");
        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            //format: texture_format,
            format: adapter.get_swap_chain_preferred_format(&surface).unwrap(),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        let gui = gui::Gui::new(&device, window, sc_desc.format, event_loop, size);

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler {
                            comparison: false,
                            filtering: true,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler {
                            comparison: false,
                            filtering: true,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler {
                            comparison: false,
                            filtering: true,
                        },
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let camera = camera::Camera::default();
        let camera_controller = camera::CameraController::new(0.2);

        let projection = camera::Projection::new(
            sc_desc.width,
            sc_desc.height,
            cgmath::Deg(45.0),
            0.1,
            100000.0,
        );

        let mut uniforms = Uniforms::new();
        uniforms.update_view_proj(&camera, &projection);

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
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
            label: Some("uniform_bing_group"),
        });

        let light = light::Light {
            position: [200.0, 200.0, 2.0],
            _padding: 0,
            color: [1.0, 1.0, 1.0],
        };

        let light = light::LightW::new(&device, light);
        const SPACE_BETWEEN: f32 = 3.0;
        let instances = (0..NUM_INSTANCES_PER_ROW)
            .flat_map(|z| {
                (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                    let position = cgmath::Vector3 {
                        x: SPACE_BETWEEN * (x as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0),
                        y: 0.0,
                        z: SPACE_BETWEEN * (z as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0),
                    };

                    let rotation = if position.is_zero() {
                        // this is needed so an object at (0, 0, 0) won't get scaled to zero
                        // as Quaternions can effect scale if they're not created correctly
                        cgmath::Rotation3::from_axis_angle(
                            cgmath::Vector3::unit_z(),
                            cgmath::Deg(0.0),
                        )
                    } else {
                        cgmath::Rotation3::from_axis_angle(
                            position.clone().normalize(),
                            cgmath::Deg(45.0),
                        )
                    };

                    Instance { position, rotation }
                })
            })
            .collect::<Vec<_>>();
        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsage::VERTEX,
        });

        let res_dir = std::path::Path::new(env!("OUT_DIR")).join("res");
        let model = model::ObjModel::load(
            &device,
            &queue,
            &texture_bind_group_layout,
            //res_dir.join("rungholt/rungholt.obj"),
            res_dir.join("sponza.obj"),
        );
        //let model = model::GltfModel::load(
        //    &device,
        //    &queue,
        //    &texture_bind_group_layout,
        //    //res_dir.join("AliciaSolid.vrm"),
        //    res_dir.join("AliciaSolid.vrm"),
        //);

        let depth_texture =
            texture::Texture::create_depth_texture(&device, &sc_desc, "depth_texture");

        let render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &texture_bind_group_layout,
                    &light.bind_group_layout,
                    &uniform_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
            let vertex_shader = wgpu::include_spirv!("shader.vert.spv");
            let fragment_shader = wgpu::include_spirv!("shader.frag.spv");
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Normal Shader"),
                flags: wgpu::ShaderFlags::all(),
                source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
            };
            Self::create_box_render_pipeline(
                &device,
                &layout,
                sc_desc.format,
                Some(texture::Texture::DEPTH_FORMAT),
                &[model::ModelVertex::desc(), InstanceRaw::desc()],
                //&shader,
                &vertex_shader,
                &fragment_shader,
            )
        };
        let light_render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Light Pipeline Layout"),
                bind_group_layouts: &[&uniform_bind_group_layout, &light.bind_group_layout],
                push_constant_ranges: &[],
            });
            Self::create_light_render_pipeline(
                &device,
                &layout,
                sc_desc.format,
                Some(texture::Texture::DEPTH_FORMAT),
                &[model::ModelVertex::desc()],
            )
        };

        let model = model::Model::OBJ(model.await.unwrap());
        let light_model = model::Model::OBJ(
            model::ObjModel::load(
                &device,
                &queue,
                &texture_bind_group_layout,
                res_dir.join("cube.obj"),
            )
            .await
            .unwrap(),
        );
        //let model = model::Model::GLTF(model.await.unwrap());
        let mut scene = scene::Scene {
            models: Vec::new(),
            lights: Vec::new(),
            cameras: Vec::new(),
        };
        scene.models.push(model);
        scene.models.push(light_model);

        Self {
            surface,
            device,
            queue,
            sc_desc,
            swap_chain,
            size,
            render_pipeline,
            camera,
            projection,
            uniforms,
            uniform_buffer,
            uniform_bind_group,
            camera_controller,
            instances,
            instance_buffer,
            depth_texture,
            light,
            light_render_pipeline,
            scene,
            gui,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.projection
            .resize(self.sc_desc.width, self.sc_desc.height);
        self.depth_texture =
            texture::Texture::create_depth_texture(&self.device, &self.sc_desc, "depth_texture");
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

    fn input(&mut self, event: &winit::event::WindowEvent) -> bool {
        self.camera_controller.process_events(event)
    }

    fn update(&mut self) {
        self.camera_controller.update_camera(&mut self.camera);
        self.uniforms
            .update_view_proj(&self.camera, &self.projection);
        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms]),
        );

        let old_position: cgmath::Vector3<f32> = self.light.light.position.into();
        let rot: cgmath::Quaternion<f32> = cgmath::Rotation3::from_axis_angle(
            cgmath::Vector3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            cgmath::Deg(1.0),
        );

        let pos: cgmath::Vector3<f32> = rot * old_position;
        self.light.light.position = pos.into();

        self.queue.write_buffer(
            &self.light.buffer,
            0,
            bytemuck::cast_slice(&[self.light.light]),
        );
    }

    fn render(
        &mut self,
        start_time: Instant,
        previous_frame_time: &mut Option<f32>,
        window: &Window,
    ) {
        let frame = self
            .swap_chain
            .get_current_frame()
            .expect("Timeout getting texture")
            .output;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
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

            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

            //for model in &self.scene.models {
            render_pass.set_pipeline(&self.render_pipeline);
            use model::DrawModel;
            //render_pass.draw_model_instanced(
            render_pass.draw_model(
                &self.scene.models[0],
                //&model,
                //0..self.instances.len() as u32,
                &self.uniform_bind_group,
                &self.light.bind_group,
            );

            render_pass.set_pipeline(&self.light_render_pipeline);
            use model::DrawLight;

            render_pass.draw_light_model(
                &self.scene.models[1],
                //&model,
                &self.uniform_bind_group,
                &self.light.bind_group,
            );
            // }
        }

        self.gui.draw(
            &self.device,
            &self.queue,
            &mut encoder,
            &frame.view,
            start_time,
            previous_frame_time,
            window,
            self.sc_desc.width,
            self.sc_desc.height,
        );

        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
    }
}
