use anyhow::Result;
use colored::Colorize;
use std::path::Path;
use tokio::process::Command;

use crate::config::INixState;

pub async fn run(config_dir: &str, all: bool, _verbose: bool, dry_run: bool) -> Result<()> {
    let config_path = Path::new(config_dir);
    let state = INixState::load(config_path)?;

    if state.version.is_empty() {
        anyhow::bail!("i-nix not initialized. Run `i-nix init` first.");
    }

    if dry_run {
        println!("{}", "DRY RUN: Would update flake inputs".yellow().bold());
        return Ok(());
    }

    let flake_dir = config_path.join("flake");

    println!();
    println!("{}", "Updating flake inputs...".bold());
    println!();

    let update = Command::new("nix")
        .args([
            "flake",
            "update",
            "--flake",
            flake_dir.to_str().unwrap_or("."),
        ])
        .output()
        .await;

    match update {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            println!("{}", stdout.trim());
            println!();
            println!("{}", "✓ Flake inputs updated.".green().bold());
            println!("  Run {} to apply changes.", "i-nix apply".bold());
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("{} {}", "✗".red(), "nix flake update failed".red().bold());
            eprintln!("{}", stderr);
        }
        Err(e) => {
            eprintln!("{} nix flake update failed: {}", "✗".red(), e);
        }
    }

    Ok(())
}
