use anyhow::Result;
use colored::Colorize;
use std::path::Path;
use tokio::process::Command;

use crate::config::INixState;

pub async fn run(config_dir: &str) -> Result<()> {
    let config_path = Path::new(config_dir);
    let state = INixState::load(config_path)?;

    if state.version.is_empty() {
        anyhow::bail!("i-nix not initialized. Run `i-nix init` first.");
    }

    println!();
    println!("{}", "i-nix rollback".bold().underline());
    println!();

    if !crate::config::is_nixos() {
        println!("  {} Rollback only available on NixOS", "⚠".yellow());
        println!("  On non-NixOS systems, use git to revert changes.");
        println!();
        return Ok(());
    }

    // Show current generation
    let current_gen = get_current_generation();
    if let Some(generation) = current_gen {
        println!("  Current generation: {}", generation.to_string().cyan());
    }

    // List available generations
    let list = Command::new("nixos-rebuild")
        .args(["list-generations"])
        .output()
        .await;

    if let Ok(output) = list {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            println!();
            println!("{}", "Available generations:".bold());
            for line in stdout.lines().take(10) {
                println!("  {}", line);
            }
        }
    }

    println!();
    println!("  Run {} to rollback to the previous generation.", "sudo nixos-rebuild switch --rollback".dimmed());
    println!();

    Ok(())
}

fn get_current_generation() -> Option<u32> {
    std::fs::read_link("/nix/var/nix/profiles/system")
        .ok()
        .and_then(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .and_then(|s| s.split('-').nth(1))
                .and_then(|n| n.parse().ok())
        })
}
