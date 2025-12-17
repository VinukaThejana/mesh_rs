use crate::{model::Mesh, ui};

pub const MIN_MM_VALUE: f64 = 1.0;

pub fn warn_units(file_name: &str, volume: f64, diagonal: f32) {
    if volume > MIN_MM_VALUE {
        return;
    }

    ui::print_newline();
    ui::print_warn(&format!(
        "the object from file '{}' is too small, and may be in 'meters' or 'inches'",
        file_name,
    ));

    let suggested_diagonal = diagonal * 1000.0;

    ui::print_warn(&format!(
        "consider scaling it to {:.2} mm diagonal using:",
        suggested_diagonal
    ));
    ui::print_bold(&format!(
        "       mesh_rs {} scale {}",
        file_name, suggested_diagonal
    ));
}

pub fn warn_topology(mesh: &Mesh) {
    let map = mesh.topology();

    let mut non_manifold_edges_count = 0;
    let mut boundary_edges_count = 0;

    for (_, count) in map {
        if count == 1 {
            boundary_edges_count += 1;
        } else if count > 2 {
            non_manifold_edges_count += 1;
        }
    }

    if non_manifold_edges_count > 0 {
        ui::print_newline();
        ui::print_warn(&format!(
            "the mesh has {} non-manifold edges.",
            non_manifold_edges_count
        ));
        ui::print_warn("this may lead to issues in 3D printing or simulations.");
    } else if boundary_edges_count > 0 {
        ui::print_newline();
        ui::print_warn(&format!(
            "the mesh has {} boundary edges.",
            boundary_edges_count
        ));
        ui::print_warn("this indicates holes in the mesh that may need to be fixed.");
    }
}
