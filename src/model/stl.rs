// STL file format
// bytes range | description
// ------------|----------------
// 0-79        | 80 byte header
// 80-83       | // 4 byte unsigned int (number of triangles)
// 84-end      | triangle data // INFO: (50 bytes per triangle)

use byteorder::{LittleEndian, ReadBytesExt};

use crate::model::{MAX_TRIANGLES, MeshParser, Triangle, Vec3};
use std::io::{Cursor, Seek, SeekFrom};

pub struct STlParser;

impl MeshParser for STlParser {
    fn parse(bytes: &[u8]) -> anyhow::Result<Vec<Triangle>, anyhow::Error> {
        if is_ascii(bytes) {
            parse_ascii(bytes)
        } else {
            parse_binary(bytes)
        }
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

    let triangle_count = if declared_count == 0 {
        physical_count
    } else if declared_count > physical_count {
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

        if !triangle.is_valid() {
            continue;
        }
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
