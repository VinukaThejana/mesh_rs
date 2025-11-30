use std::{fs::OpenOptions, io::Read, path::Path};

use object_resize::{
    calculate,
    model::{self, MeshParser},
};

fn main() -> anyhow::Result<(), anyhow::Error> {
    let input_file = "input.stl";
    let output_file = "output.stl";
    let target_unit = "mm";

    let mut file = OpenOptions::new().read(true).open(input_file)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    let format = model::Format::from_magic_bytes(&buffer)
        .or_else(|| {
            let path = Path::new(input_file);
            let ext = path.extension()?.to_str()?;
            match ext.to_lowercase().as_str() {
                "stl" => Some(model::Format::STL),
                "obj" => Some(model::Format::OBJ),
                _ => None,
            }
        })
        .ok_or_else(|| anyhow::anyhow!("unsupported file format"))?;

    println!("Detected format: {:?}", format);

    if !format.validate_bytes(&buffer) {
        return Err(anyhow::anyhow!("file validation failed"));
    }

    let mut triangles = match format {
        model::Format::STL => model::stl::STlParser::parse(&buffer)?,
        model::Format::OBJ => model::obj::OBJParser::parse(&buffer)?,
    };
    println!("Parsed {} triangles", triangles.len());

    let raw_volume = calculate::volume(&triangles);
    println!("Raw volume: {} cubic mm", raw_volume);

    calculate::scale(&mut triangles, 15 as f32)?;

    let raw_volume_after_scaling = calculate::volume(&triangles);
    println!(
        "Volume after scaling: {} cubic mm",
        raw_volume_after_scaling
    );

    anyhow::Ok(())
}
