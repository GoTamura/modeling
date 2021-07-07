use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use wgpu::CommandEncoder;
use winit::dpi::PhysicalSize;

use crate::{
    camera::{Camera, CameraController},
    light::{Light, LightRaw},
    model::{Material, Model},
    renderer::{Renderer, RendererExt},
    shader::Shader,
    texture,
};

type Materials = Arc<RwLock<HashMap<String, Arc<Material>>>>;
type Shaders = Arc<RwLock<HashMap<String, Arc<Shader>>>>;

#[derive(Debug)]
pub struct Scene {
    pub models: Vec<Model>,
    pub light: Light,
    pub camera: Camera,
    pub renderer: Renderer,
    pub materials: Materials,
    pub shaders: Shaders,
}

impl Scene {
    pub fn new(device: &wgpu::Device, sc_desc: &wgpu::SwapChainDescriptor) -> Self {
        let light_raw = LightRaw {
            position: [200.0, 200.0, 2.0],
            _padding: 0,
            color: [1.0, 1.0, 1.0],
        };
        let light = Light::new(&device, light_raw);
        let size = PhysicalSize::<u32>::new(sc_desc.width, sc_desc.height);
        let camera = Camera::new(size);
        Self {
            models: Vec::new(),
            renderer: Renderer::new(device, sc_desc, &camera, &light),
            light,
            camera,
            materials: Arc::new(RwLock::new(HashMap::new())),
            shaders: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    pub fn draw(&self, encoder: &mut wgpu::CommandEncoder, frame_view: &wgpu::TextureView) {
        self.renderer
            .draw(encoder, frame_view, &self.models, &self.light);
    }

    pub fn resize(&mut self, device: &wgpu::Device, sc_desc: &wgpu::SwapChainDescriptor) {
        self.camera.projection.resize(sc_desc.width, sc_desc.height);
        self.renderer.depth_texture =
            texture::Texture::create_depth_texture(device, sc_desc, "depth_texture");
    }

    pub fn update(&mut self, queue: &wgpu::Queue) {
        self.light.update(queue);
        self.renderer.update(queue, &self.camera);
    }
}
