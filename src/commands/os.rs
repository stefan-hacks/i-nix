use anyhow::Result;
use colored::Colorize;
use std::path::Path;
use tokio::process::Command;

use crate::config::INixState;
use crate::style;

/// nh os command: NixOS system operations.
pub async fn run(
    config_dir: &str,
    subcommand: &str,
    hostname: &str,
    specialisation: Option<String>,
    no_specialisation: bool,
    dry_run: bool,
    verbose: bool,
) -> Result<()> {
    let config_path = Path::new(config_dir);
    let state = INixState::load(config_path)?;

    if state.version.is_empty() {
        anyhow::bail!("i-nix not initialized. Run `i-nix init` first.");
    }

    let host = if hostname.is_empty() { &state.hostname } else { hostname };
    let flake_ref = format!("{}#nixosConfigurations.{}" , config_dir, host);

    style::header();
    style::section(&format!("nh os {}", subcommand));

    match subcommand {
        "switch" => {
            style::step(1, 3, "Building NixOS configuration...");
            if dry_run {
                style::dry_run_cmd(&format!("nixos-rebuild switch --flake {}", flake_ref));
                style::info("Would activate and set as boot default");
            } else {
                let mut cmd = Command::new("nixos-rebuild");
                cmd.args(["switch", "--flake", &flake_ref]);
                if verbose { cmd.arg("-v"); }
                let out = cmd.output().await?;
                if out.status.success() {
                    style::success("System switched successfully");
                } else {
                    anyhow::bail!("nixos-rebuild failed");
                }
            }
        }
        "boot" => {
            style::step(1, 2, "Building boot configuration...");
            if dry_run {
                style::dry_run_cmd(&format!("nixos-rebuild boot --flake {}", flake_ref));
            } else {
                let mut cmd = Command::new("nixos-rebuild");
                cmd.args(["boot", "--flake", &flake_ref]);
                let out = cmd.output().await?;
                if out.status.success() {
                    style::success("Boot configuration set");
                } else {
                    anyhow::bail!("nixos-rebuild boot failed");
                }
            }
        }
        "test" => {
            style::step(1, 2, "Testing configuration (no boot default)...");
            if dry_run {
                style::dry_run_cmd(&format!("nixos-rebuild test --flake {}", flake_ref));
            } else {
                let mut cmd = Command::new("nixos-rebuild");
                cmd.args(["test", "--flake", &flake_ref]);
                let out = cmd.output().await?;
                if out.status.success() {
                    style::success("Configuration tested successfully");
                } else {
                    anyhow::bail!("nixos-rebuild test failed");
                }
            }
        }
        "build" => {
            style::step(1, 1, "Building configuration...");
            if dry_run {
                style::dry_run_cmd(&format!("nixos-rebuild build --flake {}", flake_ref));
            } else {
                let mut cmd = Command::new("nixos-rebuild");
                cmd.args(["build", "--flake", &flake_ref]);
                let out = cmd.output().await?;
                if out.status.success() {
                    style::success("Build complete");
                } else {
                    anyhow::bail!("nixos-rebuild build failed");
                }
            }
        }
        "info" => {
            style::subsection("System generations");
            if !crate::config::is_nixos() {
                style::warning("Not running on NixOS");
                return Ok(());
            }
            let out = Command::new("nixos-rebuild")
                .args(["list-generations"])
                .output()
                .await?;
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                for line in stdout.lines().take(15) {
                    println!("    {}", line);
                }
            }
        }
        "repl" => {
            style::info("Starting Nix REPL with system configuration...");
            if dry_run {
                style::dry_run_cmd(&format!("nix repl {}", flake_ref));
            } else {
                let mut cmd = Command::new("nix");
                cmd.args(["repl", &flake_ref]);
                cmd.status().await?;
            }
        }
        _ => {
            anyhow::bail!("Unknown os subcommand: {}. Use: switch, boot, test, build, info, repl", subcommand);
        }
    }

    Ok(())
}
