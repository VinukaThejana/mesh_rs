pub mod obj;
pub mod stl;

use nalgebra::Vector3;

pub const MAX_TRIANGLES: u32 = 1_000_000;

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

#[derive(Debug, Clone)]
pub struct Triangle {
    pub vertices: [Vec3; 3],
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

impl Triangle {
    pub fn signed_volume(&self) -> f64 {
        let a: Vector3<f64> = self.vertices[0].into();
        let b: Vector3<f64> = self.vertices[1].into();
        let c: Vector3<f64> = self.vertices[2].into();

        (a.dot(&b.cross(&c))) / 6.0
    }

    pub fn is_valid(&self) -> bool {
        let v0 = self.vertices[0];
        let v1 = self.vertices[1];
        let v2 = self.vertices[2];

        if !v0.is_finite() || !v1.is_finite() || !v2.is_finite() {
            return false;
        }
        if v0 == v1 || v1 == v2 || v2 == v0 {
            return false;
        }

        // zero area check
        // creates two edges making v0 the origin
        let a = v1.substraction(v0);
        let b = v2.substraction(v0);

        // cross product of the two edges
        // gives a vector orthogonal to the triangle
        // whose length is proportional to the area of the triangle
        let cross = a.cross(b);
        // area squared is the dot product of the cross product with itself
        let area_sq = cross.dot(cross);

        area_sq > 1e-12
    }
}

pub trait MeshParser {
    fn parse(bytes: &[u8]) -> anyhow::Result<Vec<Triangle>, anyhow::Error>;
}

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
}
