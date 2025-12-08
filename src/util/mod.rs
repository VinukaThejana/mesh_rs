use colored::*;

pub const MIN_MM_VALUE: f64 = 1.0;

pub fn warn_units(file_name: &str, volume: f64, diagonal: f32) {
    if volume > MIN_MM_VALUE {
        return;
    }

    println!();
    eprintln!(
        "{} The object from file '{}' is too small, and may be in 'meters' or 'inches'",
        "[Warn]".yellow().bold(),
        file_name,
    );

    let suggested_diagonal = diagonal * 1000.0;

    eprintln!(
        "{} Consider scaling it to {:.2} mm diagonal using:",
        "[Warn]".yellow().bold(),
        suggested_diagonal
    );
    eprintln!("       mesh_rs {} scale {}", file_name, suggested_diagonal);
}
