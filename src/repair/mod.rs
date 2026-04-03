use std::collections::{HashMap, HashSet};

use crate::model::{Mesh, Vec3};

pub fn remove_degenerate_faces(mesh: &mut Mesh) -> usize {
    let vertices = &mesh.vertices;
    let before = mesh.faces.len();

    mesh.faces.retain(|face| {
        let indices = &face.v;
        if indices.len() < 3 {
            return false;
        }

        let v0 = vertices[indices[0]];
        for i in 1..indices.len() - 1 {
            let v1 = vertices[indices[i]];
            let v2 = vertices[indices[i + 1]];

            if !triangle_is_degenerate(v0, v1, v2) {
                return true;
            }
        }

        false
    });

    before - mesh.faces.len()
}

#[inline]
fn triangle_is_degenerate(v0: Vec3, v1: Vec3, v2: Vec3) -> bool {
    const AREA_EPSILON: f32 = f32::EPSILON * f32::EPSILON;

    let edge1 = v1.substraction(v0);
    let edge2 = v2.substraction(v0);
    let cross = edge1.cross(edge2);

    cross.dot(cross) < AREA_EPSILON
}

