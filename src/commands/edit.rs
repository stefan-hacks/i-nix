use anyhow::Result;
use colored::Colorize;
use std::env;
use std::process::Command;

pub async fn run(target: &str) -> Result<()> {
    let editor = env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());

    let path = match target {
        "packages" | "pkg" | "system" => "~/.config/i-nix/flake/systems/default.nix",
        "home" | "user" => "~/.config/i-nix/flake/home/default.nix",
        "flake" | "nix" => "~/.config/i-nix/flake/flake.nix",
        _ => target,
    };

    let expanded = shellexpand::tilde(path);

    println!();
    println!("{} Opening {} in {}", "→".cyan(), expanded.cyan(), editor.cyan());
    println!();

    let status = Command::new(&editor)
        .arg(expanded.as_ref())
        .status()?;

    if status.success() {
        println!("{}", "✓ Editor closed. Run `i-nix apply` to activate changes.".green());
    } else {
        println!("{} Editor exited with status: {}", "⚠".yellow(), status);
    }

    Ok(())
}
