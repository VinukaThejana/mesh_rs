pub mod triangulation;

use crate::model::{Mesh, Triangle, indexed_mesh::IndexedMesh};
use core::f32;
use rayon::prelude::*;

pub fn volume(mesh: &IndexedMesh) -> f64 {
    let triangles: Vec<Triangle> = mesh.into();
    if triangles.is_empty() {
        return 0.0;
    }

    const PARALLEL_THRESHOLD: usize = 1000;
    const CHUNK_SIZE: usize = 1000;

    let total_volume: f64 = if triangles.len() >= PARALLEL_THRESHOLD {
        triangles.par_chunks(CHUNK_SIZE).map(kahan_sum).sum()
    } else {
        kahan_sum(&triangles)
    };

    total_volume.abs()
}

#[inline]
fn kahan_sum(triangles: &[Triangle]) -> f64 {
    let mut sum = 0.0f64;
    let mut compensation = 0.0f64;

    for triangle in triangles {
        let y = triangle.signed_volume() - compensation;
        let t = sum + y;
        compensation = (t - sum) - y;
        sum = t;
    }

    sum
}

pub fn scale(
    mesh: &mut IndexedMesh,
    new_diagonal: f32,
) -> anyhow::Result<Vec<Triangle>, anyhow::Error> {
    let [min_vertex, max_vertex] = mesh.bounds()?;

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

    anyhow::Ok(mesh.into())
}

pub fn diagonal(mesh: &IndexedMesh) -> anyhow::Result<f32, anyhow::Error> {
    mesh.diagonal()
}
