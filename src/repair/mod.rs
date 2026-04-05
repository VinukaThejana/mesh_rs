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

pub fn remove_duplicate_faces(mesh: &mut Mesh) -> usize {
    let before = mesh.faces.len();
    let mut seen: HashSet<Vec<usize>> = HashSet::new();

    mesh.faces.retain(|face| {
        if face.v.len() < 3 {
            return true;
        }

        let mut key: Vec<usize> = face.v.to_vec();
        key.sort_unstable();

        seen.insert(key)
    });

    before - mesh.faces.len()
}

pub fn resolve_non_manifold_edges(mesh: &mut Mesh) -> (usize, usize) {
    let mut edge_faces: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
    for (face_index, face) in mesh.faces.iter().enumerate() {
        let n = face.v.len();
        for i in 0..n {
            let v1 = face.v[i];
            let v2 = face.v[(i + 1) % n];

            let edge = canonical_edge(v1, v2);
            edge_faces.entry(edge).or_default().push(face_index);
        }
    }

    let non_manifold_edges: Vec<((usize, usize), Vec<usize>)> = edge_faces
        .iter()
        .filter(|(_, faces)| faces.len() > 2)
        .map(|(edge, face_list)| (*edge, face_list.clone()))
        .collect();

    let mut faces_remapped = 0;
    for ((v1, v2), face_list) in &non_manifold_edges {
        for &face_index in face_list.iter().skip(2) {
            let new_v1 = mesh.vertices.len();
            mesh.vertices.push(mesh.vertices[*v1]);

            let new_v2 = mesh.vertices.len();
            mesh.vertices.push(mesh.vertices[*v2]);

            let face = &mut mesh.faces[face_index];
            for idx in face.v.iter_mut() {
                if *idx == *v1 {
                    *idx = new_v1;
                } else if *idx == *v2 {
                    *idx = new_v2;
                }
            }
            faces_remapped += 1;
        }
    }

    (non_manifold_edges.len(), faces_remapped)
}

#[inline]
fn canonical_edge(v1: usize, v2: usize) -> (usize, usize) {
    if v1 < v2 { (v1, v2) } else { (v2, v1) }
}
