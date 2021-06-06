use std::{fs::File, io::Read, path::Path, path::PathBuf};

#[derive(Debug)]
pub struct Shader {
    label: String,
    filename: PathBuf,
    module: wgpu::ShaderModule,
}

impl Shader {
    pub fn new(
        label: impl Into<String>,
        filename: impl Into<PathBuf>,
        device: &wgpu::Device,
    ) -> Self {
        let label = label.into();
        let filename = filename.into();
        let module = Self::compile_shader(&label, &filename, device);
        Self {
            label,
            filename,
            module,
        }
    }
    pub fn compile_shader(label: &str, path: &Path, device: &wgpu::Device) -> wgpu::ShaderModule {
        let mut f = File::open(path).unwrap();
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer);

        let shader = wgpu::ShaderModuleDescriptor {
            label: Some(label),
            source: wgpu::util::make_spirv(&buffer),
            flags: wgpu::ShaderFlags::VALIDATION,
        };
        // let shader = wgpu::ShaderModuleDescriptor {
        // label: Some(label),
        // source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        // flags: wgpu::ShaderFlags::all()
        // };
        device.create_shader_module(&shader)
    }
}
