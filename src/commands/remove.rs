use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use crate::config::{INixState, InstallTarget};
use crate::engine::NixEngine;

pub async fn run(
    config_dir: &str,
    packages: Vec<String>,
    user: bool,
    verbose: bool,
    dry_run: bool,
) -> Result<()> {
    let config_path = Path::new(config_dir);
    let state = INixState::load(config_path)?;

    if state.version.is_empty() {
        anyhow::bail!("i-nix not initialized. Run `i-nix init` first.");
    }

    let target = if user {
        InstallTarget::User
    } else {
        InstallTarget::System
    };

    let username = std::env::var("USER").unwrap_or_else(|_| "user".to_string());
    let engine = NixEngine::new(
        config_path,
        &state.hostname,
        &username,
    );

    let mut removed = Vec::new();
    let mut not_found = Vec::new();

    for pkg in &packages {
        if dry_run {
            println!(
                "{} Would remove {} from {:?} packages",
                "DRY RUN:".yellow().bold(),
                pkg.cyan(),
                target
            );
            continue;
        }

        // Check if package exists in config before removing
        match engine.remove_package(pkg, target.clone()) {
            Ok(true) => {
                removed.push(pkg.clone());
                if verbose {
                    eprintln!("{} removed {} from configuration", "✓".green(), pkg);
                }
            }
            Ok(false) => {
                if verbose {
                    eprintln!("{} {} not found in configuration", "✗".red(), pkg);
                }
                not_found.push(pkg.clone());
            }
            Err(e) => {
                if verbose {
                    eprintln!("{} error removing {}: {}", "✗".red(), pkg, e);
                }
                not_found.push(pkg.clone());
            }
        }
    }

    if !removed.is_empty() {
        println!();
        println!(
            "{}",
            format!("✓ Removed {} package(s)", removed.len()).green().bold()
        );
        for pkg in &removed {
            println!("  {} {}", "→".cyan(), pkg);
        }
        println!();
        println!("  Run {} to apply changes.", "i-nix apply".bold());
        println!();
    }

    if !not_found.is_empty() {
        println!(
            "{} {} package(s) not found in configuration",
            "⚠".yellow(),
            not_found.len()
        );
        for pkg in &not_found {
            println!("    {}", pkg.dimmed());
        }
    }

    Ok(())
}
