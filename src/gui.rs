use chrono::Timelike;
use egui::FontDefinitions;
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};
use epi::*;
use std::{
    sync::{Arc, RwLock},
};

#[cfg(not(target_arch = "wasm32"))]
use std:: time::{Duration, Instant };

use anyhow::*;
pub enum Event {
    RequestRedraw,
}

use winit::{
    dpi::PhysicalSize,
    event::Event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use crate::{collection::{self, Collection}, scene::Scene};

#[cfg(not(target_arch = "wasm32"))]
fn seconds_since_midnight() -> f64 {
    let time = chrono::Local::now().time();
    time.num_seconds_from_midnight() as f64 + 1e-9 * (time.nanosecond() as f64)
}

/// This is the repaint signal type that egui needs for requesting a repaint from another thread.
/// It sends the custom RequestRedraw event to the winit event loop.
#[cfg(not(target_arch = "wasm32"))]
pub struct ExampleRepaintSignal(std::sync::Mutex<winit::event_loop::EventLoopProxy<Event>>);
#[cfg(target_arch = "wasm32")]
pub struct ExampleRepaintSignal();

impl epi::RepaintSignal for ExampleRepaintSignal {
    fn request_repaint(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        self.0.lock().unwrap().send_event(Event::RequestRedraw).ok();
    }
}
pub struct Gui {
    platform: Platform,
    render_pass: RenderPass,
    repaint_signal: std::sync::Arc<ExampleRepaintSignal>,
    app: Box<dyn epi::App>,
    // app: egui_demo_lib::WrapApp,
}

impl Gui {
    pub fn new(
        device: &wgpu::Device,
        window: &Window,
        texture_format: wgpu::TextureFormat,
        event_loop: &EventLoop<Event>,
        size: PhysicalSize<u32>,
        scene: Arc<RwLock<Scene>>,
        collection: Arc<RwLock<Collection>>,
    ) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let repaint_signal = std::sync::Arc::new(ExampleRepaintSignal(std::sync::Mutex::new(
            event_loop.create_proxy(),
        )));
        #[cfg(target_arch = "wasm32")]
        let repaint_signal = std::sync::Arc::new(ExampleRepaintSignal());

        // We use the egui_winit_platform crate as the platform.
        let platform = Platform::new(PlatformDescriptor {
            physical_width: size.width as u32,
            physical_height: size.height as u32,
            scale_factor: window.scale_factor(),
            font_definitions: FontDefinitions::default(),
            style: Default::default(),
        });

        // We use the egui_wgpu_backend crate as the render backend.
        let msaa_samples = 1;
        let egui_rpass = RenderPass::new(&device, texture_format, msaa_samples);

        // Display the demo application that ships with egui.
        // let demo_app = egui_demo_lib::WrapApp::default();
        let demo_app = MyApp::new(scene, collection);

        Gui {
            platform,
            render_pass: egui_rpass,
            repaint_signal,
            app: Box::new(demo_app),
        }
    }

    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame_view: &wgpu::TextureView,
    #[cfg(not(target_arch = "wasm32"))]
        start_time: Instant,
    #[cfg(not(target_arch = "wasm32"))]
        previous_frame_time: &mut Option<f32>,
        window: &Window,
        width: u32,
        height: u32,
    ) -> Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        self.platform .update_time(start_time.elapsed().as_secs_f64());

        // Begin to draw the UI frame.
        #[cfg(not(target_arch = "wasm32"))]
        let eself_start = Instant::now();
        self.platform.begin_frame();
        let mut app_output = epi::backend::AppOutput::default();

        let mut iframe = epi::backend::FrameBuilder {
            info: epi::IntegrationInfo {
                web_info: None,
                #[cfg(not(target_arch = "wasm32"))]
                cpu_usage: *previous_frame_time,
                #[cfg(target_arch = "wasm32")]
                cpu_usage: None,
                #[cfg(not(target_arch = "wasm32"))]
                seconds_since_midnight: Some(seconds_since_midnight()),
                #[cfg(target_arch = "wasm32")]
                seconds_since_midnight: None,
                native_pixels_per_point: Some(window.scale_factor() as _),
                prefer_dark_mode: None,
            },
            tex_allocator: &mut self.render_pass,
            output: &mut app_output,
            repaint_signal: self.repaint_signal.clone(),
        }
        .build();

        // Draw the demo application.
        //use eself_demo_lib::WrapApp::*;
        self.app.update(&self.platform.context(), &mut iframe);

        // End the UI frame. We could now handle the output and draw the UI with the backend.
        let (_output, paint_commands) = self.platform.end_frame(Some(window));
        let paint_jobs = self.platform.context().tessellate(paint_commands);

        #[cfg(not(target_arch = "wasm32"))]
        {
        let frame_time = (Instant::now() - eself_start).as_secs_f64() as f32;
        *previous_frame_time = Some(frame_time);
        }

        // Upload all resources for the GPU.
        let screen_descriptor = ScreenDescriptor {
            physical_width: width,
            physical_height: height,
            scale_factor: window.scale_factor() as f32,
        };
        self.render_pass
            .update_texture(device, queue, &self.platform.context().texture());
        self.render_pass.update_user_textures(device, queue);
        self.render_pass
            .update_buffers(device, queue, &paint_jobs[..], &screen_descriptor);

        // Record all render passes.
        self.render_pass
            .execute(encoder, &frame_view, &paint_jobs, &screen_descriptor, None)?;
        Ok(())
    }

    pub fn handle_event<T>(&mut self, event: &winit::event::Event<T>) {
        self.platform.handle_event(event);
    }
}

struct MyApp {
    scene: Arc<RwLock<Scene>>,
    collection: Arc<RwLock<Collection>>,
    counter: u32,
}

impl MyApp {
    fn new(scene: Arc<RwLock<Scene>>, collection: Arc<RwLock<Collection>>) -> Self {
        Self { scene, counter: 0, collection }
    }
}

impl epi::App for MyApp {
    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut Frame<'_>) {
        egui::Window::new("wrap_app_top_bar")
            .min_width(50.0)
            .show(ctx, |ui| {
                egui::trace!(ui);
                ui.vertical(|ui| {
                    if ui.button("Compile shader").clicked() {
                        for shader in self.scene.write().unwrap().shaders.read().unwrap().iter() {
                            //TODO shader.1.recompile()
                        }
                    }
                    for (s, model) in self.collection.read().unwrap().models.read().unwrap().iter() {
                        ui.label(s);
                    }
                    if ui.button("-").clicked() {
                        self.counter -= 1;
                    }
                    ui.label(self.counter.to_string());
                    if ui.button("+").clicked() {
                        self.counter += 1;
                    }
                    let text_style = egui::TextStyle::Body;
                    let row_height = ui.fonts()[text_style].row_height();
                    // let row_height = ui.spacing().interact_size.y; // if you are adding buttons instead of labels.
                    let num_rows = self.scene.read().unwrap().materials.read().unwrap().len();
                    egui::ScrollArea::auto_sized().show_rows(
                        ui,
                        row_height,
                        num_rows,
                        |ui, row_range| {
                            // for row in row_range {
                                // let text = format!("Row {}/{}", row + 1, num_rows);
                                // ui.label(text);
                            // }
                            for (i, material) in self.scene.read().unwrap().materials.read().unwrap().iter().enumerate() {
                                if row_range.contains(&i) {
                                    ui.label(material.0);
                                }
                            }
                        },
                    );
                    for material in self.scene.read().unwrap().materials.read().unwrap().iter() {
                        ui.label(material.0);
                    }
                });
            });
    }

    fn name(&self) -> &str {
        "MyApp"
    }
}
