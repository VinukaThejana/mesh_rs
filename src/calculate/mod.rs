pub mod triangulation;

use crate::model::{Face, Mesh, Triangle, Vec3};
use core::f32;
use rayon::prelude::*;

pub fn volume(mesh: &Mesh) -> f64 {
    if mesh.faces.is_empty() {
        return 0.0;
    }

    const PARALLEL_THRESHOLD: usize = 1000;
    const CHUNK_SIZE: usize = 1000;

    let total_volume: f64 = if mesh.faces.len() >= PARALLEL_THRESHOLD {
        mesh.faces
            .par_chunks(CHUNK_SIZE)
            .map(|chunk| kahan_sum_faces(chunk, &mesh.vertices))
            .sum()
    } else {
        kahan_sum_faces(&mesh.faces, &mesh.vertices)
    };

    total_volume.abs()
}

#[inline]
fn kahan_sum_faces(faces: &[Face], vertices: &[Vec3]) -> f64 {
    let mut sum = 0.0f64;
    let mut compensation = 0.0f64;

    for face in faces {
        let indices = &face.v;
        let n = indices.len();
        if n < 3 {
            continue;
        }

        let v0 = vertices[indices[0]];
        for i in 1..(n - 1) {
            let v1 = vertices[indices[i]];
            let v2 = vertices[indices[i + 1]];

            let volume = Triangle {
                vertices: [v0, v1, v2],
            }
            .signed_volume();

            let y = volume - compensation;
            let t = sum + y;
            compensation = (t - sum) - y;
            sum = t;
        }
    }
    sum
}

pub fn scale(mesh: &mut Mesh, new_diagonal: f32) -> anyhow::Result<()> {
    let (min_vertex, max_vertex) = mesh.bounds()?;

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

    let scale_factor = new_diagonal / current_diagonal;

    mesh.vertices.par_iter_mut().for_each(|vertex| {
        vertex.0 = (vertex.0 - center_x) * scale_factor + center_x;
        vertex.1 = (vertex.1 - center_y) * scale_factor + center_y;
        vertex.2 = (vertex.2 - center_z) * scale_factor + center_z;
    });

    Ok(())
}

pub fn diagonal(mesh: &Mesh) -> anyhow::Result<f32, anyhow::Error> {
    mesh.diagonal()
}

pub fn triangle_count(mesh: &Mesh) -> usize {
    mesh.triangle_count()
}
