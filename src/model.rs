use crate::texture;
use anyhow::*;
use itertools::izip;
use rayon::prelude::*;
use std::ops::Range;
use std::path::Path;
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
            ],
        }
    }
}

//pub struct Renderer<'a> {
//    render_pass: &'a wgpu::RenderPass,
//}
//pub trait RendererExt {
//    fn draw();
//}
//
//impl RendererExt for Renderer {
//
//}

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

    pub fn materials(&self) -> &Vec<Material> {
        match self {
            Model::OBJ(m) => &m.materials,
            Model::GLTF(m) => &m.materials,
        }
    }
}
pub struct ObjModel {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

pub struct GltfModel {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

impl ObjModel {
    pub async fn load<P: AsRef<Path>>(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        path: P,
    ) -> Result<Self> {
        let (obj_models, obj_materials) = tobj::load_obj(path.as_ref(), &tobj::LoadOptions {
            triangulate: true,
            .. tobj::LoadOptions::default()

        })?;

        // We're assuming that the texture files are stored with the obj file
        let containing_folder = path.as_ref().parent().context("Directory has no parent")?;

        let mut materials = Vec::new();
        for (i, mat) in obj_materials.unwrap().into_iter().enumerate() {
            let diffuse_path = mat.diffuse_texture;
            let diffuse_texture =
                texture::Texture::load(device, queue, containing_folder.join(diffuse_path))
                    .unwrap_or_else(|_| {
                        texture::Texture::load(device, queue, containing_folder.join("logo.png"))
                            .unwrap()
                    });

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                    },
                ],
                label: None,
            });

            materials.push(Material {
                name: mat.name,
                diffuse_texture,
                bind_group,
                diffuse_texture_id: i as u32,
            });
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
                    tex_coords: [m.mesh.texcoords[i * 2], m.mesh.texcoords[i * 2 + 1]],
                    normal: [
                        m.mesh.normals[i * 3],
                        m.mesh.normals[i * 3 + 1],
                        m.mesh.normals[i * 3 + 2],
                    ],
                });
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
                material: m.mesh.material_id.unwrap_or(0) as u32,
            });
        }

        Ok(Self { meshes, materials })
    }
}

impl GltfModel {
    pub async fn load<P: AsRef<Path>>(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        path: P,
    ) -> Result<Self> {
        let (gltf, buffers, _) = tokio::task::block_in_place(|| gltf::import(path.as_ref()))?;

        let materials = gltf
            .materials()
            .flat_map(|material| {
                //let materials = gltf.materials().par_bridge().map(|material| {
                if let Some(base_color_texture) =
                    material.pbr_metallic_roughness().base_color_texture()
                {
                    let diffuse_texture =
                        texture::Texture::load_gltf(device, queue, &base_color_texture, &buffers)
                            .unwrap();

                    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                            },
                        ],
                        label: None,
                    });

                    Some(Material {
                        name: material.name().unwrap().to_string(),
                        diffuse_texture,
                        bind_group,
                        diffuse_texture_id: material
                            .pbr_metallic_roughness()
                            .base_color_texture()
                            .unwrap()
                            .texture()
                            .index() as u32,
                    })
                } else {
                    None
                }
            })
            .collect();

        let label_path = path.as_ref().to_str().map(|str| str.to_string());

        //let meshes = gltf.meshes().map(|mesh| {
        let meshes = gltf
            .meshes()
            .par_bridge()
            .map(|mesh| {
                println!("Mesh #{}", mesh.index());
                //mesh.primitives().map(|primitive| {
                mesh.primitives()
                    .par_bridge()
                    .map(|primitive| {
                        println!("- Primitive #{}", primitive.index());
                        let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
                        let vertex_iter = reader.read_positions().unwrap();

                        let tex_coord = primitive
                            .material()
                            .pbr_metallic_roughness()
                            .base_color_texture()
                            .unwrap()
                            .tex_coord();
                        let tex_coords_iter = match reader.read_tex_coords(tex_coord) {
                            Some(gltf::mesh::util::ReadTexCoords::F32(tex_coords_iter)) => {
                                tex_coords_iter
                            }
                            _ => panic!(),
                        };

                        let normal_iter = reader.read_normals().unwrap();
                        let iter = izip!(vertex_iter, tex_coords_iter, normal_iter);

                        // par_iter() は順序が維持されるが、par_bridge()は維持されない。
                        // par_iter()を使うためには、IntoParallelIteratorを実装する必要がある。
                        let vertices = iter
                            .map(|vertex| ModelVertex {
                                position: [vertex.0[0], vertex.0[1], vertex.0[2]],
                                tex_coords: [vertex.1[0], vertex.1[1]],
                                normal: [vertex.2[0], vertex.2[1], vertex.2[2]],
                            })
                            .collect::<Vec<_>>();
                        let vertex_buffer =
                            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: Some(&format!("{:?} Vertex Buffer", label_path)),
                                contents: bytemuck::cast_slice(&vertices),
                                usage: wgpu::BufferUsage::VERTEX,
                            });
                        let indices =
                            if let Some(gltf::mesh::util::ReadIndices::U32(indices_iter)) =
                                reader.read_indices()
                            {
                                indices_iter.collect::<Vec<_>>()
                            } else {
                                Vec::new()
                            };

                        let index_buffer =
                            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: Some(&format!("{:?} Index Buffer", label_path)),
                                contents: bytemuck::cast_slice(&indices),
                                usage: wgpu::BufferUsage::INDEX,
                            });

                        Mesh {
                            name: mesh.name().unwrap().to_string(),
                            vertex_buffer,
                            index_buffer,
                            num_elements: primitive.indices().unwrap().count() as u32,
                            material: primitive
                                .material()
                                .pbr_metallic_roughness()
                                .base_color_texture()
                                .unwrap()
                                .texture()
                                .index() as u32,
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .flatten()
            .collect();

        Ok(Self { meshes, materials })
    }
}

pub struct Material {
    pub name: String,
    pub diffuse_texture: texture::Texture,
    pub diffuse_texture_id: u32,
    pub bind_group: wgpu::BindGroup,
}

pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: u32,
}

pub trait DrawModel<'a, 'b>
where
    'b: 'a,
{
    fn draw_mesh(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    );
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
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
        material: &'b Material,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) {
        self.draw_mesh_instanced(mesh, material, 0..1, uniforms, light);
    }

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        instances: Range<u32>,
        uniforms: &'b wgpu::BindGroup,
        light: &'b wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, &material.bind_group, &[]);
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
            let material = &model
                .materials()
                .iter()
                .find(|material| material.diffuse_texture_id == mesh.material)
                .unwrap();
            self.draw_mesh_instanced(mesh, material, instances.clone(), uniforms, light);
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
