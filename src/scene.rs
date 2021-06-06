use crate::{light::Light, model::Model, camera::Camera};

pub struct Scene {
    pub models: Vec<Model>,
    pub lights: Vec<Light>,
    pub cameras: Vec<Camera>,
}

impl Scene {
    pub fn render(&self) {
    }
}