use anyhow::Result;
use colored::Colorize;
use std::path::Path;
use std::process::Command;

use crate::config::INixState;

pub async fn run(config_dir: &str) -> Result<()> {
    let config_path = Path::new(config_dir);
    let state = INixState::load(config_path)?;

    if state.version.is_empty() {
        anyhow::bail!("i-nix not initialized. Run `i-nix init` first.");
    }

    let flake_dir = config_path.join("flake");

    println!();
    println!("{}", "i-nix diff".bold().underline());
    println!();

    // Check git diff in the flake directory
    let diff = Command::new("git")
        .args(["-C", flake_dir.to_str().unwrap_or("."), "diff", "--stat"])
        .output();

    match diff {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.trim().is_empty() {
                println!("  {} No uncommitted changes.", "✓".green());
                println!();
                println!("  Configuration is in sync with last applied state.");
            } else {
                println!("  {} Uncommitted changes:", "●".yellow());
                println!();
                println!("{}", stdout);
                println!();
                println!("  Run {} to show full diff", format!("git -C {} diff", flake_dir.display()).dimmed());
                println!("  Run {} to apply changes", "i-nix apply".bold());
            }
        }
        _ => {
            // Git might not be initialized or no changes
            println!("  {} No diff available (git not initialized or no changes)", "ℹ".blue());
        }
    }

    println!();
    Ok(())
}
