use crate::model::{Triangle, Vec3};
use std::collections::HashMap;

#[derive(Debug)]
pub struct IndexedMesh {
    pub vertices: Vec<Vec3>,
    pub faces: Vec<[usize; 3]>,
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

        println!(
            "Vertex deduplication: {} triangles have {} unique vertices (merged {} duplicates)",
            triangles.len(),
            vertices.len(),
            triangles.len() * 3 - vertices.len()
        );

        IndexedMesh { vertices, faces }
    }

    pub fn to_triangles(&self) -> Vec<Triangle> {
        self.faces
            .iter()
            .map(|face| Triangle {
                vertices: [
                    self.vertices[face[0]],
                    self.vertices[face[1]],
                    self.vertices[face[2]],
                ],
            })
            .collect()
    }

    pub fn diagonal(&self) -> anyhow::Result<f32> {
        if self.vertices.is_empty() {
            return Err(anyhow::anyhow!("mesh has no vertices"));
        }

        let mut min = self.vertices[0];
        let mut max = self.vertices[0];

        for vertex in &self.vertices[1..] {
            min.0 = min.0.min(vertex.0);
            min.1 = min.1.min(vertex.1);
            min.2 = min.2.min(vertex.2);

            max.0 = max.0.max(vertex.0);
            max.1 = max.1.max(vertex.1);
            max.2 = max.2.max(vertex.2);
        }

        let dx = max.0 - min.0;
        let dy = max.1 - min.1;
        let dz = max.2 - min.2;

        let diagonal = (dx * dx + dy * dy + dz * dz).sqrt();
        if diagonal == 0.0 {
            return Err(anyhow::anyhow!("mesh has 0 dimensions"));
        }

        Ok(diagonal)
    }
}
