use std::{fs::OpenOptions, io::Read, path::PathBuf};

use mesh_rs::{
    calculate,
    model::{self, MeshCodec, obj::ObjCodec, stl::StlCodec},
    ui,
    util::{warn_topology, warn_units},
};

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "Mesh tool")]
#[command(
    about = "A CLI for analyzing and scaling 3D meshes",
    long_about = "A versatile command-line tool for analyzing and manipulating 3D mesh files.

Supported Formats:
- STL (Binary and ASCII)
- OBJ (Wavefront)

Examples:
  # Get volume of a mesh
  object_resize model.stl volume

  # Scale a mesh to 100mm diagonal
  object_resize input.obj scale 100 -o output.obj"
)]
struct Cli {
    /// The input file path (e.g., model.stl, model.obj, etc.)
    ///
    /// The tool automatically detects the file format based on the content or extension.
    input: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get the current diagonal of the mesh's bounding box
    ///
    /// This calculates the distance between the minimum and maximum corners of the axis-aligned bounding box.
    Diagonal,

    /// Get the volume of the mesh
    ///
    /// Calculates the signed volume of the mesh. Assumes the mesh is watertight and manifold.
    /// The unit is cubic units based on the input file's units (usually mm^3).
    Volume,

    /// Get the triangle count of the mesh
    ///
    /// Returns the total number of triangular faces in the mesh.
    Triangles,

    /// Get comprehensive statistics (volume, diagonal, and triangle count)
    Stats,

    /// Scale the mesh to a target diagonal length
    ///
    /// Uniformly scales the mesh so that its bounding box diagonal equals the target length.
    /// This is useful for normalizing the size of objects for 3D printing or rendering.
    Scale {
        /// The target diagonal length in the same units as the input file
        target_diagonal: f32,

        /// Optional output file path
        ///
        /// If not provided, the output will be saved as <input_stem>_scaled.<ext>
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    if !cli.input.exists() {
        ui::print_error(&format!("Input file does not exist: {:?}", cli.input));
        std::process::exit(1);
    }

    let mut file = OpenOptions::new().read(true).open(&cli.input)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    let format = model::Format::from_magic_bytes(&buffer)
        .or_else(|| {
            let ext = cli.input.extension()?.to_str()?;
            match ext.to_lowercase().as_str() {
                "stl" => Some(model::Format::STL),
                "obj" => Some(model::Format::OBJ),
                _ => None,
            }
        })
        .ok_or_else(|| anyhow::anyhow!("unsupported file format"))?;

    let mut mesh = match format {
        model::Format::STL => StlCodec.parse(&buffer)?,
        model::Format::OBJ => ObjCodec.parse(&buffer)?,
    };
    mesh.weld();

    match cli.command {
        Commands::Diagonal => {
            let diagonal = calculate::diagonal(&mesh)?;
            ui::print_kv("Diagonal", format!("{:.4}", diagonal));
        }
        Commands::Volume => {
            let volume = calculate::volume(&mesh);
            ui::print_kv("Volume", format!("{:.4}", volume));
        }
        Commands::Triangles => {
            let triangles = mesh.triangle_count();
            ui::print_success(&format!("Parsed {} triangles", triangles));
        }
        Commands::Stats => {
            let diagonal = calculate::diagonal(&mesh)?;
            let volume = calculate::volume(&mesh);
            let triangles = mesh.triangle_count();

            ui::print_section("Statistics");
            ui::print_kv("File", cli.input.display());
            ui::print_kv("Format", format!("{:?}", format));
            ui::print_kv("Triangles", triangles);
            ui::print_kv("Diagonal", format!("{:.4}", diagonal));
            ui::print_kv("Volume", format!("{:.4}", volume));

            warn_topology(&mesh);
            warn_units(cli.input.to_str().unwrap(), volume, diagonal);
        }
        Commands::Scale {
            target_diagonal,
            output,
        } => {
            let diagonal = calculate::diagonal(&mesh)?;
            ui::print_info(
                "Scaling",
                &format!("{:.4} -> {:.4}", diagonal, target_diagonal),
            );

            calculate::scale(&mut mesh, target_diagonal)?;

            let output_path = match output {
                Some(p) => p,
                None => {
                    let stem = cli
                        .input
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("output");
                    let ext = cli
                        .input
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("stl");
                    cli.input.with_file_name(format!("{}_scaled.{}", stem, ext))
                }
            };

            ui::print_success("Scaled model processed.");
            ui::print_info("Saving to", &format!("{:?}", output_path));

            match format {
                model::Format::STL => StlCodec.write(&output_path, &mesh)?,
                model::Format::OBJ => ObjCodec.write(&output_path, &mesh)?,
            }

            ui::print_success("File saved successfully.");
        }
    }

    anyhow::Ok(())
}
