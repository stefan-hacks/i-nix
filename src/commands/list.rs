use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use crate::config::{ConfigModel, INixState};

pub async fn run(
    config_dir: &str,
    user_only: bool,
    system_only: bool,
) -> Result<()> {
    let config_path = Path::new(config_dir);
    let state = INixState::load(config_path)?;

    if state.version.is_empty() {
        anyhow::bail!("i-nix not initialized. Run `i-nix init` first.");
    }

    let username = std::env::var("USER").unwrap_or_else(|_| "user".to_string());
    let model = ConfigModel::from_flake(
        &config_path.join("flake"),
        &state.hostname,
        &username,
    )?;

    println!();
    println!("{}", "i-nix managed packages".bold().underline());
    println!();

    let show_system = !user_only;
    let show_user = !system_only;

    if show_system {
        println!("{}", "System packages:".bold());
        if model.system_packages.is_empty() {
            println!("  {} No system packages configured", "∅".dimmed());
        } else {
            for pkg in &model.system_packages {
                println!("  {} {}", "●".green(), pkg);
            }
        }
        println!();
    }

    if show_user && state.has_home_manager {
        println!("{}", "User packages (home-manager):".bold());
        if model.user_packages.is_empty() {
            println!("  {} No user packages configured", "∅".dimmed());
        } else {
            for pkg in &model.user_packages {
                println!("  {} {}", "●".cyan(), pkg);
            }
        }
        println!();
    }

    if show_system {
        println!("{}", "Enabled services:".bold());
        if model.enabled_services.is_empty() {
            println!("  {} No services configured", "∅".dimmed());
        } else {
            for svc in &model.enabled_services {
                println!("  {} {}", "◆".yellow(), svc);
            }
        }
        println!();
    }

    if show_system {
        println!("{}", "Enabled programs:".bold());
        if model.enabled_programs.is_empty() {
            println!("  {} No programs configured", "∅".dimmed());
        } else {
            for prog in &model.enabled_programs {
                println!("  {} {}", "◇".blue(), prog);
            }
        }
        println!();
    }

    Ok(())
}
