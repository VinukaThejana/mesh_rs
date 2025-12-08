# mesh_rs

**mesh_rs** is a powerful and versatile command-line interface (CLI) tool designed for analyzing and manipulating 3D mesh files. Built with Rust, it offers high-performance processing for 3D printing enthusiasts and professionals who need to inspect or modify their model files directly from the terminal.

## Features

- **Format Detection**: Automatically detects file formats based on content or extension.
- **Geometric Analysis**:
  - **Diagonal**: Calculate the bounding box diagonal to understand the scale of the object.
  - **Volume**: Compute the signed volume of the mesh (assumes watertight/manifold meshes).
  - **Triangle Count**: Quickly get the total number of faces in the model.
  - **Comprehensive Stats**: View all metrics (volume, diagonal, triangle count) in a single summary.
- **Mesh Manipulation**:
  - **Scaling**: Uniformly scale meshes to a specific target diagonal length. Useful for normalizing object sizes for printing.

## Supported Formats

Currently, `mesh_rs` supports the following 3D file formats:

- **STL** (Stereolithography) - Binary and ASCII
- **OBJ** (Wavefront)

## Installation

Ensure you have Rust and Cargo installed on your system.

```bash
# Clone the repository
git clone git@github.com:VinukaThejana/mesh_rs.git 

# Navigate to the project directory
cd mesh_rs

# Build the project
cargo build --release

# (Optional) Install globally
cargo install --path .
```

## Usage

The general syntax is:
```bash
mesh_rs <INPUT_FILE> <COMMAND> [ARGS]
```

### Commands

#### 1. Get Mesh Statistics
View comprehensive details including file format, triangle count, diagonal size, and volume.

```bash
mesh_rs model.stl stats
```

#### 2. Calculate Volume
Get the volume of the mesh in cubic units (usually mm³).

```bash
mesh_rs model.obj volume
```

#### 3. Get Bounding Box Diagonal
Measure the diagonal length of the mesh's bounding box.

```bash
mesh_rs input.stl diagonal
```

#### 4. Count Triangles
Get the total number of triangular faces.

```bash
mesh_rs input.obj triangles
```

#### 5. Scale a Mesh
Resize a mesh so its bounding box diagonal matches a specific length.

```bash
# Scale 'input.stl' to have a diagonal of 100 units
mesh_rs input.stl scale 100

# Scale and save to a specific output file
mesh_rs input.obj scale 150 --output scaled_model.obj
```

## Roadmap & Future Goals

We aim to make `mesh_rs` the go-to CLI for 3D model analysis. Future plans include:

- [ ] **More Formats**: Support for PLY, 3MF, and FBX.
- [ ] **Mesh Repair**: Basic repair tools for non-manifold edges and holes.
- [ ] **Slicing Preview**: Generate simple cross-section previews in the terminal.
- [ ] **Batch Processing**: Analyze or convert multiple files in a directory at once.
- [ ] **Metadata Inspection**: Read and display metadata from supported formats.

## License

[MIT License](LICENSE)
