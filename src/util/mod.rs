use crate::ui;

pub const MIN_MM_VALUE: f64 = 1.0;

pub fn warn_units(file_name: &str, volume: f64, diagonal: f32) {
    if volume > MIN_MM_VALUE {
        return;
    }

    println!();
    ui::print_warn(&format!(
        "The object from file '{}' is too small, and may be in 'meters' or 'inches'",
        file_name,
    ));

    let suggested_diagonal = diagonal * 1000.0;

    ui::print_warn(&format!(
        "Consider scaling it to {:.2} mm diagonal using:",
        suggested_diagonal
    ));
    ui::print_bold(&format!(
        "       mesh_rs {} scale {}",
        file_name, suggested_diagonal
    ));
}
