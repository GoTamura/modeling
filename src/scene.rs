use crate::{light::Light, model::Model};

pub struct Scene {
    pub models: Vec<Model>,
    pub lights: Vec<Light>,
}