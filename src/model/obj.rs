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
use crate::model::{Face, Group, Mesh, MeshCodec, Vec2, Vec3};
use std::{
    fs::File,
    io::{BufRead, BufWriter, Cursor, Write},
    path::Path,
};

pub struct ObjCodec;

impl MeshCodec for ObjCodec {
    fn parse(&self, bytes: &[u8]) -> anyhow::Result<Mesh> {
        let mut mesh = Mesh::default();
        let mut cursor = Cursor::new(bytes);
        let mut line_buf = String::new();

        let mut current_name = String::from("mesh_rs");
        let mut current_material: Option<String> = None;

        mesh.groups.push(Group {
            name: current_name.clone(),
            material: current_material.clone(),
            face_range: 0..0,
        });

        while cursor.read_line(&mut line_buf)? > 0 {
            let line = line_buf.trim();

            if line.is_empty() {
                line_buf.clear();
                continue;
            }

            if line.starts_with("v ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4
                    && let (Ok(x), Ok(y), Ok(z)) = (
                        parts[1].parse::<f32>(),
                        parts[2].parse::<f32>(),
                        parts[3].parse::<f32>(),
                    )
                {
                    mesh.vertices.push(Vec3(x, y, z));
                }
            } else if line.starts_with("vt ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3
                    && let (Ok(u), Ok(v)) = (parts[1].parse::<f32>(), parts[2].parse::<f32>())
                {
                    mesh.textures.push(Vec2(u, v));
                }
            } else if line.starts_with("vn ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4
                    && let (Ok(x), Ok(y), Ok(z)) = (
                        parts[1].parse::<f32>(),
                        parts[2].parse::<f32>(),
                        parts[3].parse::<f32>(),
                    )
                {
                    mesh.normals.push(Vec3(x, y, z));
                }
            // Face parsing
            // v1/vt1/vn1 v2/vt2/vn2 v3/vt3/vn3 # face with texture and normals
            // v1//vn1 v2//vn2 v3//vn3 # face with normals only
            // v1/vt1 v2/vt2 v3/vt3 # face with only texture index
            } else if line.starts_with("f ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                let mut face = Face::default();

                for part in parts.iter().skip(1) {
                    let segemnt: Vec<&str> = part.split('/').collect();

                    if let Ok(idx) = segemnt[0].parse::<u32>() {
                        face.v.push((idx - 1) as usize); // OBJ indices are 1-based
                    } else {
                        // vertex index is required to process the face
                        continue;
                    }

                    // texture index (optional)
                    if segemnt.len() > 1
                        && !segemnt[1].is_empty()
                        && let Ok(idx) = segemnt[1].parse::<u32>()
                    {
                        face.vt.push((idx - 1) as usize);
                    }

                    // normal index (optional)
                    if segemnt.len() > 2
                        && !segemnt[2].is_empty()
                        && let Ok(idx) = segemnt[2].parse::<u32>()
                    {
                        face.vn.push((idx - 1) as usize);
                    }
                }

                mesh.faces.push(face);
            } else if let Some(matlib) = line.strip_prefix("mtllib ") {
                mesh.matlibs.push(matlib.trim().to_string());
            } else if line.starts_with("o ")
                || line.starts_with("g ")
                || line.starts_with("usemtl ")
            {
                // close the range of the previous group
                if let Some(last_group) = mesh.groups.last_mut() {
                    last_group.face_range.end = mesh.faces.len();
                }

                match line.starts_with("usemtl ") {
                    true => {
                        current_material = Some(line[7..].trim().to_string());
                    }
                    false => {
                        // trim the "o " or "g "
                        current_name = line[2..].trim().to_string();
                    }
                }

                mesh.groups.push(Group {
                    name: current_name.clone(),
                    material: current_material.clone(),
                    face_range: mesh.faces.len()..mesh.faces.len(),
                });
            }

            line_buf.clear();
        }

        // close the range of the last group
        if let Some(last_group) = mesh.groups.last_mut() {
            last_group.face_range.end = mesh.faces.len();
        }

        Ok(mesh)
    }

    fn write(&self, path: &Path, mesh: &Mesh) -> anyhow::Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        writeln!(writer, "# created by mesh_rs")?;

        // write data arrays
        for v in &mesh.vertices {
            writeln!(writer, "v {:.6} {:.6} {:.6}", v.0, v.1, v.2)?;
        }
        for vt in &mesh.textures {
            writeln!(writer, "vt {:.6} {:.6}", vt.0, vt.1)?;
        }
        for vn in &mesh.normals {
            writeln!(writer, "vn {:.6} {:.6} {:.6}", vn.0, vn.1, vn.2)?;
        }

        // write faces, grouped by groups
        for group in &mesh.groups {
            // skip emtpy or default groups created during parsing
            if group.face_range.start >= group.face_range.end && group.name == "mesh_rs" {
                continue;
            }

            writeln!(writer, "g {}", group.name)?;

            if let Some(material) = &group.material {
                writeln!(writer, "usemtl {}", material)?;
            }

            for i in group.face_range.clone() {
                if i >= mesh.faces.len() {
                    break;
                }

                let face = &mesh.faces[i];
                write!(writer, "f")?;

                for j in 0..face.v.len() {
                    // write vertex index (1-based)
                    write!(writer, " {}", face.v[j] + 1)?;

                    let has_vt = j < face.vt.len();
                    let has_vn = j < face.vn.len();

                    if has_vt || has_vn {
                        write!(writer, "/")?;
                        if has_vt {
                            write!(writer, "{}", face.vt[j] + 1)?;
                        }
                    }

                    if has_vn {
                        write!(writer, "/{}", face.vn[j] + 1)?;
                    }
                }

                writeln!(writer)?;
            }
        }

        writer.flush()?;
        Ok(())
    }
}

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
