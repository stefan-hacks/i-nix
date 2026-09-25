use anyhow::Result;
use colored::Colorize;
use std::path::Path;
use tokio::process::Command;

use crate::config::INixState;

/// Build and activate the NixOS/Home Manager configuration.
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

    if dry_run {
        println!("{}", "DRY RUN: Would apply Nix configuration".yellow().bold());
        println!("  Hostname: {}", state.hostname);
        println!("  Config dir: {}", config_dir);
        return Ok(());
    }

    let flake_dir = config_path.join("flake");
    let flake_path = format!("{}#nixosConfigurations.{}", flake_dir.display(), state.hostname);

    // Stage all changes in git
    let git_add = Command::new("git")
        .args(["-C", flake_dir.to_str().unwrap_or("."), "add", "-A"])
        .output()
        .await;

    match git_add {
        Ok(output) if output.status.success() => {
            if verbose {
                eprintln!("{} staged changes in git", "→".cyan());
            }
        }
        _ => {
            if verbose {
                eprintln!("{} git add failed (not critical)", "ℹ".blue());
            }
        }
    }

    // Show diff summary
    let diff = Command::new("git")
        .args(["-C", flake_dir.to_str().unwrap_or("."), "diff", "--stat", "--cached"])
        .output()
        .await;

    let has_changes = match diff {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            !stdout.trim().is_empty()
        }
        _ => false,
    };

    if !has_changes && !yes {
        println!("{}", "⚠ No changes to apply.".yellow());
        return Ok(());
    }

    // Confirm if not --yes
    if !yes {
        println!();
        println!("{}", "About to apply the following changes:".bold());
        println!();

        let diff_show = Command::new("git")
            .args(["-C", flake_dir.to_str().unwrap_or("."), "diff", "--cached", "--stat"])
            .output()
            .await?;
        println!("{}", String::from_utf8_lossy(&diff_show.stdout));

        print!("Proceed? [Y/n] ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();
        if !input.is_empty() && input != "y" && input != "yes" {
            println!("{} cancelled.", "ℹ".blue());
            return Ok(());
        }
    }

    // Build the NixOS configuration
    println!();
    println!("{} {}", "→".cyan(), "Evaluating Nix configuration...".dimmed());

    let mut build_cmd = Command::new("nixos-rebuild");
    build_cmd.args([
        "build",
        "--flake",
        &flake_path,
    ]);

    if verbose {
        build_cmd.arg("--verbose");
    }

    let build_output = build_cmd.output().await;

    match build_output {
        Ok(output) if output.status.success() => {
            println!("{} {}", "✓".green(), "Build successful".green().bold());

            if verbose {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if !stdout.is_empty() {
                    eprintln!("{}", stdout);
                }
            }
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("{} {}", "✗".red(), "Build failed".red().bold());
            eprintln!("{}", stderr);
            anyhow::bail!("nixos-rebuild build failed");
        }
        Err(e) => {
            // nixos-rebuild not available (not on NixOS)
            if verbose {
                eprintln!("{} nixos-rebuild not available: {}", "ℹ".blue(), e);
            }

            // Try home-manager switch as fallback
            println!();
            println!("{} {}", "→".cyan(), "Trying home-manager switch...".dimmed());

            let home_result = Command::new("home-manager")
                .args([
                    "switch",
                    "--flake",
                    &format!("{}#lin", flake_dir.display()),
                ])
                .output()
                .await;

            match home_result {
                Ok(output) if output.status.success() => {
                    println!("{} {}", "✓".green(), "Home Manager activated".green().bold());
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    eprintln!("{} {}", "✗".red(), "Home Manager failed".red().bold());
                    eprintln!("{}", stderr);
                    anyhow::bail!("home-manager switch failed");
                }
                Err(_) => {
                    println!();
                    println!("{} {}", "ℹ".blue(), "Neither nixos-rebuild nor home-manager available.".blue());
                    println!("  This usually means:");
                    println!("    - You're not on NixOS (nixos-rebuild unavailable)");
                    println!("    - Home Manager is not installed");
                    println!();
                    println!("  The configuration has been updated at: {}", flake_dir.display());
                    println!("  Run `i-nix apply` on a NixOS machine to activate.");
                }
            }
        }
    }

    if build_only {
        println!("{} Build complete — not activating (dry build)", "ℹ".blue());
        return Ok(());
    }

    // Actually switch if we have nixos-rebuild
    let switch = Command::new("nixos-rebuild")
        .args([
            "switch",
            "--flake",
            &flake_path,
        ])
        .output()
        .await;

    match switch {
        Ok(output) if output.status.success() => {
            println!("{} {}", "✓".green(), "System activated".green().bold());

            // Update state with generation info
            let gen_cmd = Command::new("nixos-rebuild")
                .args([
                    "list-generations",
                    "--json",
                ])
                .output()
                .await;

            if let Ok(gen_output) = gen_cmd {
                // Parse current generation
                // (simplified — in production would parse JSON)
            }
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("{} {}", "✗".red(), "Switch failed".red().bold());
            eprintln!("{}", stderr);
        }
        Err(_) => {
            // Not on NixOS — expected
        }
    }

    // Commit the changes
    let git_commit = Command::new("git")
        .args([
            "-C",
            flake_dir.to_str().unwrap_or("."),
            "commit",
            "-m",
            &format!("i-nix apply: {}", state.hostname),
        ])
        .output()
        .await;

    match git_commit {
        Ok(output) if output.status.success() => {
            if verbose {
                eprintln!("{} committed changes", "→".cyan());
            }
        }
        _ => {}
    }

    println!();
    println!("{}", "✓ Done.".green().bold());

    Ok(())
}
