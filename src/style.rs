#![allow(dead_code)]

//! i-nix visual style system — Charmbracelet-inspired modern terminal output.
//!
//! Provides:
//!   • Full color palette (pinks, purples, cyans, greens)
//!   • Progress bars with indicatif
//!   • Spinners with animated frames
//!   • Styled tables, boxes, and panels
//!   • ASCII art banners and headers
//!   • Consistent iconography (Nerd Font / Unicode symbols)

use colored::Colorize;
use console::Term;
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};

use std::time::Duration;

// ═════════════════════════════════════════════════════════════════════════════
// Color Palette — Charmbracelet Lip Gloss inspired
// ═════════════════════════════════════════════════════════════════════════════

pub const C_PRIMARY:    &str = "#FF79C6";  // Pink (Gum pink)
pub const C_SECONDARY:  &str = "#BD93F9";  // Purple
pub const C_ACCENT:     &str = "#8BE9FD";  // Cyan
pub const C_SUCCESS:    &str = "#50FA7B";  // Green
pub const C_WARNING:    &str = "#F1FA8C";  // Yellow
pub const C_ERROR:      &str = "#FF5555";  // Red
pub const C_DIM:        &str = "#6272A4";  // Comment gray
pub const C_TEXT:       &str = "#F8F8F2";  // White
pub const C_BG:         &str = "#282A36";  // Background

// ═════════════════════════════════════════════════════════════════════════════
// Unicode Icons / Nerd Font Symbols
// ═════════════════════════════════════════════════════════════════════════════

pub const ICON_CHECK:     &str = "✓";
pub const ICON_CROSS:     &str = "✗";
pub const ICON_WARN:      &str = "⚠";
pub const ICON_INFO:      &str = "ℹ";
pub const ICON_BULLET:    &str = "●";
pub const ICON_ARROW:     &str = "→";
pub const ICON_DOT:       &str = "•";
pub const ICON_STAR:      &str = "★";
pub const ICON_GEAR:      &str = "⚙";
pub const ICON_PACKAGE:   &str = "📦";
pub const ICON_HOME:      &str = "🏠";
pub const ICON_DESKTOP:   &str = "🖥";
pub const ICON_USER:      &str = "👤";
pub const ICON_FLAKE:     &str = "❄";
pub const ICON_NIXOS:     &str = "❄️";
pub const ICON_HEART:     &str = "♥";
pub const ICON_SPARKLE:   &str = "✨";
pub const ICON_ROCKET:    &str = "🚀";
pub const ICON_MAGNIFIER: &str = "🔍";
pub const ICON_WRENCH:    &str = "🔧";
pub const ICON_TRASH:     &str = "🗑";
pub const ICON_CLOCK:     &str = "⏱";
pub const ICON_SHIELD:    &str = "🛡";
pub const ICON_LIGHTNING: &str = "⚡";
pub const ICON_DISK:      &str = "💿";
pub const ICON_LOCK:      &str = "🔒";
pub const ICON_KEY:       &str = "🔑";
pub const ICON_BRANCH:    &str = "🌿";
pub const ICON_TAG:       &str = "🏷";
pub const ICON_CHART:     &str = "📊";

// ═════════════════════════════════════════════════════════════════════════════
// Terminal Utilities
// ═════════════════════════════════════════════════════════════════════════════

/// Detect if stdout supports color.
pub fn supports_color() -> bool {
    console::user_attended()
}

/// Get terminal width.
pub fn term_width() -> usize {
    Term::stdout().size().1 as usize
}

/// Center text in terminal.
pub fn center(text: &str) -> String {
    let width = term_width().saturating_sub(4);
    let pad = width.saturating_sub(text.len()) / 2;
    format!("{}{}", " ".repeat(pad), text)
}

// ═════════════════════════════════════════════════════════════════════════════
// Box Drawing / Panels
// ═════════════════════════════════════════════════════════════════════════════

/// Draw a rounded box around content.
pub fn rounded_box(lines: &[String], width: usize) -> Vec<String> {
    let mut out = Vec::new();
    let top = format!("╭{}╮", "─".repeat(width));
    let bot = format!("╰{}╯", "─".repeat(width));
    out.push(top);
    for line in lines {
        let padded = format!("{:width$}", line, width = width);
        out.push(format!("│{}│", padded));
    }
    out.push(bot);
    out
}

/// Draw a double-line box.
pub fn double_box(lines: &[String], width: usize) -> Vec<String> {
    let mut out = Vec::new();
    let top = format!("╔{}╗", "═".repeat(width));
    let bot = format!("╚{}╝", "═".repeat(width));
    out.push(top);
    for line in lines {
        let padded = format!("{:width$}", line, width = width);
        out.push(format!("║{}║", padded));
    }
    out.push(bot);
    out
}

/// Print a rounded panel with title.
pub fn panel(title: &str, lines: &[String]) {
    let width = lines.iter().map(|l| l.len()).max().unwrap_or(20).max(title.len() + 4);
    let box_lines = rounded_box(lines, width);
    println!();
    println!("  {}", format!("╭─ {} {}", title, "─".repeat(width.saturating_sub(title.len().saturating_add(3)))).bright_magenta());
    for line in &box_lines[1..box_lines.len()-1] {
        println!("  {}", line.bright_magenta());
    }
    println!("  {}", box_lines.last().unwrap().bright_magenta());
}

// ═════════════════════════════════════════════════════════════════════════════
// Headers & Banners
// ═════════════════════════════════════════════════════════════════════════════

/// Print the legacy header (deprecated, use header_compact or banner).
pub fn header() {
    header_compact();
}

/// Print a styled section header (legacy single-arg).
pub fn section_legacy(title: &str) {
    section("", title);
}
pub fn banner_art() {
    let art = vec![
        r#"    ╭────────────────────────────────────╮"#,
        r#"    │                                    │"#,
        r#"    │   ██  ██ ████  ██   ██ ███████    │"#,
        r#"    │   ██  ██  ██   ███  ██ ██         │"#,
        r#"    │   ██████   ██  ██ █ ██ █████      │"#,
        r#"    │   ██  ██   ██  ██  ███ ██         │"#,
        r#"    │   ██  ██  ████ ██   ██ ███████    │"#,
        r#"    │                                    │"#,
        r#"    │   Imperative UX for Nix/NixOS      │"#,
        r#"    ╰────────────────────────────────────╯"#,
    ];
    println!();
    for line in art {
        println!("{}", line.bright_cyan().bold());
    }
    println!();
}

/// Print a compact header with i-nix branding.
pub fn header_compact() {
    println!();
    println!(
        "  {} {}",
        "❄ i-nix".bright_cyan().bold(),
        "Imperative UX for Nix/NixOS".bright_black()
    );
    println!("  {}", "━".repeat(40).bright_black());
}

/// Print a command header with icon.
pub fn command_header(icon: &str, title: &str) {
    println!();
    println!(
        "  {} {}",
        icon.bright_magenta().bold(),
        title.bright_white().bold()
    );
    println!("  {}", "─".repeat(title.len() + 2).bright_black());
}

// ═════════════════════════════════════════════════════════════════════════════
// Section & Subsection Headers
// ═════════════════════════════════════════════════════════════════════════════

/// Print a styled section header with optional emoji.
pub fn section(icon: &str, title: &str) {
    println!();
    println!(
        "  {} {}",
        icon.bright_magenta(),
        title.bright_white().bold()
    );
    let len = title.len() + icon.len() + 1;
    println!("  {}", "─".repeat(len).bright_black());
}

/// Print a subsection header (legacy single-arg).
pub fn subsection_legacy(title: &str) {
    subsection("", title);
}
pub fn subsection(icon: &str, title: &str) {
    println!();
    println!(
        "    {} {}",
        icon.bright_cyan(),
        title.bright_white()
    );
}

/// Print a tiny label (e.g. for lists).
pub fn label(icon: &str, text: &str) {
    println!("    {} {}", icon.bright_black(), text);
}

// ═════════════════════════════════════════════════════════════════════════════
// Status Messages
// ═════════════════════════════════════════════════════════════════════════════

/// Print a success message.
pub fn success(msg: &str) {
    println!(
        "  {} {}",
        format!("{} Done", ICON_CHECK).green().bold(),
        msg
    );
}

/// Print an error message.
pub fn error(msg: &str) {
    eprintln!(
        "  {} {}",
        format!("{} Error", ICON_CROSS).red().bold(),
        msg
    );
}

/// Print a warning message.
pub fn warning(msg: &str) {
    println!(
        "  {} {}",
        format!("{} Warn", ICON_WARN).yellow().bold(),
        msg
    );
}

/// Print an info message.
pub fn info(msg: &str) {
    println!(
        "  {} {}",
        format!("{} Info", ICON_INFO).blue(),
        msg
    );
}

/// Print a note (dimmed).
pub fn note(msg: &str) {
    println!(
        "    {} {}",
        ICON_ARROW.dimmed(),
        msg.dimmed()
    );
}

/// Print a dry-run command preview.
pub fn dry_run_cmd(cmd: &str) {
    println!(
        "    {} {}",
        "$".bright_black().dimmed(),
        cmd.cyan()
    );
}

/// Print a step in a multi-step process.
pub fn step(n: usize, total: usize, msg: &str) {
    let indicator = format!("[{}/{}]", n, total).bright_black();
    println!("  {} {}", indicator, msg);
}

// ═════════════════════════════════════════════════════════════════════════════
// Lists & Tables
// ═════════════════════════════════════════════════════════════════════════════

/// Print a bulleted list item with optional color.
pub fn bullet(text: &str, color: Option<&str>) {
    let bullet = match color {
        Some("green") => ICON_BULLET.green().bold().to_string(),
        Some("cyan") => ICON_BULLET.cyan().bold().to_string(),
        Some("yellow") => ICON_BULLET.yellow().bold().to_string(),
        Some("blue") => ICON_BULLET.blue().bold().to_string(),
        Some("magenta") => ICON_BULLET.magenta().bold().to_string(),
        _ => ICON_BULLET.bright_black().to_string(),
    };
    println!("  {} {}", bullet, text);
}

/// Print a key-value pair aligned.
pub fn kv(key: &str, value: &str) {
    println!(
        "    {} {}",
        format!("{:22}", key).bright_black(),
        value
    );
}

/// Print a key-value pair with colored value.
pub fn kv_colored(key: &str, value: &str, value_color: &str) {
    let val = match value_color {
        "green" => value.green().to_string(),
        "cyan" => value.cyan().to_string(),
        "magenta" => value.magenta().to_string(),
        "yellow" => value.yellow().to_string(),
        _ => value.to_string(),
    };
    println!("    {} {}", format!("{:22}", key).bright_black(), val);
}

/// Print a table header row.
pub fn table_header(cols: &[(&str, usize)]) {
    let mut line = String::from("    ");
    let mut sep = String::from("    ");
    for (i, (col, width)) in cols.iter().enumerate() {
        if i > 0 {
            line.push_str(" │ ");
            sep.push_str("─┼─");
        }
        line.push_str(&format!("{:<width$}", col.bold(), width = width));
        sep.push_str(&"─".repeat(*width));
    }
    println!("{}", line.bright_white());
    println!("{}", sep.bright_black());
}

/// Print a table data row.
pub fn table_row(cols: &[(&str, usize)]) {
    let mut line = String::from("    ");
    for (i, (col, width)) in cols.iter().enumerate() {
        if i > 0 {
            line.push_str(" │ ");
        }
        line.push_str(&format!("{:<width$}", col, width = width));
    }
    println!("{}", line);
}

// ═════════════════════════════════════════════════════════════════════════════
// Indicatif Progress Bars
// ═════════════════════════════════════════════════════════════════════════════

/// Create a styled progress bar.
pub fn progress_bar(total: u64, msg: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    let style = ProgressStyle::default_bar()
        .template(
            "{spinner:.cyan} [{elapsed_precise}] [{bar:40.cyan/bright_black}] {pos}/{len} {msg}"
        )
        .unwrap()
        .progress_chars("█▓▒░");
    pb.set_style(style);
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

/// Create a spinner progress bar.
pub fn spinner_bar(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    let style = ProgressStyle::default_spinner()
        .tick_strings(&[
            "◐", "◓", "◑", "◒", "✓",
        ])
        .template("{spinner:.cyan.bold} {msg}")
        .unwrap();
    pb.set_style(style);
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

/// Create a multi-progress handle for parallel bars.
pub fn multi_progress() -> MultiProgress {
    MultiProgress::new()
}

// ═════════════════════════════════════════════════════════════════════════════
// ASCII Progress Bar (for lightweight use without indicatif)
// ═════════════════════════════════════════════════════════════════════════════

/// Print a simple ASCII progress bar.
pub fn ascii_progress(label: &str, current: usize, total: usize) {
    let width = 40;
    let pct = if total > 0 { (current as f32 / total as f32) * 100.0 } else { 0.0 };
    let filled = ((pct / 100.0) * width as f32) as usize;
    let bar = format!(
        "[{}{}]",
        "█".repeat(filled).cyan(),
        "░".repeat((width as usize).saturating_sub(filled)).bright_black()
    );
    println!(
        "  {:28} {} {:6.1}%",
        label.bright_black(),
        bar,
        pct
    );
}

/// Print a banner with command name and description.
pub fn banner(name: &str, desc: &str) {
    let width: usize = 60;
    let inner = width.saturating_sub(4);
    let name_pad = inner.saturating_sub(name.len()).saturating_sub(2) / 2;
    let desc_pad = inner.saturating_sub(desc.len()).saturating_sub(2) / 2;
    println!();
    println!("{}", format!("╭{}╮", "─".repeat(width - 2)).cyan());
    println!(
        "{}",
        format!(
            "│{}{}  {}{}│",
            " ".repeat(name_pad),
            name.cyan().bold(),
            " ".repeat(inner.saturating_sub(name_pad).saturating_sub(name.len()).saturating_sub(2)),
            ""
        )
        .cyan()
    );
    println!(
        "{}",
        format!(
            "│{}{}{}{}│",
            " ".repeat(desc_pad),
            desc.dimmed(),
            " ".repeat(inner.saturating_sub(desc_pad).saturating_sub(desc.len()).saturating_sub(2)),
            ""
        )
        .cyan()
    );
    println!("{}", format!("╰{}╯", "─".repeat(width - 2)).cyan());
    println!();
}

/// Print a single-item label (e.g. "discovered: <value>").
pub fn item_label(label: &str, value: &str) {
    println!("    {} {}", format!("{}", label).bright_black(), value.cyan());
}

/// Print a completion badge with checkmark.
pub fn completion_badge(msg: &str) {
    println!(
        "  {} {}",
        format!("{} Done", ICON_CHECK).green().bold(),
        msg.bright_white().bold()
    );
}

/// Print a step message (simple single-arg version).
pub fn step_simple(msg: &str) {
    println!();
    println!("  {} {}", ICON_ARROW.cyan(), msg.bright_white());
}

/// Print a list of items with bullets.
pub fn list(items: &[&str]) {
    for item in items {
        println!("    {} {}", "·".cyan(), item);
    }
}

/// Print a footer separator.
pub fn footer() {
    println!("  {}", "─".repeat(50).bright_black());
}

/// Wrap text to terminal width.
pub fn wrap(text: &str) -> Vec<String> {
    let _w = term_width().saturating_sub(6);
    text.split_whitespace().collect::<Vec<_>>()
        .chunks(10)
        .map(|chunk| chunk.join(" "))
        .collect()
}

/// Print a completion badge.
pub fn badge(label: &str, variant: &str) {
    let (fg, bg) = match variant {
        "success" => ("🟢", "green"),
        "warning" => ("🟡", "yellow"),
        "error" => ("🔴", "red"),
        "info" => ("🔵", "blue"),
        _ => ("⚪", "white"),
    };
    let text = format!(" {} {} ", fg, label);
    let styled = match bg {
        "green" => text.on_bright_green().black().bold().to_string(),
        "yellow" => text.on_bright_yellow().black().bold().to_string(),
        "red" => text.on_bright_red().black().bold().to_string(),
        "blue" => text.on_bright_blue().white().bold().to_string(),
        _ => text.on_white().black().bold().to_string(),
    };
    print!("{} ", styled);
}

// ═════════════════════════════════════════════════════════════════════════════
// Generations / Diff / Package Listings
// ═════════════════════════════════════════════════════════════════════════════

/// Print a generation entry.
pub fn generation_entry(number: u32, current: bool, date: &str, version: &str, kernel: &str) {
    let marker = if current {
        ICON_BULLET.green().bold().to_string()
    } else {
        "○".bright_black().to_string()
    };
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

/// Print a package entry with description.
pub fn package_entry(name: &str, version: &str, description: &str) {
    println!(
        "  {} {} {}",
        ICON_DOT.bright_black(),
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
        "▸".magenta(),
        name.bold(),
        pkg_version.bright_black()
    );
    println!("    {} {}", channel.dimmed(), desc);
}

// ═════════════════════════════════════════════════════════════════════════════
// Separators
// ═════════════════════════════════════════════════════════════════════════════

/// Print a horizontal separator line.
pub fn separator() {
    let width = term_width().saturating_sub(4).min(60);
    println!("  {}", "─".repeat(width).bright_black());
}

/// Print a thick separator.
pub fn thick_separator() {
    let width = term_width().saturating_sub(4).min(60);
    println!("  {}", "━".repeat(width).bright_black());
}

/// Print an empty line.
pub fn blank() {
    println!();
}

// ═════════════════════════════════════════════════════════════════════════════
// Dialoguer Helpers (interactive prompts)
// ═════════════════════════════════════════════════════════════════════════════

/// Confirm yes/no with styled prompt.
pub fn confirm(prompt: &str, default: bool) -> bool {
    use dialoguer::{theme::ColorfulTheme, Confirm};
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .default(default)
        .interact()
        .unwrap_or(default)
}

/// Select from a list with styled prompt.
pub fn select(title: &str, items: &[&str]) -> Option<usize> {
    use dialoguer::{theme::ColorfulTheme, Select};
    Select::with_theme(&ColorfulTheme::default())
        .with_prompt(title)
        .items(items)
        .interact_opt()
        .ok()
        .flatten()
}

/// Multi-select with styled prompt.
pub fn multi_select(title: &str, items: &[&str]) -> Vec<usize> {
    use dialoguer::{theme::ColorfulTheme, MultiSelect};
    MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt(title)
        .items(items)
        .interact()
        .unwrap_or_default()
}

// ═════════════════════════════════════════════════════════════════════════════
// Help Template Utilities (for clap custom help)
// ═════════════════════════════════════════════════════════════════════════════

/// Return a colored clap help template.
pub fn clap_help_template() -> String {
    format!(
        "{}

{}

{}

{}

{}
        ",
        "{about}".bright_cyan().bold(),
        "{usage-heading}".bright_white().bold(),
        "{usage}".bright_white(),
        "{all-args}".bright_white(),
        "{subcommands}".bright_white(),
    )
}

/// Style a command name for help.
pub fn fmt_cmd(name: &str) -> String {
    name.bright_cyan().bold().to_string()
}

/// Style an option flag for help.
pub fn fmt_flag(flag: &str) -> String {
    flag.bright_magenta().to_string()
}

/// Style a description for help.
pub fn fmt_desc(desc: &str) -> String {
    desc.bright_black().to_string()
}
