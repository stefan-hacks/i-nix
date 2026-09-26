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
    test_only: bool,
    remote: Option<String>,
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
        if let Some(ref host) = remote {
            println!("  Remote host: {}", host);
        }
        return Ok(());
    }

    let flake_dir = config_path.join("flake");
    let flake_path = format!("{}#nixosConfigurations.{}", flake_dir.display(), state.hostname);

    // Handle remote deployment
    if let Some(remote_host) = remote {
        return deploy_remote(&flake_dir, &flake_path, &remote_host, yes, verbose, dry_run).await;
    }

    // Stage all changes in git
    let _ = Command::new("git")
        .args(["-C", flake_dir.to_str().unwrap_or("."), "add", "-A"])
        .output()
        .await;

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

        if test_only {
            println!("{}", "Mode: TEST — will activate but NOT make default boot".cyan());
        }
        if build_only {
            println!("{}", "Mode: BUILD-ONLY — will build but not activate".cyan());
        }

        print!("Proceed? [Y/n] ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();
        if !input.is_empty() && input != "y" && input != "yes" {
            println!("{} cancelled.", "ℹ".blue());
            return Ok(());
        }
    }

    // Determine build/switch/test command
    let action = if test_only {
        "test"
    } else if build_only {
        "build"
    } else {
        "switch"
    };

    println!();
    println!(
        "{} {}",
        "→".cyan(),
        format!("Running nixos-rebuild {}...", action).dimmed()
    );

    let mut cmd = Command::new("nixos-rebuild");
    cmd.arg(action)
        .args(["--flake", &flake_path]);

    if verbose {
        cmd.arg("--verbose");
    }

    let output = cmd.output().await;

    match output {
        Ok(output) if output.status.success() => {
            if test_only {
                println!(
                    "{} {}",
                    "✓".green(),
                    "Configuration tested successfully".green().bold()
                );
                println!("  {} This is a test activation — it will be lost on reboot.", "ℹ".blue());
                println!("  {} Run `i-nix apply` (without --test) to make it permanent.", "→".cyan());
            } else if build_only {
                println!("{} {}", "✓".green(), "Build successful".green().bold());
            } else {
                println!("{} {}", "✓".green(), "System activated".green().bold());
            }

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
            anyhow::bail!("nixos-rebuild {} failed", action);
        }
        Err(e) => {
            // Not on NixOS
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
                    &format!("{}#{}", flake_dir.display(),
                        std::env::var("USER").unwrap_or_else(|_| "user".to_string())),
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
                }
                Err(_) => {
                    println!("  Configuration updated at: {}", flake_dir.display());
                    println!("  Run `i-nix apply` on a NixOS machine to activate.");
                }
            }
        }
    }

    // Commit
    let _ = Command::new("git")
        .args([
            "-C",
            flake_dir.to_str().unwrap_or("."),
            "commit",
            "-m",
            &format!("i-nix apply: {}", state.hostname),
        ])
        .output()
        .await;

    println!();
    println!("{}", "✓ Done.".green().bold());

    Ok(())
}

async fn deploy_remote(
    flake_dir: &std::path::Path,
    flake_path: &str,
    remote_host: &str,
    yes: bool,
    verbose: bool,
    dry_run: bool,
) -> Result<()> {
    println!();
    println!("{}", "Remote Deployment".bold().underline());
    println!();
    println!("  Host: {}", remote_host.cyan().bold());
    println!("  Flake: {}", flake_path.bright_black());
    println!();

    if dry_run {
        println!("  {} Would copy flake to remote and run nixos-rebuild", "DRY RUN".yellow().bold());
        return Ok(());
    }

    // Build locally first (nixos-rebuild build --target-host)
    // Or copy flake and build remotely
    println!("  {} Building locally for remote target...", "→".cyan());

    let mut cmd = Command::new("nixos-rebuild");
    cmd.args([
        "switch",
        "--flake", flake_path,
        "--target-host", remote_host,
        "--build-host", "localhost",
    ]);

    if verbose {
        cmd.arg("--verbose");
    }

    if !yes {
        println!();
        println!("  {}", "This will:".bold());
        println!("    1. Build the configuration locally");
        println!("    2. Copy closure to {}", remote_host);
        println!("    3. Activate on remote host");
        println!();
        println!("  Run with --yes to skip this prompt.");
        println!();
    }

    let output = cmd.output().await?;

    if output.status.success() {
        println!("  {} Deployed to {}", "✓".green().bold(), remote_host);
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Remote deployment failed: {}", stderr);
    }

    // Commit local changes
    let _ = Command::new("git")
        .args([
            "-C",
            flake_dir.to_str().unwrap_or("."),
            "commit",
            "-m",
            "i-nix remote deploy",
        ])
        .output()
        .await;

    Ok(())
}
