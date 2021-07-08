use crate::scene::Scene;
use crate::shader;
use crate::texture;
use anyhow::*;
use std::ops::Range;
use std::path::Path;
use std::sync::Arc;
use std::sync::RwLock;
use wgpu::util::DeviceExt;

pub trait Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
    normal: [f32; 3],
    tangent: [f32; 3],
    bitangent: [f32; 3],
}

impl Vertex for ModelVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 11]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

#[derive(Debug)]
pub enum Model {
    OBJ(ObjModel),
    GLTF(GltfModel),
}

impl Model {
    pub fn meshes(&self) -> &Vec<Mesh> {
        match self {
            Model::OBJ(ref m) => &m.meshes,
            Model::GLTF(ref m) => &m.meshes,
        }
    }
}
#[derive(Debug)]
pub struct ObjModel {
    pub meshes: Vec<Mesh>,
}

#[derive(Debug)]
pub struct GltfModel {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

impl ObjModel {
    pub async fn load<P: AsRef<Path>>(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: P,
        sc_desc: &wgpu::SwapChainDescriptor,
        scene: Arc<RwLock<Scene>>,
    ) -> Result<Self> {
        let scene = scene.read().unwrap();
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

        let mut material_keys = Vec::new();

        let mut materials = Vec::new();
        for (i, mat) in obj_materials.unwrap().into_iter().enumerate() {
            let diffuse_path = &mat.diffuse_texture;
            let diffuse_texture = if !diffuse_path.is_empty() {
                texture::Texture::load(device, queue, containing_folder.join(diffuse_path), false)
                    .with_context(|| format!("Diffuse texture: {} not found", diffuse_path))?
                // .unwrap_or_else(|_| panic!("Diffuse texture: {} not found", diffuse_path))
            } else {
                let mut diffuse_color = mat
                    .diffuse
                    .iter()
                    .map(|i| (i * 255.) as u8)
                    .collect::<Vec<u8>>();
                diffuse_color.push(0xff);
                texture::Texture::one_pixel(
                    device,
                    queue,
                    &diffuse_color,
                    Some("diffuse texture"),
                    true,
                )
            };

            let normal_path = &mat.normal_texture;
            let normal_texture = if !normal_path.is_empty() {
                texture::Texture::load(device, queue, containing_folder.join(normal_path), true)
                    .with_context(|| format!("Normal texture: {} not found", normal_path))?
            } else {
                texture::Texture::one_pixel(
                    device,
                    queue,
                    &[0x80, 0x80, 0xff, 0],
                    Some("default normal texture"),
                    true,
                )
            };

            let specular_path = &mat.specular_texture;
            let specular_texture = if !specular_path.is_empty() {
                texture::Texture::load(device, queue, containing_folder.join(specular_path), false)
                    .with_context(|| format!("Diffuse texture: {} not found", specular_path))?
            } else {
                let mut specular_color = mat
                    .specular
                    .iter()
                    .map(|i| (i * 255.) as u8)
                    .collect::<Vec<u8>>();
                specular_color.push(0xff);
                texture::Texture::one_pixel(
                    device,
                    queue,
                    &specular_color,
                    Some("specular texture"),
                    true,
                )
            };

            let shader_key = std::path::Path::new(env!("OUT_DIR"))
                .join("shader")
                .to_string_lossy()
                .into_owned();
            let shader = scene
                .shaders
                .write()
                .unwrap()
                .entry(shader_key)
                .or_insert_with(|| {
                    Arc::new(shader::Shader::new(
                        "obj vertex shader",
                        std::path::Path::new(env!("OUT_DIR")).join("shader"),
                        device,
                        &scene.renderer.texture_bind_group_layout,
                        &scene.light.bind_group_layout,
                        &scene.renderer.uniforms.bind_group_layout,
                        &sc_desc.format,
                    ))
                })
                .clone();

            let material_key = format!("{}-{}", &mat.name, i);
            let material = scene
                .materials
                .write()
                .unwrap()
                .entry(material_key.clone())
                .or_insert_with(|| {
                    Arc::new(Material::new(
                        device,
                        &mat.name,
                        diffuse_texture,
                        normal_texture,
                        specular_texture,
                        i as u32,
                        &scene.renderer.texture_bind_group_layout,
                        shader,
                    ))
                })
                .clone();
            materials.push(material);
            material_keys.push(material_key.clone());
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

            let indices = &m.mesh.indices;

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

            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Vertex Buffer", path.as_ref())),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsage::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Index Buffer", path.as_ref())),
                contents: bytemuck::cast_slice(&m.mesh.indices),
                usage: wgpu::BufferUsage::INDEX,
            });

            meshes.push(Mesh {
                name: m.name,
                vertex_buffer,
                index_buffer,
                num_elements: m.mesh.indices.len() as u32,
                material: scene
                    .materials
                    .read()
                    .unwrap()
                    .get(&material_keys[m.mesh.material_id.unwrap()])
                    .unwrap()
                    .clone(),
            });
        }

        Ok(Self { meshes })
    }

    //pub fn update(&mut self, queue: &wgpu::Queue, camera: &Camera) {
    //    self.renderer.update(queue, camera);
    //}
}

//impl GltfModel {
//    pub async fn load<P: AsRef<Path>>(
//        device: &wgpu::Device,
//        queue: &wgpu::Queue,
//        layout: &wgpu::BindGroupLayout,
//        path: P,
//    ) -> Result<Self> {
//        let (gltf, buffers, _) = tokio::task::block_in_place(|| gltf::import(path.as_ref()))?;
//
//        let materials = gltf
//            .materials()
//            .flat_map(|material| {
//                //let materials = gltf.materials().par_bridge().map(|material| {
//                if let Some(base_color_texture) =
//                    material.pbr_metallic_roughness().base_color_texture()
//                {
//                    let diffuse_texture =
//                        texture::Texture::load_gltf(device, queue, &base_color_texture, &buffers)
//                            .unwrap();
//
//                    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
//                        layout,
//                        entries: &[
//                            wgpu::BindGroupEntry {
//                                binding: 0,
//                                resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
//                            },
//                            wgpu::BindGroupEntry {
//                                binding: 1,
//                                resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
//                            },
//                        ],
//                        label: None,
//                    });
//
//                    Some(Material {
//                        name: material.name().unwrap().to_string(),
//                        diffuse_texture,
//                        bind_group,
//                        id: material
//                            .pbr_metallic_roughness()
//                            .base_color_texture()
//                            .unwrap()
//                            .texture()
//                            .index() as u32,
//                    })
//                } else {
//                    None
//                }
//            })
//            .collect();
//
//        let label_path = path.as_ref().to_str().map(|str| str.to_string());
//
//        //let meshes = gltf.meshes().map(|mesh| {
//        let meshes = gltf
//            .meshes()
//            .par_bridge()
//            .map(|mesh| {
//                println!("Mesh #{}", mesh.index());
//                //mesh.primitives().map(|primitive| {
//                mesh.primitives()
//                    .par_bridge()
//                    .map(|primitive| {
//                        println!("- Primitive #{}", primitive.index());
//                        let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
//                        let vertex_iter = reader.read_positions().unwrap();
//
//                        let tex_coord = primitive
//                            .material()
//                            .pbr_metallic_roughness()
//                            .base_color_texture()
//                            .unwrap()
//                            .tex_coord();
//                        let tex_coords_iter = match reader.read_tex_coords(tex_coord) {
//                            Some(gltf::mesh::util::ReadTexCoords::F32(tex_coords_iter)) => {
//                                tex_coords_iter
//                            }
//                            _ => panic!(),
//                        };
//
//                        let normal_iter = reader.read_normals().unwrap();
//                        let iter = izip!(vertex_iter, tex_coords_iter, normal_iter);
//
//                        // par_iter() は順序が維持されるが、par_bridge()は維持されない。
//                        // par_iter()を使うためには、IntoParallelIteratorを実装する必要がある。
//                        let vertices = iter
//                            .map(|vertex| ModelVertex {
//                                position: [vertex.0[0], vertex.0[1], vertex.0[2]],
//                                tex_coords: [vertex.1[0], vertex.1[1]],
//                                normal: [vertex.2[0], vertex.2[1], vertex.2[2]],
//                            })
//                            .collect::<Vec<_>>();
//                        let vertex_buffer =
//                            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//                                label: Some(&format!("{:?} Vertex Buffer", label_path)),
//                                contents: bytemuck::cast_slice(&vertices),
//                                usage: wgpu::BufferUsage::VERTEX,
//                            });
//                        let indices =
//                            if let Some(gltf::mesh::util::ReadIndices::U32(indices_iter)) =
//                                reader.read_indices()
//                            {
//                                indices_iter.collect::<Vec<_>>()
//                            } else {
//                                Vec::new()
//                            };
//
//                        let index_buffer =
//                            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//                                label: Some(&format!("{:?} Index Buffer", label_path)),
//                                contents: bytemuck::cast_slice(&indices),
//                                usage: wgpu::BufferUsage::INDEX,
//                            });
//
//                        Mesh {
//                            name: mesh.name().unwrap().to_string(),
//                            vertex_buffer,
//                            index_buffer,
//                            num_elements: primitive.indices().unwrap().count() as u32,
//                            material: primitive
//                                .material()
//                                .pbr_metallic_roughness()
//                                .base_color_texture()
//                                .unwrap()
//                                .texture()
//                                .index() as u32,
//                        }
//                    })
//                    .collect::<Vec<_>>()
//            })
//            .flatten()
//            .collect();
//
//        Ok(Self { meshes, materials })
//    }
//}

#[derive(Debug)]
pub struct Material {
    pub name: String,
    pub diffuse_texture: texture::Texture,
    pub normal_texture: texture::Texture,
    pub specular_texture: texture::Texture,
    pub id: u32,
    pub bind_group: wgpu::BindGroup,
    pub shader: Arc<shader::Shader>,
}

impl Material {
    pub fn new(
        device: &wgpu::Device,
        name: &str,
        diffuse_texture: texture::Texture,
        normal_texture: texture::Texture,
        specular_texture: texture::Texture,
        id: u32,
        layout: &wgpu::BindGroupLayout,
        shader: Arc<shader::Shader>,
    ) -> Self {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&normal_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&normal_texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&specular_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&specular_texture.sampler),
                },
            ],
            label: None,
        });

        Self {
            name: name.to_string(),
            diffuse_texture,
            normal_texture,
            specular_texture,
            bind_group,
            id,
            shader,
        }
    }
}

#[derive(Debug)]
pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: Arc<Material>,
}

pub trait DrawModel<'a, 'b>
where
    'b: 'a,
{
    fn draw_mesh(
        &mut self,
        mesh: &'b Mesh,
        material: &Option<&'b Material>,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    );
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        material: &Option<&'b Material>,
        instances: Range<u32>,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    );
    fn draw_model(
        &mut self,
        model: &'b Model,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    );
    fn draw_model_instanced(
        &mut self,
        model: &'b Model,
        instances: Range<u32>,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    );
}
impl<'a, 'b> DrawModel<'a, 'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh(
        &mut self,
        mesh: &'b Mesh,
        material: &Option<&'b Material>,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) {
        self.draw_mesh_instanced(mesh, material, 0..1, uniforms, light);
    }

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        material: &Option<&'b Material>,
        instances: Range<u32>,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        match material {
            Some(m) => {
                self.set_pipeline(&m.shader.render_pipeline);
                self.set_bind_group(0, &m.bind_group, &[]);
            }
            None => {
                todo!();
            }
        }
        self.set_bind_group(1, &uniforms, &[]);
        self.set_bind_group(2, &light, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }
    fn draw_model(
        &mut self,
        model: &'b Model,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) {
        self.draw_model_instanced(model, 0..1, uniforms, light);
    }

    fn draw_model_instanced(
        &mut self,
        model: &'b Model,
        instances: Range<u32>,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) {
        for mesh in model.meshes() {
            self.draw_mesh_instanced(
                mesh,
                &Some(&mesh.material),
                instances.clone(),
                uniforms,
                light,
            );
        }
    }
}

pub trait DrawLight<'a, 'b>
where
    'b: 'a,
{
    fn draw_light_mesh(
        &mut self,
        mesh: &'b Mesh,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    );
    fn draw_light_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        instances: Range<u32>,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) where
        'b: 'a;

    fn draw_light_model(
        &mut self,
        model: &'b Model,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    );
    fn draw_light_model_instanced(
        &mut self,
        model: &'b Model,
        instances: Range<u32>,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawLight<'a, 'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_light_mesh(
        &mut self,
        mesh: &'b Mesh,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) {
        self.draw_light_mesh_instanced(mesh, 0..1, uniforms, light);
    }

    fn draw_light_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        instances: Range<u32>,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, uniforms, &[]);
        self.set_bind_group(1, light, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }

    fn draw_light_model(
        &mut self,
        model: &'b Model,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) {
        self.draw_light_model_instanced(model, 0..1, uniforms, light);
    }
    fn draw_light_model_instanced(
        &mut self,
        model: &'b Model,
        instances: Range<u32>,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) {
        for mesh in model.meshes() {
            self.draw_light_mesh_instanced(mesh, instances.clone(), uniforms, light);
        }
    }
}
