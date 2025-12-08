// file format
// OBJ files are ASCII text files with the following line types:
// v x y z          | vertex position
// vt u v           | texture coordinate
// vn x y z         | vertex normal
// f v1 v2 v3       | face (triangle) - can reference v/vt/vn indices (starts at 1)
// f v1/vt1 v2/vt2 v3/vt3           | face with texture coords
// f v1/vt1/vn1 v2/vt2/vn2 v3/vt3/vn3 | face with texture and normals
// f v1//vn1 v2//vn2 v3//vn3        | face with normals only
// # comment        | comment line
// o name           | object name
// g name           | group name
// mtllib file      | material library
// usemtl name      | use material
use crate::{
    calculate::triangulation::triangulate,
    model::{MeshParser, Triangle, Vec3, indexed_mesh::IndexedMesh},
};
use rayon::prelude::*;
use std::{
    fs::File,
    io::{BufRead, BufWriter, Cursor, Write},
};

pub fn validate_bytes(bytes: &[u8]) -> bool {
    let Ok(content) = std::str::from_utf8(bytes) else {
        return false;
    };

    let mut has_vertices = false;
    let mut has_faces = false;

    for line in content.lines().take(1000) {
        let line = line.trim();
        if line.is_empty() || line.starts_with("#") {
            continue;
        }

        if line.starts_with("v ") {
            has_vertices = true;
        } else if line.starts_with("f ") {
            has_faces = true;
        }

        if has_vertices && has_faces {
            return true;
        }
    }

    has_vertices
}

pub struct OBJParser;

impl MeshParser for OBJParser {
    fn parse(bytes: &[u8]) -> anyhow::Result<Vec<super::Triangle>, anyhow::Error> {
        let mut vertices = Vec::<Vec3>::new();
        let mut faces = Vec::<Vec<usize>>::new();

        let mut cursor = Cursor::new(bytes);
        let mut line_buf = String::new();

        while cursor.read_line(&mut line_buf)? > 0 {
            let line = line_buf.trim();

            // vertex parsing
            // v x y z
            if line.starts_with("v ") {
                let mut split = line.split_whitespace();
                split.next(); // skip the "v"

                let x: f32 = split
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("missing x coordinate on vertex"))?
                    .parse()?;
                let y: f32 = split
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("missing y coordinate on vertex"))?
                    .parse()?;
                let z: f32 = split
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("missing z coordinate on vertex"))?
                    .parse()?;

                vertices.push(Vec3(x, y, z));
            }
            // face parsing
            else if line.starts_with("f ") {
                let mut split = line.split_whitespace();
                split.next(); // skip the "f"

                let mut face = Vec::new();

                for part in split {
                    let mut nums = part.split('/');

                    let part_str = nums
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("missing vertex index on face"))?;
                    let idx: usize = part_str.parse()?;

                    face.push(idx - 1); // OBJ indices are 1-based
                }

                faces.push(face);
            }

            line_buf.clear();
        }

        let result_vecs: anyhow::Result<Vec<Vec<Triangle>>, anyhow::Error> = faces
            .par_iter()
            .map(|face_indices| {
                if face_indices.len() == 3 {
                    Ok(vec![Triangle {
                        vertices: [
                            vertices[face_indices[0]],
                            vertices[face_indices[1]],
                            vertices[face_indices[2]],
                        ],
                    }])
                } else if face_indices.len() > 3 {
                    let triangles = triangulate(&vertices, face_indices)?
                        .into_iter()
                        .collect::<Vec<Triangle>>();
                    Ok(triangles)
                } else {
                    Ok(Vec::new())
                }
            })
            .collect();

        let result: Vec<Triangle> = result_vecs?.into_iter().flatten().collect();
        Ok(result)
    }

    fn write(path: &std::path::Path, triangles: &[Triangle]) -> anyhow::Result<(), anyhow::Error> {
        let mesh = IndexedMesh::from_triangles(triangles);

        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        writeln!(writer, "# created by mesh_rs")?;
        writeln!(writer, "o Mesh")?;

        //  format v x y z
        //  we use string formatting ({:.6}) to write human readable numbers
        for vertex in &mesh.vertices {
            writeln!(writer, "v {:.6} {:.6} {:.6}", vertex.0, vertex.1, vertex.2)?;
        }

        // format "f v1 v2 v3"
        for face in &mesh.faces {
            // OBJ files are using 1-based indexing for vertex indices
            // (see the OBJ specification on top)
            writeln!(writer, "f {} {} {}", face[0] + 1, face[1] + 1, face[2] + 1)?;
        }

        writer.flush()?;
        anyhow::Ok(())
    }
}
