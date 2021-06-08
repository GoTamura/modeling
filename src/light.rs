use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightRaw {
    pub position: [f32; 3],
    pub _padding: u32,
    pub color: [f32; 3],
}

#[repr(C)]
#[derive(Debug)]
pub struct Light {
    pub light: LightRaw,
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl Light {
    pub fn new(device: &wgpu::Device, light: LightRaw) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light VB"),
            contents: bytemuck::cast_slice(&[light]),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
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
        let old_position: cgmath::Vector3<f32> = self.light.position.into();
        let rot: cgmath::Quaternion<f32> = cgmath::Rotation3::from_axis_angle(
            cgmath::Vector3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            cgmath::Deg(0.2),
        );

        let pos: cgmath::Vector3<f32> = rot * old_position;
        self.light.position = pos.into();

        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.light]));
    }
}
