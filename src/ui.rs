use colored::*;
use std::fmt::Display;

pub fn print_error(msg: &str) {
    eprintln!("{} {}", "[Error]".red().bold(), msg);
}

pub fn print_success(msg: &str) {
    println!("{} {}", "[Success]".green().bold(), msg);
}

pub fn print_warn(msg: &str) {
    eprintln!("{} {}", "[Warn]".yellow().bold(), msg);
}

pub fn print_info(label: &str, msg: &str) {
    println!("{} {}", format!("[Info] {}:", label).cyan().bold(), msg);
}

pub fn print_section(title: &str) {
    println!("\n{}", title.bold().underline());
}

pub fn print_kv<T: Display>(key: &str, value: T) {
    println!("{:<15} {}", format!("{}:", key).bold(), value);
}

pub fn print_newline() {
    println!();
}

pub fn print_plain(msg: &str) {
    println!("{}", msg);
}

pub fn print_bold(msg: &str) {
    println!("{}", msg.bold());
}

pub fn print_underline(msg: &str) {
    println!("{}", msg.underline());
}

pub fn print_italic(msg: &str) {
    println!("{}", msg.italic());
}
