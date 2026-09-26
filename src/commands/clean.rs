use anyhow::Result;
use colored::Colorize;
use tokio::process::Command;

use crate::style;

/// nh clean command: Enhanced nix cleanup.
pub async fn run(
    subcommand: &str,
    profile: Option<String>,
    dry_run: bool,
    verbose: bool,
) -> Result<()> {
    style::header();
    style::section(&format!("nh clean {}", subcommand));

    match subcommand {
        "all" => {
            style::step(1, 2, "Collecting garbage...");
            if dry_run {
                style::dry_run_cmd("nix-collect-garbage -d");
                style::info("Would delete all unreachable store paths");
            } else {
                let out = Command::new("nix-collect-garbage")
                    .args(["-d"])
                    .output()
                    .await?;
                if out.status.success() {
                    style::success("Garbage collection complete");
                } else {
                    anyhow::bail!("nix-collect-garbage failed");
                }
            }
        }
        "user" => {
            style::step(1, 2, "Cleaning user profiles...");
            if dry_run {
                style::dry_run_cmd("nix-collect-garbage");
            } else {
                let out = Command::new("nix-collect-garbage")
                    .output()
                    .await?;
                if out.status.success() {
                    style::success("User profiles cleaned");
                } else {
                    anyhow::bail!("Cleanup failed");
                }
            }
        }
        "profile" => {
            if let Some(profile_name) = profile {
                style::step(1, 2, &format!("Cleaning profile: {}", profile_name));
                if dry_run {
                    style::dry_run_cmd(&format!("nix profile remove {} --profile {}", profile_name, profile_name));
                } else {
                    style::info("Profile cleanup not yet implemented");
                }
            } else {
                anyhow::bail!("--profile required for 'clean profile'");
            }
        }
        _ => {
            anyhow::bail!("Unknown clean subcommand: {}. Use: all, user, profile", subcommand);
        }
    }

    Ok(())
}
