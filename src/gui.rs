use chrono::Timelike;
use egui::FontDefinitions;
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};
use epi::*;
use std::time::{Duration, Instant};
pub enum Event {
    RequestRedraw,
}

use winit::{
    dpi::PhysicalSize,
    event::Event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};


fn seconds_since_midnight() -> f64 {
    let time = chrono::Local::now().time();
    time.num_seconds_from_midnight() as f64 + 1e-9 * (time.nanosecond() as f64)
}

/// This is the repaint signal type that egui needs for requesting a repaint from another thread.
/// It sends the custom RequestRedraw event to the winit event loop.
pub struct ExampleRepaintSignal(std::sync::Mutex<winit::event_loop::EventLoopProxy<Event>>);

impl epi::RepaintSignal for ExampleRepaintSignal {
    fn request_repaint(&self) {
        self.0.lock().unwrap().send_event(Event::RequestRedraw).ok();
    }
}
pub struct Gui {
    platform: Platform,
    render_pass: RenderPass,
    repaint_signal: std::sync::Arc<ExampleRepaintSignal>,
    app: egui_demo_lib::WrapApp,
}

impl Gui {
    pub fn new(
        device: &wgpu::Device,
        window: &Window,
        texture_format: wgpu::TextureFormat,
        event_loop: &EventLoop<Event>,
        size: PhysicalSize<u32>,
    ) -> Self {
        let repaint_signal = std::sync::Arc::new(ExampleRepaintSignal(std::sync::Mutex::new(
            event_loop.create_proxy(),
        )));

        // We use the egui_winit_platform crate as the platform.
        let platform = Platform::new(PlatformDescriptor {
            physical_width: size.width as u32,
            physical_height: size.height as u32,
            scale_factor: window.scale_factor(),
            font_definitions: FontDefinitions::default(),
            style: Default::default(),
        });

        // We use the egui_wgpu_backend crate as the render backend.
        let egui_rpass = RenderPass::new(&device, texture_format);

        // Display the demo application that ships with egui.
        let demo_app = egui_demo_lib::WrapApp::default();

        Gui {
            platform,
            render_pass: egui_rpass,
            repaint_signal,
            app: demo_app,
        }
    }

    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame_view: &wgpu::TextureView,
        start_time: Instant,
        previous_frame_time: &mut Option<f32>,
        window: &Window,
        width: u32,
        height: u32,
    ) {
        self.platform
            .update_time(start_time.elapsed().as_secs_f64());

        // Begin to draw the UI frame.
        let eself_start = Instant::now();
        self.platform.begin_frame();
        let mut app_output = epi::backend::AppOutput::default();

        let mut iframe = epi::backend::FrameBuilder {
            info: epi::IntegrationInfo {
                web_info: None,
                cpu_usage: *previous_frame_time,
                seconds_since_midnight: Some(seconds_since_midnight()),
                native_pixels_per_point: Some(window.scale_factor() as _),
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
        let (_output, paint_commands) = self.platform.end_frame();
        let paint_jobs = self.platform.context().tessellate(paint_commands);

        let frame_time = (Instant::now() - eself_start).as_secs_f64() as f32;
        *previous_frame_time = Some(frame_time);

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
            .execute(encoder, &frame_view, &paint_jobs, &screen_descriptor, None);
    }

    pub fn handle_event(&mut self, event: &winit::event::Event<Event>) {
        self.platform.handle_event(&event);
    }
}
