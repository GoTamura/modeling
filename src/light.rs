use std::{mem, num::NonZeroU32, ops::Range};

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightRaw {
    pub projection: [[f32; 4]; 4],
    pub position: [f32; 4],
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Debug)]
pub struct Light {
    pub position: cgmath::Point3<f32>,
    pub color: cgmath::Vector3<f32>,
    pub fov: cgmath::Rad<f32>,
    pub depth: Range<f32>,
    pub shadow_view: Option<wgpu::TextureView>,
}

impl Light {
    pub fn to_raw(&self) -> LightRaw {
        use crate::camera::PerspectiveFovExt;
        use cgmath::{Deg, EuclideanSpace, Matrix4, PerspectiveFov, Point3, Vector3};

        let view_matrix = Matrix4::look_at_rh(self.position, Point3::origin(), Vector3::unit_z());
        let projection = PerspectiveFov {
            fovy: self.fov,
            aspect: 1.0,
            near: self.depth.start,
            far: self.depth.end,
        };
        let view_proj = projection.calc_matrix() * view_matrix;
        LightRaw {
            projection: *view_proj.as_ref(),
            position: [self.position.x, self.position.y, self.position.z, 1.0],
            color: [
                self.color.x as f32,
                self.color.y as f32,
                self.color.z as f32,
                1.0,
            ],
        }
    }

    pub fn new<F: Into<cgmath::Rad<f32>>>(
        position: cgmath::Point3<f32>,
        color: cgmath::Vector3<f32>,
        fov: F,
        depth: Range<f32>,
    ) -> Self {
        Self {
            position,
            color,
            fov: fov.into(),
            depth,
            shadow_view: None,
        }
    }
}

#[derive(Debug)]
pub struct LightObject {
    pub light: Light,
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl LightObject {
    pub fn new(device: &wgpu::Device, light: Light) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light VB"),
            contents: bytemuck::cast_slice(&[light.to_raw()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: None,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: None,
        });

        Self {
            light,
            buffer,
            bind_group,
            bind_group_layout,
        }
    }
    pub fn update(&mut self, queue: &wgpu::Queue) {
        use cgmath::EuclideanSpace;
        let old_position: cgmath::Vector3<f32> = self.light.position.to_vec();
        let rot: cgmath::Quaternion<f32> = cgmath::Rotation3::from_axis_angle(
            cgmath::Vector3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            cgmath::Deg(0.2),
        );

        let pos: cgmath::Vector3<f32> = rot * old_position;
        self.light.position = cgmath::Point3::new(0., 0., 0.) + pos;

        queue.write_buffer(
            &self.buffer,
            0,
            bytemuck::cast_slice(&[self.light.to_raw()]),
        );
    }
}

#[derive(Debug)]
pub struct Lights {
    pub lights: Vec<LightObject>,
    pub shadow_texture: wgpu::Texture,
    pub shadow_view: wgpu::TextureView,
    pub light_storage_buf: wgpu::Buffer,
}

impl Lights {
    pub const SHADOW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float; // 1.
    pub const SHADOW_SIZE: wgpu::Extent3d = wgpu::Extent3d {
        width: 1,
        height: 1,
        depth_or_array_layers: 1,
    };

    pub fn new(device: &wgpu::Device, lights: Vec<LightObject>) -> Self {
        let shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: Self::SHADOW_SIZE,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::SHADOW_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT // 3.
                | wgpu::TextureUsages::TEXTURE_BINDING,
        });

        let shadow_view = shadow_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual), // 5.
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        let mut lights = lights;
        lights.iter_mut().enumerate().for_each(|(i, lo)| {
            lo.light.shadow_view = Some(shadow_texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("shadow"),
                format: None,
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: None,
                base_array_layer: i as u32,
                array_layer_count: NonZeroU32::new(1),
            }))
        });

        const MAX_LIGHTS: usize = 2;
        let light_uniform_size =
            (MAX_LIGHTS * mem::size_of::<LightRaw>()) as wgpu::BufferAddress;
        let light_storage_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: light_uniform_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });


        Self {
            lights,
            shadow_texture,
            shadow_view,
            light_storage_buf,
        }
    }
}
