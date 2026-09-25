use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use crate::config::{INixState, InstallTarget};
use crate::engine::NixEngine;

pub async fn run(
    config_dir: &str,
    packages: Vec<String>,
    user: bool,
    program: bool,
    attribute: Option<String>,
    verbose: bool,
    dry_run: bool,
) -> Result<()> {
    let config_path = Path::new(config_dir);
    let state = INixState::load(config_path)?;

    if state.version.is_empty() {
        anyhow::bail!(
            "i-nix not initialized. Run `i-nix init` first."
        );
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

    let mut installed = Vec::new();
    let skipped: Vec<String> = Vec::new();

    for pkg in &packages {
        // Resolve attribute: if user provided --attribute, use it; otherwise assume pkg name
        let attr = attribute.clone().unwrap_or_else(|| pkg.clone());
        
        if verbose {
            eprintln!("{} resolving {} → {}", "→".cyan(), pkg, attr.dimmed());
        }

        // Verify the package exists in nixpkgs
        let nix_attr = format!("nixpkgs#{}", attr);
        let verify = tokio::process::Command::new("nix")
            .args(["eval", "--json", &nix_attr, "--apply", "x: builtins.tryEval (x.meta.name or false)"])
            .output()
            .await;

        match verify {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if verbose {
                    eprintln!("  nix eval returned: {}", stdout.trim());
                }
            }
            _ => {
                if verbose {
                    eprintln!("  {} could not verify package in nixpkgs (will try anyway)", "⚠".yellow());
                }
            }
        }

        if dry_run {
            println!("{} Would add {} to {:?} packages", "DRY RUN:".yellow().bold(), pkg.cyan(), target);
            continue;
        }

        engine.add_package(pkg, &attr, target.clone())?;
        installed.push(pkg.clone());

        if verbose {
            eprintln!("{} added {} to configuration", "✓".green(), pkg);
        }
    }

    if !installed.is_empty() {
        println!();
        println!("{}", format!("✓ Added {} package(s)", installed.len()).green().bold());
        for pkg in &installed {
            println!("  {} {}", "→".cyan(), pkg);
        }
        println!();
        println!("  Run {} to apply changes.", "i-nix apply".bold());
        println!();
    }

    if !skipped.is_empty() {
        println!("{} Skipped {} package(s) (already present)", "ℹ".blue(), skipped.len());
    }

    Ok(())
}
