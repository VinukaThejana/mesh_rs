// STL file format
// bytes range | description
// ------------|----------------
// 0-79        | 80 byte header
// 80-83       | // 4 byte unsigned int (number of triangles)
// 84-end      | triangle data // INFO: (50 bytes per triangle)

use crate::model::{MAX_TRIANGLES, MeshParser, Triangle, Vec3, indexed_mesh::IndexedMesh};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::{
    fs::File,
    io::{BufWriter, Cursor, Seek, SeekFrom, Write},
};

pub struct STlParser;

impl MeshParser for STlParser {
    fn parse(bytes: &[u8]) -> anyhow::Result<Vec<Triangle>, anyhow::Error> {
        if is_ascii(bytes) {
            parse_ascii(bytes)
        } else {
            parse_binary(bytes)
        }
    }

    fn write(path: &std::path::Path, triangles: &[Triangle]) -> anyhow::Result<(), anyhow::Error> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        // write 80 byte header
        let mut header = [0u8; 80];
        // add a simple signature to the header
        let signature = b"created by mesh_rs";
        header[..signature.len()].copy_from_slice(signature);
        writer.write_all(&header)?;

        // write the triangle count
        // 4 bytes, u32, little-endian
        let triangle_count = triangles.len();
        if triangle_count > u32::MAX as usize {
            return Err(anyhow::anyhow!("too many triangles to write to STL file"));
        }
        writer.write_u32::<LittleEndian>(triangle_count as u32)?;

        // write each triangle
        for triangle in triangles {
            let v0 = triangle.vertices[0];
            let v1 = triangle.vertices[1];
            let v2 = triangle.vertices[2];

            // edges, vectors from v0 to v1 and v0 to v2
            let edge1 = v1.substraction(v0);
            let edge2 = v2.substraction(v0);

            // cross product to get the normal vector
            // and converting it to a unit vector
            let normal = edge1.cross(edge2).normalize();

            // normal vector (3 * 4 bytes, (x, y, z))
            writer.write_f32::<LittleEndian>(normal.0)?; // normal vector x
            writer.write_f32::<LittleEndian>(normal.1)?; // normal vector y
            writer.write_f32::<LittleEndian>(normal.2)?; // normal vector z

            // vertices of the triangle
            for vertex in &triangle.vertices {
                writer.write_f32::<LittleEndian>(vertex.0)?; // vertex x
                writer.write_f32::<LittleEndian>(vertex.1)?; // vertex y
                writer.write_f32::<LittleEndian>(vertex.2)?; // vertex z
            }

            // attribute byte count (2 bytes)
            // we write zero for no attributes
            writer.write_u16::<LittleEndian>(0)?;
        }

        writer.flush()?;
        anyhow::Ok(())
    }
}

pub fn write_indexed_mesh(
    path: &std::path::Path,
    mesh: &IndexedMesh,
) -> anyhow::Result<(), anyhow::Error> {
    use byteorder::{LittleEndian, WriteBytesExt};
    use std::fs::File;
    use std::io::{BufWriter, Write};

    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    let mut header = [0u8; 80];
    let signature = b"created by mesh_rs";
    header[..signature.len()].copy_from_slice(signature);
    writer.write_all(&header)?;

    if mesh.faces.len() > u32::MAX as usize {
        return Err(anyhow::anyhow!("too many triangles to write to STL file"));
    }
    writer.write_u32::<LittleEndian>(mesh.faces.len() as u32)?;

    // Write each face directly from the indexed representation
    for face in &mesh.faces {
        // Get vertices from the index - these are the EXACT same float values
        let v0 = mesh.vertices[face[0]];
        let v1 = mesh.vertices[face[1]];
        let v2 = mesh.vertices[face[2]];

        // Calculate normal
        let edge1 = v1.substraction(v0);
        let edge2 = v2.substraction(v0);
        let normal = edge1.cross(edge2).normalize();

        // Write normal
        writer.write_f32::<LittleEndian>(normal.0)?;
        writer.write_f32::<LittleEndian>(normal.1)?;
        writer.write_f32::<LittleEndian>(normal.2)?;

        // Write vertices - CRITICAL: These are bit-exact for shared vertices!
        writer.write_f32::<LittleEndian>(v0.0)?;
        writer.write_f32::<LittleEndian>(v0.1)?;
        writer.write_f32::<LittleEndian>(v0.2)?;

        writer.write_f32::<LittleEndian>(v1.0)?;
        writer.write_f32::<LittleEndian>(v1.1)?;
        writer.write_f32::<LittleEndian>(v1.2)?;

        writer.write_f32::<LittleEndian>(v2.0)?;
        writer.write_f32::<LittleEndian>(v2.1)?;
        writer.write_f32::<LittleEndian>(v2.2)?;

        writer.write_u16::<LittleEndian>(0)?;
    }

    writer.flush()?;
    anyhow::Ok(())
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

fn parse_binary(bytes: &[u8]) -> anyhow::Result<Vec<Triangle>, anyhow::Error> {
    if bytes.len() < 84 {
        return Err(anyhow::anyhow!("binary STL file too small"));
    }

    let mut cursor = Cursor::new(bytes);

    // Skip 80 byte header
    cursor.seek(SeekFrom::Start(80))?;

    let declared_count = cursor.read_u32::<LittleEndian>()? as usize;
    // data length after header and count
    let data_len = bytes.len().saturating_sub(84);
    let physical_count = data_len / 50;

    let triangle_count = if declared_count == 0 || declared_count > physical_count {
        physical_count
    } else {
        declared_count
    };

    // position cursor at the start of triangle data
    cursor.seek(SeekFrom::Start(84))?;

    let mut triangles = Vec::with_capacity(triangle_count);

    for _ in 0..triangle_count {
        // skip normal vector (3 * 4 bytes, (x, y, z))
        cursor.seek(SeekFrom::Current(12))?;

        // vertices
        let v0 = Vec3(
            cursor.read_f32::<LittleEndian>()?,
            cursor.read_f32::<LittleEndian>()?,
            cursor.read_f32::<LittleEndian>()?,
        );
        let v1 = Vec3(
            cursor.read_f32::<LittleEndian>()?,
            cursor.read_f32::<LittleEndian>()?,
            cursor.read_f32::<LittleEndian>()?,
        );
        let v2 = Vec3(
            cursor.read_f32::<LittleEndian>()?,
            cursor.read_f32::<LittleEndian>()?,
            cursor.read_f32::<LittleEndian>()?,
        );

        // skip attribute byte count (2 bytes)
        cursor.seek(SeekFrom::Current(2))?;

        let triangle = Triangle {
            vertices: [v0, v1, v2],
        };

        triangles.push(triangle);
    }

    anyhow::Ok(triangles)
}

fn parse_ascii(bytes: &[u8]) -> anyhow::Result<Vec<Triangle>, anyhow::Error> {
    let content = std::str::from_utf8(bytes)?;

    let mut triangles = Vec::new();
    let mut current_vertices = Vec::with_capacity(3);

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
                current_vertices.push(Vec3(x, y, z));
            }
        } else if line.starts_with("endfacet") || line.starts_with("endloop") {
            if current_vertices.len() == 3 {
                triangles.push(Triangle {
                    vertices: [
                        current_vertices[0],
                        current_vertices[1],
                        current_vertices[2],
                    ],
                });
            }
            current_vertices.clear();
        }
    }

    anyhow::Ok(triangles)
}
