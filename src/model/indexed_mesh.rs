use crate::model::{Mesh, Triangle, Vec3};
use std::collections::HashMap;

#[derive(Debug)]
pub struct IndexedMesh {
    pub vertices: Vec<Vec3>,
    pub faces: Vec<[usize; 3]>,
}

impl Mesh for IndexedMesh {
    fn bounds(&self) -> anyhow::Result<[(f32, f32, f32); 2], anyhow::Error> {
        if self.vertices.is_empty() {
            return Err(anyhow::anyhow!("mesh has no vertices"));
        }

        let (min_vertex, max_vertex) = self.vertices.iter().fold(
            (
                (f32::MAX, f32::MAX, f32::MAX),
                (f32::MIN, f32::MIN, f32::MIN),
            ),
            |acc, v| {
                (
                    (acc.0.0.min(v.0), acc.0.1.min(v.1), acc.0.2.min(v.2)),
                    (acc.1.0.max(v.0), acc.1.1.max(v.1), acc.1.2.max(v.2)),
                )
            },
        );

        anyhow::Ok([min_vertex, max_vertex])
    }

    fn diagonal(&self) -> anyhow::Result<f32, anyhow::Error> {
        let [min_vertex, max_vertex] = self.bounds()?;

        let dx = max_vertex.0 - min_vertex.0;
        let dy = max_vertex.1 - min_vertex.1;
        let dz = max_vertex.2 - min_vertex.2;

        let current_diagonal = (dx * dx + dy * dy + dz * dz).sqrt();
        if current_diagonal == 0.0 {
            return Err(anyhow::anyhow!("mesh has 0 dimensions"));
        }

        anyhow::Ok(current_diagonal)
    }
}

impl IndexedMesh {
    pub fn from_triangles(triangles: &[Triangle]) -> Self {
        let mut vertices = Vec::new();
        let mut faces = Vec::new();
        let mut map = HashMap::new();

        for triangle in triangles {
            let mut face = [0usize; 3];
            for (i, vertex) in triangle.vertices.iter().enumerate() {
                let bitpattern = [vertex.0.to_bits(), vertex.1.to_bits(), vertex.2.to_bits()];

                let idx = *map.entry(bitpattern).or_insert_with(|| {
                    let idx = vertices.len();
                    vertices.push(*vertex);
                    idx
                });

                face[i] = idx;
            }
            faces.push(face)
        }

        IndexedMesh { vertices, faces }
    }
}
