use std::{fs::File, io::Read, path::Path, path::PathBuf};

use crate::{model::{self, ModelVertex, Vertex}, texture};

#[derive(Debug)]
pub struct Shader {
    label: String,
    filename: PathBuf,
    modules: Vec<wgpu::ShaderModule>,
    pub render_pipeline: wgpu::RenderPipeline,
}

pub trait Pass {
    fn pipeline(&self) -> &wgpu::RenderPipeline;
    fn bind_group(&self) -> &wgpu::BindGroup;
    fn uniform_buf(&self) -> &wgpu::Buffer;
}
pub struct ShadowPass {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buf: wgpu::Buffer,
}

impl Pass for ShadowPass {
    fn pipeline(&self) -> &wgpu::RenderPipeline {
        &self.pipeline
    }

    fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    fn uniform_buf(&self) -> &wgpu::Buffer {
        &self.uniform_buf
    }
}


impl Shader {
    pub fn new(
        label: impl Into<String>,
        filename: impl Into<PathBuf>,
        device: &wgpu::Device,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        light_bind_group_layout: &wgpu::BindGroupLayout,
        uniforms_bind_group_layout: &wgpu::BindGroupLayout,
        texture_format: &wgpu::TextureFormat,
    ) -> Self {
        let label = label.into();
        let filename = filename.into();
        let mut vert_name = filename.clone();
        vert_name.set_extension("vert.spv");
        let mut frag_name = filename.clone();
        frag_name.set_extension("frag.spv");
        let vs_module = Self::compile_shader(&label, &vert_name, device);
        let fs_module = Self::compile_shader(&label, &frag_name, device);
        let render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    texture_bind_group_layout,
                    light_bind_group_layout,
                    uniforms_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
            Self::create_render_pipeline2(
                &device,
                &layout,
                *texture_format,
                Some(texture::Texture::DEPTH_FORMAT),
                &[model::ModelVertex::desc()],
                &vs_module,
                &fs_module,
            )
        };

        let modules = vec![vs_module, fs_module];

        Self {
            label,
            filename,
            modules,
            render_pipeline,
        }
    }
    pub fn compile_shader(label: &str, path: &Path, device: &wgpu::Device) -> wgpu::ShaderModule {
        let mut f = File::open(path).unwrap();
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer);

        let shader = wgpu::ShaderModuleDescriptor {
            label: Some(label),
            source: wgpu::util::make_spirv(&buffer),
        };
        // let shader = wgpu::ShaderModuleDescriptor {
        // label: Some(label),
        // source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        // flags: wgpu::ShaderFlags::all()
        // };
        device.create_shader_module(&shader)
    }
    fn create_render_pipeline(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        color_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
        vertex_layouts: &[wgpu::VertexBufferLayout],
        shader: &wgpu::ShaderModule,
        //vs_module: &wgpu::ShaderModule,
        //fs_module: &wgpu::ShaderModule,
    ) -> wgpu::RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main", // 1.
                buffers: vertex_layouts,
            },
            fragment: Some(wgpu::FragmentState {
                // 2.
                module: &shader,
                entry_point: "fs_main",
                targets: &[wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),

            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                //..Default::default()
                strip_index_format: None,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                clamp_depth: device.features().contains(wgpu::Features::DEPTH_CLAMPING),
                conservative: false,
            },

            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format.unwrap_or_else(|| texture::Texture::DEPTH_FORMAT),
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less, // 1.
                stencil: wgpu::StencilState::default(),     // 2.
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            // {
            //    count: 1,
            //    mask: !0,
            //    alpha_to_coverage_enabled: false,
            //},
        })
    }

    fn create_render_pipeline2(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        color_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
        vertex_layouts: &[wgpu::VertexBufferLayout],
        //shader: &wgpu::ShaderModule,
        vs_module: &wgpu::ShaderModule,
        fs_module: &wgpu::ShaderModule,
    ) -> wgpu::RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &vs_module,
                entry_point: "main", // 1.
                buffers: vertex_layouts,
            },
            fragment: Some(wgpu::FragmentState {
                // 2.
                module: &fs_module,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),

            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                ..Default::default()
                //strip_index_format: None,
                //cull_mode: Some(wgpu::Face::Back),
                //polygon_mode: wgpu::PolygonMode::Fill,
                //clamp_depth: device.features().contains(wgpu::Features::DEPTH_CLAMPING),
                //conservative: false,
            },

            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format.unwrap_or_else(|| texture::Texture::DEPTH_FORMAT),
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less, // 1.
                stencil: wgpu::StencilState::default(),     // 2.
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            // {
            //    count: 1,
            //    mask: !0,
            //    alpha_to_coverage_enabled: false,
            //},
        })
    }

    fn create_box_render_pipeline(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        color_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
        vertex_layouts: &[wgpu::VertexBufferLayout],
        //shader: &wgpu::ShaderModuleDescriptor,
        vertex_shader: &wgpu::ShaderModuleDescriptor,
        fragent_shader: &wgpu::ShaderModuleDescriptor,
    ) -> wgpu::RenderPipeline {
        let vs_module = device.create_shader_module(vertex_shader);
        let fs_module = device.create_shader_module(fragent_shader);
        //let shader_module = device.create_shader_module(&shader);

        Self::create_render_pipeline2(
            &device,
            &layout,
            color_format,
            depth_format,
            vertex_layouts,
            //&shader_module,
            &vs_module,
            &fs_module,
        )
    }
}
