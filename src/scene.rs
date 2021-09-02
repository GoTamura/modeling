use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use wgpu::CommandEncoder;
use winit::dpi::PhysicalSize;

use crate::{camera::{Camera, CameraController}, light::{Light, LightObject, LightRaw, Lights}, model::{Material, Model}, renderer::{Renderer, RendererExt}, shader::Shader, texture};

type Materials = Arc<RwLock<HashMap<String, Arc<Material>>>>;
type Shaders = Arc<RwLock<HashMap<String, Arc<Shader>>>>;

#[derive(Debug)]
pub struct Scene {
    pub models: Vec<Model>,
    pub lights: Lights,
    pub camera: Camera,
    pub renderer: Renderer,
    pub materials: Materials,
    pub shaders: Shaders,
}

impl Scene {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        let light = Light::new(
            cgmath::Point3::new(200.0, 200.0, 2.0),
            cgmath::Vector3::new(1., 1., 1.),
            cgmath::Deg(45.),
            1.0..20.0,
        );
        let lights = Lights::new(device, vec!(LightObject::new(&device, light)));

        let size = PhysicalSize::<u32>::new(config.width, config.height);
        let camera = Camera::new(size);
        Self {
            models: Vec::new(),
            renderer: Renderer::new(device, config, &camera, &lights.lights[0]),
            lights,
            camera,
            materials: Arc::new(RwLock::new(HashMap::new())),
            shaders: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    pub fn draw(&self, encoder: &mut wgpu::CommandEncoder, frame_view: &wgpu::TextureView) {
        self.renderer
            .draw(encoder, frame_view, &self.models, &self.lights);
    }

    pub fn resize(&mut self, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) {
        use crate::camera::PerspectiveFovExt;
        self.camera.projection.resize(config.width, config.height);
        self.renderer.depth_texture =
            texture::Texture::create_depth_texture(device, config, "depth_texture");
    }

    pub fn update(&mut self, queue: &wgpu::Queue) {
        self.lights.lights[0].update(queue);
        self.renderer.update(queue, &self.camera);
    }
}
