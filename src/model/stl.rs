// STL file format
// bytes range | description
// ------------|----------------
// 0-79        | 80 byte header
// 80-83       | // 4 byte unsigned int (number of triangles)
// 84-end      | triangle data // INFO: (50 bytes per triangle)
// each triangle:
// ------------|----------------
// bytes range | description
// ------------|----------------
// 0-11        | normal vector (3 * 4 bytes, (x, y, z))
// 12-23       | vertex 1 (3 * 4 bytes, (x, y, z))
// 24-35       | vertex 2 (3 * 4 bytes, (x, y, z))
// 36-47       | vertex 3 (3 * 4 bytes, (x, y, z))
// 48-49       | attribute byte count (2 bytes) (usually zero; padding for alignment)

use crate::model::{Face, MAX_TRIANGLES, Mesh, MeshCodec, Vec3};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::{
    collections::HashMap,
    fs::File,
    io::{BufWriter, Cursor, Seek, SeekFrom, Write},
    path::Path,
};

pub struct StlCodec;

impl MeshCodec for StlCodec {
    fn parse(&self, bytes: &[u8]) -> anyhow::Result<Mesh> {
        if is_ascii(bytes) {
            parse_ascii(bytes)
        } else {
            parse_binary(bytes)
        }
    }

    fn write(&self, path: &Path, mesh: &Mesh) -> anyhow::Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        // write 80 byte header
        let mut header = [0u8; 80];
        let signature = b"created by mesh_rs";
        header[..signature.len()].copy_from_slice(signature);
        writer.write_all(&header)?;

        let mut triangle_count = 0;
        // STL only supports triangular faces
        for face in &mesh.faces {
            if face.v.len() >= 3 {
                triangle_count += (face.v.len() - 2) as u32;
            }
        }
        writer.write_u32::<LittleEndian>(triangle_count)?;

        for face in &mesh.faces {
            if face.v.len() < 3 {
                continue;
            }

            let v0_idx = face.v[0];
            let v0 = mesh.vertices[v0_idx];

            // Fan triangulation: Connect v0 to v(i) and v(i+1)
            for i in 1..(face.v.len() - 1) {
                let v1 = mesh.vertices[face.v[i]];
                let v2 = mesh.vertices[face.v[i + 1]];

                let a = v1.substraction(v0);
                let b = v2.substraction(v0);
                let normal = a.cross(b).normalize();

                // write normal
                writer.write_f32::<LittleEndian>(normal.0)?;
                writer.write_f32::<LittleEndian>(normal.1)?;
                writer.write_f32::<LittleEndian>(normal.2)?;

                // write vertices
                for vertex in &[v0, v1, v2] {
                    writer.write_f32::<LittleEndian>(vertex.0)?;
                    writer.write_f32::<LittleEndian>(vertex.1)?;
                    writer.write_f32::<LittleEndian>(vertex.2)?;
                }

                // write attribute byte count (2 bytes)
                writer.write_u16::<LittleEndian>(0)?;
            }
        }

        writer.flush()?;
        Ok(())
    }
}

pub fn validate_bytes(bytes: &[u8]) -> bool {
    if is_ascii(bytes) {
        return true;
    }

    // binary STL must have at least 80 byte header + 4 byte count
    if bytes.len() < 84 {
        return false;
    }

    let triangle_count = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]) as usize;
    let data_len = bytes.len() - 84;

    if triangle_count > MAX_TRIANGLES as usize {
        return false;
    }

    // if triangle count is zero, and there is data present, it's valid (common export bug)
    if triangle_count == 0 {
        return data_len >= 50;
    }

    // triangle is 50 bytes
    let expected_min_data = triangle_count * 50;
    data_len >= expected_min_data
}

fn is_ascii(bytes: &[u8]) -> bool {
    // if the file does not start with "solid", it is binary or invalid
    if !bytes.starts_with(b"solid") {
        return false;
    }

    // check the first 1KB for the "facet" keyword
    let check_len = bytes.len().min(1024);
    if let Ok(header) = std::str::from_utf8(&bytes[..check_len]) {
        header.contains("facet")
    } else {
        false
    }
}

fn parse_binary(bytes: &[u8]) -> anyhow::Result<Mesh> {
    if bytes.len() < 84 {
        return Err(anyhow::anyhow!("binary STL file too small"));
    }

    let mut cursor = Cursor::new(bytes);

    // Skip 80 byte header
    cursor.seek(SeekFrom::Start(80))?;

    let declared_count = cursor.read_u32::<LittleEndian>()? as usize;
    // the actual number of triangles that can be read from this file
    let data_len = bytes.len().saturating_sub(84);
    let physical_count = data_len / 50;

    let triangle_count = if declared_count == 0 || declared_count > physical_count {
        physical_count
    } else {
        declared_count
    };

    // seek to the beginning of the triangle data
    cursor.seek(SeekFrom::Start(84))?;

    let mut mesh = Mesh::default();
    // using Euler's characteristic, we can estimate the number of unique vertices
    // as roughly half the number of triangles for a well-formed mesh
    mesh.vertices.reserve(triangle_count / 2);
    mesh.faces.reserve(triangle_count);

    let mut map = HashMap::with_capacity(triangle_count / 2);

    for _ in 0..triangle_count {
        // skip normal vector (3 * 4 bytes, (x, y, z))
        // we can compute it ourselves if needed
        // in counter part, some exporters write really bad normals
        cursor.seek(SeekFrom::Current(12))?;

        let mut face = Face::default();

        for _ in 0..3 {
            let x = cursor.read_f32::<LittleEndian>()?;
            let y = cursor.read_f32::<LittleEndian>()?;
            let z = cursor.read_f32::<LittleEndian>()?;

            let key = (x.to_bits(), y.to_bits(), z.to_bits());

            let idx = *map.entry(key).or_insert_with(|| {
                let idx = mesh.vertices.len();
                mesh.vertices.push(Vec3(x, y, z));
                idx
            });
            face.v.push(idx);
        }

        // skip attribute byte count (2 bytes)
        cursor.seek(SeekFrom::Current(2))?;
        mesh.faces.push(face);
    }

    anyhow::Ok(mesh)
}

fn parse_ascii(bytes: &[u8]) -> anyhow::Result<Mesh> {
    let content = std::str::from_utf8(bytes)?;
    let mut mesh = Mesh::default();

    let mut map = HashMap::new();
    let mut face = Face::default();

    for line in content.lines() {
        let line = line.trim();

        if line.starts_with("vertex") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            // expected format: vertex x y z
            if parts.len() == 4
                && let (Ok(x), Ok(y), Ok(z)) = (
                    parts[1].parse::<f32>(),
                    parts[2].parse::<f32>(),
                    parts[3].parse::<f32>(),
                )
            {
                let key = (x.to_bits(), y.to_bits(), z.to_bits());

                let idx = *map.entry(key).or_insert_with(|| {
                    let idx = mesh.vertices.len();
                    mesh.vertices.push(Vec3(x, y, z));
                    idx
                });
                face.v.push(idx);
            }
        } else if (line.starts_with("endfacet") || line.starts_with("endloop"))
            && !face.v.is_empty()
        {
            mesh.faces.push(face);
            face = Face::default();
        }
    }

    anyhow::Ok(mesh)
}
