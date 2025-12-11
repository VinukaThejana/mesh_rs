pub mod obj;
pub mod stl;

use std::ops::Range;

use nalgebra::Vector3;
use rayon::iter::{IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator};
use smallvec::SmallVec;

pub const MAX_TRIANGLES: u32 = 1_000_000;

#[derive(Debug)]
pub enum Format {
    STL,
    OBJ,
}

impl Format {
    pub fn from_content_type(content_type: &str) -> Option<Self> {
        if content_type.contains("application/sla")
            || content_type.contains("application/vnd.ms-pki.stl")
            || content_type.contains("model/stl")
        {
            Some(Format::STL)
        } else if content_type.contains("model/obj") || content_type.contains("application/x-tgif")
        {
            Some(Format::OBJ)
        } else {
            None
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().rsplit('.').next()? {
            "stl" => Some(Format::STL),
            "obj" => Some(Format::OBJ),
            _ => None,
        }
    }

    pub fn from_magic_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.is_empty() {
            return None;
        }

        // STL file detection
        // binary STL files detection
        if bytes.len() >= 84 {
            let traingle_count = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]);
            if traingle_count > 0
                && traingle_count <= MAX_TRIANGLES
                && let Some(expected_size) = 84usize.checked_add(traingle_count as usize * 50)
                && bytes.len() >= expected_size
                && bytes.len() <= expected_size + 80
            {
                return Some(Format::STL);
            }
        }

        // ASCII STL files detection
        if bytes.len() >= 5 && &bytes[..5] == b"solid" {
            let preview = &bytes[..bytes.len().min(4096)];
            if let Ok(content) = std::str::from_utf8(preview)
                && content.contains("facet")
                && content.contains("vertex")
            {
                return Some(Format::STL);
            }
        }

        // OBJ file detection
        let preview = &bytes[..bytes.len().min(4096)];
        if let Ok(content) = std::str::from_utf8(preview) {
            let trimmed = content.trim_start();

            // OBJ files typically contain 'v ' (vertex), 'vt ' (texture), 'vn ' (normal), or 'f ' (face) lines
            let markers = trimmed
                .lines()
                .filter(|line| !line.trim().is_empty())
                .take(50)
                .any(|line| {
                    let line = line.trim_start();
                    line.starts_with("v ")
                        || line.starts_with("vt ")
                        || line.starts_with("vn ")
                        || line.starts_with("f ")
                        || line.starts_with("o ")
                        || line.starts_with("g ")
                        || line.starts_with("mtllib ")
                        || line.starts_with("usemtl ")
                });

            if markers {
                return Some(Format::OBJ);
            }
        }

        None
    }

    pub fn validate_bytes(&self, bytes: &[u8]) -> bool {
        match self {
            Self::STL => stl::validate_bytes(bytes),
            Self::OBJ => obj::validate_bytes(bytes),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::STL => "stl",
            Self::OBJ => "obj",
        }
    }

    pub fn get_codec(&self) -> Box<dyn MeshCodec> {
        match self {
            Self::STL => Box::new(stl::StlCodec),
            Self::OBJ => Box::new(obj::ObjCodec),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Mesh {
    // list of all vertices
    pub vertices: Vec<Vec3>,

    // list of all vertex normals
    // used to define smooth shading (how light interacts with the surface)
    pub normals: Vec<Vec3>,
    // list of all vertex texture coordinates
    // used to map 2D images to the 3D surface
    pub textures: Vec<Vec2>,

    // list of all the faces
    pub faces: Vec<Face>,
    // groups of faces
    // used to organize the mesh into logical sections
    // e.g., wheels of a car
    pub groups: Vec<Group>,

    // material libraries associated with the mesh
    pub matlibs: Vec<String>,
}

impl Mesh {
    pub fn scale(&mut self, target_diagonal: f32) -> anyhow::Result<()> {
        let (min_vertex, max_vertex) = self.bounds()?;

        let dx = max_vertex.0 - min_vertex.0;
        let dy = max_vertex.1 - min_vertex.1;
        let dz = max_vertex.2 - min_vertex.2;

        let current_diagonal = (dx * dx + dy * dy + dz * dz).sqrt();
        if current_diagonal == 0.0 {
            return Err(anyhow::anyhow!("mesh has 0 dimensions"));
        }

        let center_x = (min_vertex.0 + max_vertex.0) / 2.0;
        let center_y = (min_vertex.1 + max_vertex.1) / 2.0;
        let center_z = (min_vertex.2 + max_vertex.2) / 2.0;

        let scale_factor = target_diagonal / current_diagonal;

        self.vertices.par_iter_mut().for_each(|vertex| {
            vertex.0 = (vertex.0 - center_x) * scale_factor + center_x;
            vertex.1 = (vertex.1 - center_y) * scale_factor + center_y;
            vertex.2 = (vertex.2 - center_z) * scale_factor + center_z;
        });

        Ok(())
    }

    pub fn triangle_count(&self) -> usize {
        self.faces
            .iter()
            .map(|face| face.v.len().saturating_sub(2))
            .sum()
    }
}

impl Mesh {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            normals: Vec::new(),
            textures: Vec::new(),
            faces: Vec::new(),
            groups: Vec::new(),
            matlibs: Vec::new(),
        }
    }

    #[inline]
    pub fn bounds(&self) -> anyhow::Result<(Vec3, Vec3), anyhow::Error> {
        if self.vertices.is_empty() {
            return Err(anyhow::anyhow!("mesh has no vertices"));
        }

        let (min_vertex, max_vertex) = self
            .vertices
            .par_iter()
            .fold(
                || {
                    (
                        Vec3(f32::MAX, f32::MAX, f32::MAX),
                        Vec3(f32::MIN, f32::MIN, f32::MIN),
                    )
                },
                |acc, vertex| {
                    (
                        Vec3(
                            acc.0.0.min(vertex.0),
                            acc.0.1.min(vertex.1),
                            acc.0.2.min(vertex.2),
                        ),
                        Vec3(
                            acc.1.0.max(vertex.0),
                            acc.1.1.max(vertex.1),
                            acc.1.2.max(vertex.2),
                        ),
                    )
                },
            )
            .reduce(
                || {
                    (
                        Vec3(f32::MAX, f32::MAX, f32::MAX),
                        Vec3(f32::MIN, f32::MIN, f32::MIN),
                    )
                },
                |a, b| {
                    (
                        Vec3(a.0.0.min(b.0.0), a.0.1.min(b.0.1), a.0.2.min(b.0.2)),
                        Vec3(a.1.0.max(b.1.0), a.1.1.max(b.1.1), a.1.2.max(b.1.2)),
                    )
                },
            );

        Ok((min_vertex, max_vertex))
    }

    pub fn diagonal(&self) -> anyhow::Result<f32, anyhow::Error> {
        let (min_vertex, max_vertex) = self.bounds()?;

        let dx = max_vertex.0 - min_vertex.0;
        let dy = max_vertex.1 - min_vertex.1;
        let dz = max_vertex.2 - min_vertex.2;

        let current_diagonal = (dx * dx + dy * dy + dz * dz).sqrt();
        if current_diagonal == 0.0 {
            return Err(anyhow::anyhow!("mesh has 0 dimensions"));
        }

        Ok(current_diagonal)
    }
}

impl Default for Mesh {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Default, Clone)]
pub struct Face {
    // vertex indices
    pub v: SmallVec<[usize; 4]>,
    // vertex normal indices
    pub vn: SmallVec<[usize; 4]>,
    // vertex texture indices
    pub vt: SmallVec<[usize; 4]>,
}

#[derive(Debug, Clone)]
pub struct Group {
    // group name
    // e.g., "wheel", "door"
    pub name: String,

    // material name used by this group
    pub material: Option<String>,

    // range of faces in this group
    pub face_range: Range<usize>,
}

pub trait MeshCodec {
    fn parse(&self, bytes: &[u8]) -> anyhow::Result<Mesh>;
    fn write(&self, path: &std::path::Path, mesh: &Mesh) -> anyhow::Result<()>;
}

#[derive(Debug, Clone, Copy)]
pub struct Triangle {
    pub vertices: [Vec3; 3],
}

impl Triangle {
    #[inline]
    pub fn signed_volume(&self) -> f64 {
        let a: Vector3<f64> = self.vertices[0].into();
        let b: Vector3<f64> = self.vertices[1].into();
        let c: Vector3<f64> = self.vertices[2].into();

        (a.dot(&b.cross(&c))) / 6.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3(pub f32, pub f32, pub f32);

impl Vec3 {
    pub fn substraction(self, other: Vec3) -> Vec3 {
        Vec3(self.0 - other.0, self.1 - other.1, self.2 - other.2)
    }

    pub fn cross(self, other: Vec3) -> Vec3 {
        Vec3(
            // basically the determinant of a 3x3 matrix
            self.1 * other.2 - self.2 * other.1,
            self.2 * other.0 - self.0 * other.2,
            self.0 * other.1 - self.1 * other.0,
        )
    }

    pub fn dot(self, other: Vec3) -> f32 {
        self.0 * other.0 + self.1 * other.1 + self.2 * other.2
    }

    pub fn is_finite(self) -> bool {
        self.0.is_finite() && self.1.is_finite() && self.2.is_finite()
    }

    pub fn length(self) -> f32 {
        (self.0 * self.0 + self.1 * self.1 + self.2 * self.2).sqrt()
    }

    pub fn normalize(self) -> Vec3 {
        let len = self.length();
        if len > 0.0 {
            Vec3(self.0 / len, self.1 / len, self.2 / len)
        } else {
            Vec3(0.0, 0.0, 0.0)
        }
    }
}

impl From<[f32; 3]> for Vec3 {
    fn from(arr: [f32; 3]) -> Self {
        Vec3(arr[0], arr[1], arr[2])
    }
}

impl From<Vec3> for Vector3<f64> {
    fn from(v: Vec3) -> Self {
        Vector3::new(v.0.into(), v.1.into(), v.2.into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2(pub f32, pub f32);

impl Vec2 {
    pub fn substraction(self, other: Vec2) -> Self {
        Vec2(self.0 - other.0, self.1 - other.1)
    }

    pub fn cross(self, other: Vec2) -> f32 {
        self.0 * other.1 - self.1 * other.0
    }

    pub fn dot(self, other: Vec2) -> f32 {
        self.0 * other.0 + self.1 * other.1
    }
}
