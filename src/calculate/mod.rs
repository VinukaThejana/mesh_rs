pub mod triangulation;

use crate::model::Triangle;
use core::f32;
use rayon::prelude::*;

pub fn volume(triangles: &[Triangle]) -> f64 {
    if triangles.is_empty() {
        return 0.0;
    }

    const PARALLEL_THRESHOLD: usize = 1000;
    const CHUNK_SIZE: usize = 1000;

    let total_volume: f64 = if triangles.len() >= PARALLEL_THRESHOLD {
        triangles.par_chunks(CHUNK_SIZE).map(kahan_sum).sum()
    } else {
        kahan_sum(triangles)
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

pub fn scale(triangles: &mut [Triangle], new_diagonal: f32) -> anyhow::Result<(), anyhow::Error> {
    let [min_vertex, max_vertex] = bounds(triangles)?;

    let dx = max_vertex.0 - min_vertex.0;
    let dy = max_vertex.1 - min_vertex.1;
    let dz = max_vertex.2 - min_vertex.2;

    // diagonal length of the current bounding box (using euclidean distance)
    let current_diagonal = (dx * dx + dy * dy + dz * dz).sqrt();
    if current_diagonal == 0.0 {
        return Err(anyhow::anyhow!("mesh has 0 dimensions"));
    }

    let center_x = min_vertex.0 + (dx / 2.0);
    let center_y = min_vertex.1 + (dy / 2.0);
    let center_z = min_vertex.2 + (dz / 2.0);

    let scale_factor = new_diagonal / current_diagonal;

    triangles.par_iter_mut().for_each(|triangle| {
        for vertex in &mut triangle.vertices {
            vertex.0 = (vertex.0 - center_x) * scale_factor + center_x;
            vertex.1 = (vertex.1 - center_y) * scale_factor + center_y;
            vertex.2 = (vertex.2 - center_z) * scale_factor + center_z;
        }
    });

    anyhow::Ok(())
}

pub fn diagonal(triangles: &[Triangle]) -> anyhow::Result<f32, anyhow::Error> {
    let [min_vertex, max_vertex] = bounds(triangles)?;

    let dx = max_vertex.0 - min_vertex.0;
    let dy = max_vertex.1 - min_vertex.1;
    let dz = max_vertex.2 - min_vertex.2;

    // diagonal length of the current bounding box (using euclidean distance)
    let current_diagonal = (dx * dx + dy * dy + dz * dz).sqrt();
    if current_diagonal == 0.0 {
        return Err(anyhow::anyhow!("mesh has 0 dimensions"));
    }

    anyhow::Ok(current_diagonal)
}

fn bounds(triangles: &[Triangle]) -> anyhow::Result<[(f32, f32, f32); 2], anyhow::Error> {
    if triangles.is_empty() {
        return Err(anyhow::anyhow!("No triangles provided"));
    }

    let (min_vertex, max_vertex) = triangles
        .par_iter()
        .fold(
            || {
                (
                    (f32::MAX, f32::MAX, f32::MAX),
                    (f32::MIN, f32::MIN, f32::MIN),
                )
            },
            |acc, triangle| {
                let mut local_min = acc.0;
                let mut local_max = acc.1;

                for vertex in &triangle.vertices {
                    local_min.0 = local_min.0.min(vertex.0);
                    local_min.1 = local_min.1.min(vertex.1);
                    local_min.2 = local_min.2.min(vertex.2);

                    local_max.0 = local_max.0.max(vertex.0);
                    local_max.1 = local_max.1.max(vertex.1);
                    local_max.2 = local_max.2.max(vertex.2);
                }
                (local_min, local_max)
            },
        )
        .reduce(
            || {
                (
                    (f32::MAX, f32::MAX, f32::MAX),
                    (f32::MIN, f32::MIN, f32::MIN),
                )
            },
            |acc1, acc2| {
                (
                    (
                        acc1.0.0.min(acc2.0.0),
                        acc1.0.1.min(acc2.0.1),
                        acc1.0.2.min(acc2.0.2),
                    ),
                    (
                        acc1.1.0.max(acc2.1.0),
                        acc1.1.1.max(acc2.1.1),
                        acc1.1.2.max(acc2.1.2),
                    ),
                )
            },
        );

    anyhow::Ok([min_vertex, max_vertex])
}
