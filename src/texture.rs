use anyhow::*;
use image::GenericImageView;
use image::ImageFormat::{Jpeg, Png};

use std::path::Path;

#[derive(Debug)]
pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub id: u32,
    pub tex_coord: u32,
}

impl Texture {
    pub fn one_pixel(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: Option<&str>,
        is_normal_map: bool,
    ) -> Self {
        let rgba = bytes;
        let dimensions = (1, 1);

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: if is_normal_map {
                wgpu::TextureFormat::Rgba8Unorm
            } else {
                wgpu::TextureFormat::Rgba8UnormSrgb
            },
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        });
        queue.write_texture(
            // Tells wgpu where to copy the pixel data
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            // The actual pixel data
            &rgba,
            // The layout of the texture
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(std::num::NonZeroU32::new(4 * dimensions.0).unwrap()),
                rows_per_image: Some(std::num::NonZeroU32::new(dimensions.1).unwrap()),
            },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
            id: 0,
            tex_coord: 0,
        }
    }

    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: &str,
        is_normal_map: bool,
    ) -> Result<Self> {
        let img = image::load_from_memory(bytes)?;
        Self::from_image(device, queue, &img, Some(label), is_normal_map)
    }

    pub fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &image::DynamicImage,
        label: Option<&str>,
        is_normal_map: bool,
    ) -> Result<Self> {
        let rgba = img.to_rgba8();
        let dimensions = img.dimensions();

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: if is_normal_map {
                wgpu::TextureFormat::Rgba8Unorm
            } else {
                wgpu::TextureFormat::Rgba8UnormSrgb
            },
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        });
        queue.write_texture(
            // Tells wgpu where to copy the pixel data
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            // The actual pixel data
            &rgba,
            // The layout of the texture
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(std::num::NonZeroU32::new(4 * dimensions.0).unwrap()),
                rows_per_image: Some(std::num::NonZeroU32::new(dimensions.1).unwrap()),
            },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler,
            id: 0,
            tex_coord: 0,
        })
    }
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float; // 1.

    pub fn create_depth_texture(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        label: &str,
    ) -> Self {
        let size = wgpu::Extent3d {
            // 2.
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT // 3.
                | wgpu::TextureUsages::TEXTURE_BINDING,
        };
        let texture = device.create_texture(&desc);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            // 4.
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

        Self {
            texture,
            view,
            sampler,
            id: 1000000,
            tex_coord: 0,
        }
    }
    pub fn load<P: AsRef<Path>>(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: P,
        is_normal_map: bool,
    ) -> Result<Self> {
        let path_copy = path.as_ref().to_path_buf();
        let label = path_copy.to_str();

        let img = image::open(path)?;
        Self::from_image(device, queue, &img, label, is_normal_map)
    }

    pub fn load_house<P: AsRef<Path>>(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: P,
        is_normal_map: bool,
    ) -> Result<Self> {
        let alpha_bytes = include_bytes!("model/rungholt/house-Alpha.png");
        let rgb_bytes = include_bytes!("model/rungholt/house-RGB.png");
        let rgba_bytes = include_bytes!("model/rungholt/house-RGBA.png");

        let path_copy = path.as_ref().to_path_buf();
        let label = path_copy.to_str();


        let img = match path_copy.file_name().unwrap().to_str().unwrap() {
            "house-Alpha.png" => {
                image::load_from_memory_with_format(alpha_bytes, image::ImageFormat::Png)?
            }
            "house-RGB.png" => {
                image::load_from_memory_with_format(rgb_bytes, image::ImageFormat::Png)?
            }
            "house-RGBA.png" => {
                image::load_from_memory_with_format(rgba_bytes, image::ImageFormat::Png)?
            }
            _ => unreachable!(),
        };
        Self::from_image(device, queue, &img, label, is_normal_map)
    }

    pub fn load_gltf(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_info: &gltf::texture::Info,
        buffers: &Vec<gltf::buffer::Data>,
    ) -> Result<Self> {
        let texture = texture_info.texture();
        let id = texture.index() as u32;
        let source = texture.source();
        let sampler = texture.sampler();
        let (img, label) = match source.source() {
            gltf::image::Source::View { view, mime_type } => {
                let buffer = &buffers[view.buffer().index()].0;
                let data = &buffer[view.offset()..view.offset() + view.length()];
                let label = view.name().map(|str| str.to_string());

                let img = match mime_type {
                    "image/jpeq" => image::load_from_memory_with_format(data, Jpeg),
                    "image/png" => image::load_from_memory_with_format(data, Png),
                    _ => panic!("mimetype"),
                };
                (img, label)
            }
            gltf::image::Source::Uri {
                ref uri,
                mime_type: _,
            } => {
                let uri_dir = std::path::Path::new(env!("OUT_DIR")).join("res").join(uri);
                let label = uri_dir.to_str().map(|str| str.to_string());

                let img = image::open(uri_dir);
                (img, label)
            }
        };
        let img = img.unwrap();

        let rgba = img.to_rgba8();
        let dimensions = img.dimensions();

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: label.as_deref(),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        });
        queue.write_texture(
            // Tells wgpu where to copy the pixel data
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            // The actual pixel data
            &rgba,
            // The layout of the texture
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: std::num::NonZeroU32::new(4 * dimensions.0),
                rows_per_image: std::num::NonZeroU32::new(dimensions.1),
            },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mag_filter = match sampler.mag_filter().unwrap() {
            gltf::texture::MagFilter::Linear => wgpu::FilterMode::Linear,
            gltf::texture::MagFilter::Nearest => wgpu::FilterMode::Nearest,
        };

        let min_filter = match sampler.min_filter().unwrap() {
            gltf::texture::MinFilter::Linear => wgpu::FilterMode::Linear,
            gltf::texture::MinFilter::Nearest => wgpu::FilterMode::Nearest,
            _ => wgpu::FilterMode::Nearest,
        };
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter,
            min_filter,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler,
            id,
            tex_coord: texture_info.tex_coord(),
        })
    }
}
