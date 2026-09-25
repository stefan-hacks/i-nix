use anyhow::Result;
use colored::Colorize;
use std::io::{self, Write};
use std::path::Path;
use tokio::process::Command;

use crate::config::INixState;

pub async fn run(
    config_dir: &str,
    yes: bool,
    build_only: bool,
    verbose: bool,
    dry_run: bool,
) -> Result<()> {
    let config_path = Path::new(config_dir);
    let state = INixState::load(config_path)?;

    if state.version.is_empty() {
        anyhow::bail!("i-nix not initialized. Run `i-nix init` first.");
    }

    let flake_dir = config_path.join("flake");
    let flake_ref = format!("{}#nixosConfigurations.{}", flake_dir.display(), state.hostname);

    if dry_run {
        println!(
            "{} Would build: {}",
            "DRY RUN:".yellow().bold(),
            flake_ref.dimmed()
        );
        return Ok(());
    }

    println!();
    println!("{}", "i-nix apply".bold().underline());
    println!();

    // Stage all changes in git
    let git_add = Command::new("git")
            .args(["-C", flake_dir.to_str().unwrap_or("."), "add", "-A"])
        .output()
        .await;

    if let Ok(output) = git_add {
        if output.status.success() && verbose {
            eprintln!("{} staged changes in git", "→".cyan());
        }
    }

    // Show what we're about to do
    println!("  Target: {}", flake_ref.dimmed());
    println!("  Hostname: {}", state.hostname.cyan());
    println!();

    if !yes {
        print!("  Proceed? [y/N/q] ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        match input.trim().to_lowercase().as_str() {
            "y" | "yes" => {}
            "q" | "quit" => {
                println!();
                println!("  {} Aborted.", "✗".red());
                return Ok(());
            }
            _ => {
                println!();
                println!("  {} Aborted.", "✗".red());
                return Ok(());
            }
        }
    }

    println!();
    println!("  {} Evaluating configuration...", "→".cyan());

    // Build the configuration
    let build_args = if build_only {
        vec![
            "build".to_string(),
            flake_ref.clone(),
            "--no-link".to_string(),
        ]
    } else {
        vec![
            "os".to_string(),
            "switch".to_string(),
            "--flake".to_string(),
            flake_dir.to_str().unwrap_or(".").to_string(),
        ]
    };

    let build_cmd = if build_only {
        Command::new("nix").args(&build_args).output().await
    } else {
        Command::new("sudo")
            .args(["nixos-rebuild"])
            .args(&build_args)
            .output()
            .await
    };

    match build_cmd {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            if output.status.success() {
                println!("  {} Build successful", "✓".green());
                
                if verbose {
                    if !stdout.is_empty() {
                        println!("{}", stdout);
                    }
                }

                if !build_only {
                    println!();
                    println!(
                        "  {} {}",
                        "✓".green().bold(),
                        "System activated successfully".green().bold()
                    );
                    println!();
                    println!(
                        "  {} {}",
                        "→".cyan(),
                        "Configuration is now declaratively managed."
                    );
                }

                // Update state
                let mut new_state = state.clone();
                new_state.last_applied_generation = Some(
                    get_current_generation().unwrap_or(0)
                );
                new_state.last_applied_time = Some(
                    chrono::Local::now().to_rfc3339()
                );
                new_state.save(config_path)?;

                // Commit to git
                let _ = Command::new("git")
                    .args([
                        "-C",
                        flake_dir.to_str().unwrap_or("."),
                        "commit",
                        "-m",
                        "i-nix apply",
                    ])
                    .output()
                    .await;
            } else {
                eprintln!();
                eprintln!("  {} Build failed", "✗".red().bold());
                eprintln!();
                if !stderr.is_empty() {
                    eprintln!("{}", stderr.red());
                }
                if !stdout.is_empty() {
                    eprintln!("{}", stdout);
                }
                anyhow::bail!("nixos-rebuild failed");
            }
        }
        Err(e) => {
            anyhow::bail!("Failed to run build command: {}", e);
        }
    }

    println!();
    Ok(())
}

fn get_current_generation() -> Option<u32> {
    // Read current generation from /nix/var/nix/profiles/system
    std::fs::read_link("/nix/var/nix/profiles/system")
        .ok()
        .and_then(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .and_then(|s| s.split('-').nth(1))
                .and_then(|n| n.parse().ok())
        })
}
