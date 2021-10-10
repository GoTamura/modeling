use std::{collections::HashMap, path::Path, sync::{Arc, RwLock}};

use anyhow::*;

#[derive(Debug, Clone, Copy)]
pub struct ModelVertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
    normal: [f32; 3],
    tangent: [f32; 3],
    bitangent: [f32; 3],
}

type Models = Arc<RwLock<HashMap<String, Arc<Model>>>>;
pub struct Collection {
    pub models: Models,
}

impl Collection {
    pub fn new() -> Self {
        Self {
            models: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    pub fn add_model<S: AsRef<str>>(&mut self, model: Arc<Model>, key: S) {
        self.models.write().unwrap().insert(key.as_ref().to_string(), model.clone());
    }
    
    pub fn update_buffers(&self) {
        self.models.read().unwrap().iter().for_each(|m| m.1.update_buffers());
    }
    
}

#[derive(Debug)]
pub enum Model {
    OBJ(ObjModel),
    GLTF(GltfModel),
    RUNGHOLT(Rungholt),
}

impl Model {
    pub fn meshes(&self) -> &Vec<Mesh> {
        match self {
            Model::OBJ(ref m) => &m.meshes,
            Model::GLTF(ref m) => &m.meshes,
            Model::RUNGHOLT(ref m) => &m.meshes,
        }
    }
    
    pub fn update_buffers(&self) {
        match self {
            Model::OBJ(ref m) => &m.update_buffers(),
            Model::GLTF(ref m) => &(),
            Model::RUNGHOLT(ref m) => &(),
        };
    }
}
#[derive(Debug)]
pub struct ObjModel {
    pub meshes: Vec<Mesh>,
    pub is_dirty: bool,
}

#[derive(Debug)]
pub struct GltfModel {
    pub meshes: Vec<Mesh>,
    pub is_dirty: bool,
    // pub materials: Vec<Material>,
}

impl ObjModel {
    pub async fn load<P: AsRef<Path>>(
        path: P,
    ) -> Result<Self> {
        let (obj_models, obj_materials) = tobj::load_obj(
            path.as_ref(),
            &tobj::LoadOptions {
                triangulate: true,
                single_index: true,
                ..Default::default()
            },
        )?;

        // We're assuming that the texture files are stored with the obj file
        let containing_folder = path.as_ref().parent().context("Directory has no parent")?;

        for (i, mat) in obj_materials.unwrap().into_iter().enumerate() {
            let diffuse_path = &mat.diffuse_texture;

            let normal_path = &mat.normal_texture;

            let specular_path = &mat.specular_texture;

        }

        let mut meshes = Vec::new();
        for m in obj_models {
            let mut vertices = Vec::new();
            for i in 0..m.mesh.positions.len() / 3 {
                vertices.push(ModelVertex {
                    position: [
                        m.mesh.positions[i * 3],
                        m.mesh.positions[i * 3 + 1],
                        m.mesh.positions[i * 3 + 2],
                    ],
                    tex_coords: [m.mesh.texcoords[i * 2], 1.0 - m.mesh.texcoords[i * 2 + 1]],
                    normal: [
                        m.mesh.normals[i * 3],
                        m.mesh.normals[i * 3 + 1],
                        m.mesh.normals[i * 3 + 2],
                    ],
                    tangent: [0.0; 3],
                    bitangent: [0.0; 3],
                });
            }

            let indices = m.mesh.indices.clone();

            for c in indices.chunks(3) {
                let v0 = vertices[c[0] as usize];
                let v1 = vertices[c[1] as usize];
                let v2 = vertices[c[2] as usize];

                let p0: cgmath::Point3<_> = v0.position.into();
                let p1: cgmath::Point3<_> = v1.position.into();
                let p2: cgmath::Point3<_> = v2.position.into();

                let w0: cgmath::Point2<_> = v0.tex_coords.into();
                let w1: cgmath::Point2<_> = v1.tex_coords.into();
                let w2: cgmath::Point2<_> = v2.tex_coords.into();

                let dp1 = p1 - p0;
                let dp2 = p2 - p0;

                let dw1 = w1 - w0;
                let dw2 = w2 - w0;

                let r = 1.0 / (dw1.x * dw2.y - dw1.y * dw2.x);
                let tangent = (dp1 * dw2.y - dp2 * dw1.y) * r;
                let bitangent = (dp2 * dw1.x - dp1 * dw2.x) * r;

                vertices[c[0] as usize].tangent = tangent.into();
                vertices[c[1] as usize].tangent = tangent.into();
                vertices[c[2] as usize].tangent = tangent.into();

                vertices[c[0] as usize].bitangent = bitangent.into();
                vertices[c[1] as usize].bitangent = bitangent.into();
                vertices[c[2] as usize].bitangent = bitangent.into();
            }

            meshes.push(Mesh {
                name: m.name,
                vertices,
                indices,
                num_elements: m.mesh.indices.len() as u32,
            });
        }

        Ok(Self { meshes, is_dirty: true })
    }
    
    pub fn update_buffers(&self) {
        if self.is_dirty {
            // send message to wgpu
        }
    }

    //pub fn update(&mut self, queue: &wgpu::Queue, camera: &Camera) {
    //    self.renderer.update(queue, camera);
    //}
}

#[derive(Debug)]
pub struct Mesh {
    pub name: String,
    pub vertices: Vec<ModelVertex>,
    pub indices: Vec<u32>,
    pub num_elements: u32,
}

#[derive(Debug)]
pub struct Rungholt {
    pub meshes: Vec<Mesh>,
    pub is_dirty: bool,
}

impl Rungholt {
    pub async fn load<P: AsRef<Path>>(
        path: P,
    ) -> Result<Self> {
        let obj_bytes = include_bytes!("model/rungholt/house.obj");
        let mut obj_file = std::io::BufReader::new(&obj_bytes[..]);

        let (obj_models, obj_materials) = tobj::load_obj_buf(
            &mut obj_file,
            &tobj::LoadOptions {
                triangulate: true,
                single_index: true,
                ..Default::default()
            },
            |p| match p.file_name().unwrap().to_str().unwrap() {
                "house.mtl" => {
                    let mtl_bytes = include_bytes!("model/rungholt/house.mtl");
                    tobj::load_mtl_buf(&mut std::io::BufReader::new(&mtl_bytes[..]))
                }
                _ => unreachable!(),
            },
        )?;

        // We're assuming that the texture files are stored with the obj file
        let containing_folder = path.as_ref().parent().context("Directory has no parent")?;

        for (i, mat) in obj_materials.unwrap().into_iter().enumerate() {
            let diffuse_path = &mat.diffuse_texture;

            let normal_path = &mat.normal_texture;

            let specular_path = &mat.specular_texture;

        }

        let mut meshes = Vec::new();
        for m in obj_models {
            let mut vertices = Vec::new();
            for i in 0..m.mesh.positions.len() / 3 {
                vertices.push(ModelVertex {
                    position: [
                        m.mesh.positions[i * 3],
                        m.mesh.positions[i * 3 + 1],
                        m.mesh.positions[i * 3 + 2],
                    ],
                    tex_coords: [m.mesh.texcoords[i * 2], 1.0 - m.mesh.texcoords[i * 2 + 1]],
                    normal: [
                        m.mesh.normals[i * 3],
                        m.mesh.normals[i * 3 + 1],
                        m.mesh.normals[i * 3 + 2],
                    ],
                    tangent: [0.0; 3],
                    bitangent: [0.0; 3],
                });
            }

            let indices = m.mesh.indices.clone();

            for c in indices.chunks(3) {
                let v0 = vertices[c[0] as usize];
                let v1 = vertices[c[1] as usize];
                let v2 = vertices[c[2] as usize];

                let p0: cgmath::Point3<_> = v0.position.into();
                let p1: cgmath::Point3<_> = v1.position.into();
                let p2: cgmath::Point3<_> = v2.position.into();

                let w0: cgmath::Point2<_> = v0.tex_coords.into();
                let w1: cgmath::Point2<_> = v1.tex_coords.into();
                let w2: cgmath::Point2<_> = v2.tex_coords.into();

                let dp1 = p1 - p0;
                let dp2 = p2 - p0;

                let dw1 = w1 - w0;
                let dw2 = w2 - w0;

                let r = 1.0 / (dw1.x * dw2.y - dw1.y * dw2.x);
                let tangent = (dp1 * dw2.y - dp2 * dw1.y) * r;
                let bitangent = (dp2 * dw1.x - dp1 * dw2.x) * r;

                vertices[c[0] as usize].tangent = tangent.into();
                vertices[c[1] as usize].tangent = tangent.into();
                vertices[c[2] as usize].tangent = tangent.into();

                vertices[c[0] as usize].bitangent = bitangent.into();
                vertices[c[1] as usize].bitangent = bitangent.into();
                vertices[c[2] as usize].bitangent = bitangent.into();
            }

            meshes.push(Mesh {
                name: m.name,
                vertices,
                indices,
                num_elements: m.mesh.indices.len() as u32,
            });
        }

        Ok(Self { meshes, is_dirty: true })
    }
    
    pub fn update_buffers(&self) {
        if self.is_dirty {
            // send message to wgpu
        }
    }

    //pub fn update(&mut self, queue: &wgpu::Queue, camera: &Camera) {
    //    self.renderer.update(queue, camera);
    //}
}