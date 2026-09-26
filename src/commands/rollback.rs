use anyhow::Result;
use colored::Colorize;
use std::io::{self, Write};
use std::path::Path;
use tokio::process::Command;

use crate::config::INixState;

/// Rollback with interactive generation selection.
pub async fn run(config_dir: &str, interactive: bool, generation: Option<u32>) -> Result<()> {
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

    // Get current generation
    let current_gen = get_current_generation();
    if let Some(generation) = current_gen {
        println!("  Current generation: {}", generation.to_string().cyan().bold());
    }

    // List generations with parsing
    let generations = list_generations().await?;

    if generations.is_empty() {
        println!("  {} No generations found.", "⚠".yellow());
        return Ok(());
    }

    println!();
    println!("{}", "Available generations:".bold());
    println!();

    for (i, generation) in generations.iter().enumerate() {
        let marker = if Some(generation.number) == current_gen {
            "●".green().bold()
        } else {
            " ".into()
        };
        println!(
            "  {} {:>3} │ {} │ {} │ {}",
            marker,
            generation.number,
            generation.date.bright_black(),
            generation.nixos_version.bright_black(),
            generation.kernel.bright_black()
        );
    }

    // If a specific generation was requested
    if let Some(target_generation) = generation {
        println!();
        println!(
            "  {} Rolling back to generation {}",
            "→".cyan(),
            target_generation.to_string().cyan().bold()
        );

        let result = Command::new("sudo")
            .args([
                "nixos-rebuild",
                "switch",
                "--generation",
                &target_generation.to_string(),
            ])
            .output()
            .await?;

        if result.status.success() {
            println!("  {} Rolled back to generation {}", "✓".green(), target_generation);
        } else {
            let stderr = String::from_utf8_lossy(&result.stderr);
            anyhow::bail!("Rollback failed: {}", stderr);
        }
        return Ok(());
    }

    // Interactive mode
    if interactive {
        println!();
        print!("  Enter generation number to rollback to (or 'q' to quit): ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input == "q" || input.is_empty() {
            println!("  {} Cancelled.", "ℹ".blue());
            return Ok(());
        }

        let target_generation: u32 = input.parse().map_err(|_| anyhow::anyhow!("Invalid generation number"))?;

        // Verify generation exists
        if !generations.iter().any(|g| g.number == target_generation) {
            anyhow::bail!("Generation {} not found", target_generation);
        }

        println!();
        println!(
            "  {} Rolling back to generation {}...",
            "→".cyan(),
            target_generation.to_string().cyan().bold()
        );

        let result = Command::new("sudo")
            .args([
                "nixos-rebuild",
                "switch",
                "--generation",
                &target_generation.to_string(),
            ])
            .output()
            .await?;

        if result.status.success() {
            println!("  {} Rolled back to generation {}", "✓".green().bold(), target_generation);
        } else {
            let stderr = String::from_utf8_lossy(&result.stderr);
            anyhow::bail!("Rollback failed: {}", stderr);
        }
    } else {
        // Default: show how to rollback
        println!();
        println!("  {}", "Usage:".bold());
        println!("    i-nix rollback -i              # Interactive selection");
        println!("    i-nix rollback -g <number>     # Rollback to specific generation");
        println!("    sudo nixos-rebuild switch --rollback  # Immediate previous");
    }

    println!();
    Ok(())
}

#[derive(Debug)]
struct Generation {
    number: u32,
    date: String,
    nixos_version: String,
    kernel: String,
    is_current: bool,
}

async fn list_generations() -> Result<Vec<Generation>> {
    let output = Command::new("nixos-rebuild")
        .args(["list-generations"])
        .output()
        .await;

    let mut generations = Vec::new();

    if let Ok(output) = output {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                // Parse: "  42   2026-09-25 14:30:21   24.05   6.12.0"
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    if let Ok(num) = parts[0].parse::<u32>() {
                        generations.push(Generation {
                            number: num,
                            date: parts[1..3].join(" "),
                            nixos_version: parts.get(3).unwrap_or(&"").to_string(),
                            kernel: parts.get(4).unwrap_or(&"").to_string(),
                            is_current: false,
                        });
                    }
                }
            }
        }
    }

    // Fallback: parse from /nix/var/nix/profiles/
    if generations.is_empty() {
        if let Ok(entries) = std::fs::read_dir("/nix/var/nix/profiles") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if let Some(num_str) = name.strip_prefix("system-") {
                    if let Some(num_str) = num_str.strip_suffix("-link") {
                        if let Ok(num) = num_str.parse::<u32>() {
                            let meta = std::fs::metadata(entry.path()).ok();
                            let date = meta
                                .and_then(|m| m.modified().ok())
                                .and_then(|t| {
                                    let datetime: chrono::DateTime<chrono::Local> = t.into();
                                    Some(datetime.format("%Y-%m-%d %H:%M:%S").to_string())
                                })
                                .unwrap_or_else(|| "unknown".to_string());

                            generations.push(Generation {
                                number: num,
                                date,
                                nixos_version: "unknown".to_string(),
                                kernel: "".to_string(),
                                is_current: false,
                            });
                        }
                    }
                }
            }
            generations.sort_by_key(|g| g.number);
            generations.reverse(); // Newest first
        }
    }

    Ok(generations)
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
