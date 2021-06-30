use winit::{
    dpi::PhysicalSize,
    event::Event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use wgpu::util::DeviceExt;

use bytemuck::{Pod, Zeroable};
use std::time::{Duration, Instant};

use cgmath::prelude::*;

use crate::{
    camera::{self, CameraController},
    gui, light,
    model::{self, Vertex},
    renderer::RendererExt,
    scene, texture,
};

pub struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    size: winit::dpi::PhysicalSize<u32>,
    scene: scene::Scene,
    camera_controller: camera::CameraController,

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

        let res_dir = std::path::Path::new(env!("OUT_DIR")).join("res");
        //let model = model::Model::GLTF(model.await.unwrap());
        let mut scene = scene::Scene::new(&device, &sc_desc);

        let model = model::ObjModel::load(
            &device,
            &queue,
            //res_dir.join("breakfast_room.obj"),
            //res_dir.join("sponza.obj"),
            res_dir.join("rungholt/rungholt.obj"),
            &sc_desc,
            &scene,
        );

        let model = model::Model::OBJ(model.await.unwrap());
        let light_model = model::Model::OBJ(
            model::ObjModel::load(
                &device,
                &queue,
                res_dir.join("cube.obj"),
                &sc_desc,
                &scene,
            )
            .await
            .unwrap(),
        );
        scene.models.push(model);
        scene.models.push(light_model);

        let camera_controller = CameraController::new(0.2, size);

        Self {
            surface,
            device,
            queue,
            sc_desc,
            swap_chain,
            size,
            scene,
            camera_controller,
            gui,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
        self.scene.resize(&self.device, &self.sc_desc);
        self.camera_controller.size = self.size;
    }

    fn input(&mut self, event: &winit::event::WindowEvent) -> bool {
        self.camera_controller.process_events(event, self.size)
    }

    fn update(&mut self) {
        self.camera_controller.update_camera(&mut self.scene.camera);
        self.scene.update(&self.queue);
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

        self.scene.draw(&mut encoder, &frame.view);

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
