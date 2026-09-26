use anyhow::Result;
use colored::Colorize;
use std::path::Path;
use tokio::process::Command;

use crate::config::INixState;
use crate::style;

/// nh home command: Home-manager operations.
pub async fn run(
    config_dir: &str,
    subcommand: &str,
    user: &str,
    hostname: &str,
    dry_run: bool,
    verbose: bool,
) -> Result<()> {
    let config_path = Path::new(config_dir);
    let state = INixState::load(config_path)?;

    if state.version.is_empty() {
        anyhow::bail!("i-nix not initialized. Run `i-nix init` first.");
    }

    let host = if hostname.is_empty() { &state.hostname } else { hostname };
    let username = if user.is_empty() {
        std::env::var("USER").unwrap_or_else(|_| "user".to_string())
    } else {
        user.to_string()
    };

    let flake_ref = format!("{}#homeConfigurations.\"{}@{}\"", config_dir, username, host);

    style::header();
    style::section(&format!("nh home {}", subcommand));

    match subcommand {
        "switch" => {
            style::step(1, 2, "Building home configuration...");
            if dry_run {
                style::dry_run_cmd(&format!("home-manager switch --flake {}", flake_ref));
            } else {
                let mut cmd = Command::new("home-manager");
                cmd.args(["switch", "--flake", &flake_ref]);
                if verbose { cmd.arg("-v"); }
                let out = cmd.output().await?;
                if out.status.success() {
                    style::success("Home configuration activated");
                } else {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    anyhow::bail!("home-manager failed: {}", stderr);
                }
            }
        }
        "build" => {
            style::step(1, 1, "Building home configuration...");
            if dry_run {
                style::dry_run_cmd(&format!("home-manager build --flake {}", flake_ref));
            } else {
                let mut cmd = Command::new("home-manager");
                cmd.args(["build", "--flake", &flake_ref]);
                let out = cmd.output().await?;
                if out.status.success() {
                    style::success("Home configuration built");
                } else {
                    anyhow::bail!("home-manager build failed");
                }
            }
        }
        "repl" => {
            style::info("Starting Nix REPL with home configuration...");
            if dry_run {
                style::dry_run_cmd(&format!("nix repl {}", flake_ref));
            } else {
                let mut cmd = Command::new("nix");
                cmd.args(["repl", &flake_ref]);
                cmd.status().await?;
            }
        }
        _ => {
            anyhow::bail!("Unknown home subcommand: {}. Use: switch, build, repl", subcommand);
        }
    }

    Ok(())
}
