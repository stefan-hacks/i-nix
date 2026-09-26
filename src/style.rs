//! nh-inspired TUI styling for i-nix.
#![allow(dead_code)]
//!
//! Provides beautiful, sectioned output with consistent colors,
//! progress indicators, and structured formatting.

use colored::Colorize;

/// Print a styled section header.
pub fn section(title: &str) {
    println!();
    println!("  {}", title.bright_white().bold());
    println!("  {}", "─".repeat(title.len()).bright_black());
}

/// Print a styled subsection header.
pub fn subsection(title: &str) {
    println!();
    println!("  {}", title.bright_white());
}

/// Print a key-value pair aligned.
pub fn kv(key: &str, value: &str) {
    println!("    {} {}", format!("{:20}", key).bright_black(), value);
}

/// Print a success message.
pub fn success(msg: &str) {
    println!("  {} {}", "✓".green().bold(), msg);
}

/// Print an error message.
pub fn error(msg: &str) {
    eprintln!("  {} {}", "✗".red().bold(), msg);
}

/// Print a warning message.
pub fn warning(msg: &str) {
    println!("  {} {}", "⚠".yellow().bold(), msg);
}

/// Print an info message.
pub fn info(msg: &str) {
    println!("  {} {}", "ℹ".blue(), msg);
}

/// Print a progress step.
pub fn step(n: usize, total: usize, msg: &str) {
    let indicator = format!("[{}/{}]", n, total).bright_black();
    println!("  {} {}", indicator, msg);
}

/// Print a dimmed note.
pub fn note(msg: &str) {
    println!("    {} {}", "→".dimmed(), msg.dimmed());
}

/// Print a command that would be run (dry-run style).
pub fn dry_run_cmd(cmd: &str) {
    println!("    {} {}", "$".bright_black().dimmed(), cmd.cyan());
}

/// Print a horizontal separator.
pub fn separator() {
    println!("  {}", "─".repeat(50).bright_black());
}

/// Print the i-nix header.
pub fn header() {
    println!();
    println!(
        "  {} {}",
        "i-nix".bright_cyan().bold(),
        "— Yet Another Nix Helper".bright_black()
    );
}

/// Print a table row.
pub fn table_row(cols: &[&str], widths: &[usize]) {
    let mut line = String::from("    ");
    for (i, (col, width)) in cols.iter().zip(widths.iter()).enumerate() {
        if i > 0 {
            line.push_str(" │ ");
        }
        let padded = format!("{:width$}", col, width = width);
        line.push_str(&padded);
    }
    println!("{}", line);
}

/// Print a table header row.
pub fn table_header(cols: &[&str], widths: &[usize]) {
    let mut line = String::from("    ");
    for (i, (col, width)) in cols.iter().zip(widths.iter()).enumerate() {
        if i > 0 {
            line.push_str(" │ ");
        }
        let padded = format!("{:width$}", col, width = width);
        line.push_str(&padded.bold().to_string());
    }
    println!("{}", line.bright_white());
    // separator
    let mut sep = String::from("    ");
    for (i, width) in widths.iter().enumerate() {
        if i > 0 {
            sep.push_str("─┼─");
        }
        sep.push_str(&"─".repeat(*width));
    }
    println!("{}", sep.bright_black());
}

/// Print a generation entry.
pub fn generation_entry(number: u32, current: bool, date: &str, version: &str, kernel: &str) {
    let marker = if current { "●".green().bold() } else { "○".bright_black() };
    println!(
        "  {} {:3} │ {:20} │ {:15} │ {}",
        marker,
        number.to_string().cyan(),
        date.bright_black(),
        version.bright_black(),
        kernel.bright_black()
    );
}

/// Print a diff entry (added/removed package).
pub fn diff_entry(name: &str, version: &str, added: bool) {
    let sign = if added { "+".green() } else { "-".red() };
    let color = if added { name.green() } else { name.red() };
    println!("    {} {} {}", sign, color, version.bright_black());
}

/// Print a package entry.
pub fn package_entry(name: &str, version: &str, description: &str) {
    println!(
        "    {} {} {}",
        "•".bright_black(),
        name.cyan().bold(),
        version.bright_black()
    );
    if !description.is_empty() {
        println!("      {}", description.dimmed());
    }
}

/// Print a search result.
pub fn search_result(name: &str, pkg_version: &str, channel: &str, desc: &str) {
    println!(
        "  {} {} {}",
        "▸".cyan(),
        name.bold(),
        pkg_version.bright_black()
    );
    println!("    {} {}", channel.dimmed(), desc);
}

/// Print a progress bar.
pub fn progress_bar(label: &str, pct: f32) {
    let width = 40;
    let filled = ((pct / 100.0) * width as f32) as usize;
    let bar = format!(
        "[{}{}]",
        "█".repeat(filled).green(),
        "░".repeat(width - filled).bright_black()
    );
    println!("  {:25} {} {:5.1}%", label.bright_black(), bar, pct);
}

/// Print a spinner message.
pub fn spinner(label: &str) {
    println!("  {} {}", "◐".cyan(), label);
}
