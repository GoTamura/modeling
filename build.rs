use anyhow::*;
use glob::glob;
use rayon::prelude::*;
use std::fs::{read_to_string, write};
use std::path::Path;
use std::path::PathBuf;

use fs_extra::{copy_items, dir::CopyOptions};
use std::env;

struct ShaderData {
    src: String,
    src_path: PathBuf,
    spv_path: PathBuf,
    kind: shaderc::ShaderKind,
}

impl ShaderData {
    pub fn load(src_path: PathBuf) -> Result<Self> {
        let extension = src_path
            .extension()
            .context("File has no extension")?
            .to_str()
            .context("Extension cannot be converted to &str")?;
        let kind = match extension {
            "vert" => shaderc::ShaderKind::Vertex,
            "frag" => shaderc::ShaderKind::Fragment,
            "comp" => shaderc::ShaderKind::Compute,
            _ => bail!("Unsupported shader: {}", src_path.display()),
        };

        let src = read_to_string(src_path.clone())?;
        let spv_path = src_path.with_extension(format!("{}.spv", extension));

        Ok(Self {
            src,
            src_path,
            spv_path,
            kind,
        })
    }
}

fn main() -> Result<()> {
    // Collect all shaders recursively within /src/
    let mut shader_paths = Vec::new();
    shader_paths.extend(glob("./src/**/*.vert")?);
    shader_paths.extend(glob("./src/**/*.frag")?);
    shader_paths.extend(glob("./src/**/*.comp")?);

    // This could be parallelized
    let shaders = shader_paths
        .into_par_iter()
        .map(|glob_result| ShaderData::load(glob_result?))
        .collect::<Vec<Result<_>>>()
        .into_iter()
        .collect::<Result<Vec<_>>>()?;

    let mut compiler = shaderc::Compiler::new().context("Unable to create shader compiler")?;
    let mut options = shaderc::CompileOptions::new().unwrap();
    options.set_include_callback(
        |name, include_type, source_name, depth| match include_type {
            shaderc::IncludeType::Relative => {
                let path = std::path::Path::new(source_name);
                Ok(shaderc::ResolvedInclude {
                    resolved_name: name.to_string(),
                    content: std::fs::read_to_string(path.parent().unwrap().join(name)).unwrap(),
                })
            }
            shaderc::IncludeType::Standard => {
                let path = std::path::Path::new("src/shaders");
                Ok(shaderc::ResolvedInclude {
                    resolved_name: name.to_string(),
                    content: std::fs::read_to_string(path.parent().unwrap().join(name)).unwrap(),
                })
            }
        },
    );

    // This can't be parallelized. The [shaderc::Compiler] is not
    // thread safe. Also, it creates a lot of resources. You could
    // spawn multiple processes to handle this, but it would probably
    // be better just to only compile shaders that have been changed
    // recently.
    for shader in shaders {
        // This tells cargo to rerun this script if something in /src/ changes.
        println!("cargo:rerun-if-changed={:?}", shader.src_path);
        let compiled = compiler.compile_into_spirv(
            &shader.src,
            shader.kind,
            &shader.src_path.to_str().unwrap(),
            "main",
            Some(&options),
        )?;
        write(&shader.spv_path, compiled.as_binary_u8())?;
        write(
            Path::new(&env::var("OUT_DIR").unwrap()).join(shader.spv_path.file_name().unwrap()),
            compiled.as_binary_u8(),
        )?;
    }

    println!("cargo:rerun-if-changed=res/*");

    let out_dir = env::var("OUT_DIR")?;
    let mut copy_options = CopyOptions::new();
    copy_options.overwrite = true;
    let mut paths_to_copy = Vec::new();
    paths_to_copy.push("res/");
    copy_items(&paths_to_copy, out_dir, &copy_options)?;

    Ok(())
}
