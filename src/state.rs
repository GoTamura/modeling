use winit::{
    dpi::PhysicalSize,
    event::Event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use wgpu::util::DeviceExt;

use bytemuck::{Pod, Zeroable};
use std::{
    sync::{Arc, RwLock},
    time::{Duration},
};
use instant::Instant;

use cgmath::prelude::*;

use crate::{
    camera::{self, CameraController},
    collection, gui, light,
    model::{self, Vertex},
    renderer::RendererExt,
    scene, texture,
};

pub struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    scene: Arc<RwLock<scene::Scene>>,
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
        let backend = wgpu::util::backend_bits_from_env().unwrap_or_else(wgpu::Backends::all);
        let instance = wgpu::Instance::new(backend);
        let (size, surface) = unsafe {
            let size = window.inner_size();
            let surface = instance.create_surface(window);
            (size, surface)
        };
        let adapter =
            wgpu::util::initialize_adapter_from_env_or_default(&instance, backend, Some(&surface))
                .await
                .expect("No suitable GPU adapters found on the system!");
        #[cfg(not(target_arch = "wasm32"))]
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
        #[cfg(target_arch = "wasm32")]
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::default(),
                    limits: wgpu::Limits::downlevel_webgl2_defaults()
                        .using_resolution(adapter.limits()),
                    label: None,
                },
                None,
            )
            .await
            .expect("Unable to find a suitable GPU adapter!");
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            //format: texture_format,
            format: surface.get_preferred_format(&adapter).unwrap(),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        surface.configure(&device, &config);

        let res_dir = std::path::Path::new(env!("OUT_DIR")).join("res");
        //let model = model::Model::GLTF(model.await.unwrap());
        let mut scene = Arc::new(RwLock::new(scene::Scene::new(&device, &config)));
        let mut collection = Arc::new(RwLock::new(collection::Collection::new()));
        collection.write().unwrap().add_model(
            Arc::new(collection::Model::RUNGHOLT(
                collection::Rungholt::load(res_dir.join("rungholt/rungholt.obj"))
                    .await
                    .unwrap(),
            )),
            "rungholt",
        );
        let gui = gui::Gui::new(
            &device,
            window,
            config.format,
            event_loop,
            size,
            scene.clone(),
            collection.clone(),
        );

        // let model = model::ObjModel::load(
        let model = model::House::load(
            &device,
            &queue,
            //res_dir.join("breakfast_room.obj"),
            //res_dir.join("sponza.obj"),
            res_dir.join("rungholt/rungholt.obj"),
            &config,
            scene.clone(),
        );

        let model = model::Model::HOUSE(model.await.unwrap());
        //let light_model = model::Model::OBJ(
        //    model::ObjModel::load(
        //        &device,
        //        &queue,
        //        res_dir.join("cube.obj"),
        //        &config,
        //        scene.clone(),
        //    )
        //    .await
        //    .unwrap(),
        //);
        scene.write().unwrap().models.push(model);
        // scene.write().unwrap().models.push(light_model);

        let camera_controller = CameraController::new(0.2, size);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            scene,
            camera_controller,
            gui,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
        self.scene
            .write()
            .unwrap()
            .resize(&self.device, &self.config);
        self.camera_controller.size = self.size;
    }

    fn input(&mut self, event: &winit::event::WindowEvent) -> bool {
        self.camera_controller.process_events(event, self.size)
    }

    fn update(&mut self) {
        self.camera_controller
            .update_camera(&mut self.scene.write().unwrap().camera);
        self.scene.write().unwrap().update(&self.queue);
    }

    fn render(
        &mut self,
        start_time: Instant,
        previous_frame_time: &mut Option<f32>,
        window: &Window,
    ) {
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(_) => {
                self.surface.configure(&self.device, &self.config);
                self.surface
                    .get_current_texture()
                    .expect("Failed to acquire next surface texture!")
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        self.scene.read().unwrap().draw(&mut encoder, &view);

        self.gui.draw(
            &self.device,
            &self.queue,
            &mut encoder,
            &view,
            start_time,
            previous_frame_time,
            window,
            self.config.width,
            self.config.height,
        );

        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }
}
