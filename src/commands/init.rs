use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::path::Path;

use crate::config::{INixMode, INixState};
use crate::engine::NixEngine;

pub async fn run(
    config_dir: &str,
    hostname: Option<String>,
    user_only: bool,
    system: bool,
    verbose: bool,
    dry_run: bool,
) -> Result<()> {
    let config_path = Path::new(config_dir);
    
    if config_path.join("flake/flake.nix").exists() {
        anyhow::bail!(
            "i-nix configuration already exists at {}.\nUse `i-nix apply` to apply changes, or remove the directory to reinitialize.",
            config_dir
        );
    }

    let hostname = hostname.unwrap_or_else(crate::config::hostname);
    let username = std::env::var("USER").unwrap_or_else(|_| "user".to_string());
    
    let mode = if user_only {
        INixMode::UserOnly
    } else if system || crate::config::is_nixos() {
        INixMode::System
    } else {
        INixMode::UserOnly
    };

    if verbose {
        eprintln!("{} initializing i-nix", "→".cyan());
        eprintln!("  config_dir: {}", config_dir);
        eprintln!("  hostname: {}", hostname);
        eprintln!("  username: {}", username);
        eprintln!("  mode: {:?}", mode);
    }

    if dry_run {
        println!("{} Would create i-nix configuration at {}", "DRY RUN:".yellow().bold(), config_dir);
        println!("  hostname: {}", hostname);
        println!("  username: {}", username);
        println!("  mode: {:?}", mode);
        return Ok(());
    }

    // Create the flake structure
    let engine = NixEngine::new(config_path, &hostname, &username);
    let with_home_manager = matches!(mode, INixMode::System | INixMode::UserOnly);

    // Configuration is generated via template files during init
    // No additional engine.init() needed — templates are already written
    let state = INixState {
        version: env!("CARGO_PKG_VERSION").to_string(),
        hostname: hostname.clone(),
        last_applied_generation: None,
        last_applied_time: None,
        mode,
        flake_path: config_path.join("flake"),
        has_home_manager: with_home_manager,
    };
    state.save(config_path)?;

    // Initialize git repo
    let git_init = std::process::Command::new("git")
        .arg("init")
        .current_dir(config_path.join("flake"))
        .output();
    
    if let Ok(output) = git_init {
        if output.status.success() && verbose {
            eprintln!("{} initialized git repository", "→".cyan());
        }
    }

    // Add and commit initial files
    let _ = std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(config_path.join("flake"))
        .output();
    
    let _ = std::process::Command::new("git")
        .args(["commit", "-m", "Initial i-nix configuration"])
        .current_dir(config_path.join("flake"))
        .output();

    println!();
    println!("{}", "✓ i-nix initialized successfully".green().bold());
    println!();
    println!("  Configuration directory: {}", config_dir.dimmed());
    println!("  Hostname: {}", hostname.dimmed());
    println!("  Mode: {}", format!("{:?}", mode).dimmed());
    println!();
    println!("  {}", "Next steps:".bold());
    println!("    i-nix search firefox     {}", "# Find packages".dimmed());
    println!("    i-nix install firefox      {}", "# Install a package".dimmed());
    println!("    i-nix install --user kitty {}", "# Install a user package".dimmed());
    println!("    i-nix apply                {}", "# Apply changes".dimmed());
    println!();

    Ok(())
}
