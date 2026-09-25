use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use crate::config::INixState;
use crate::desktop::detect::{self, detect_current, DesktopEnv};
use crate::desktop::generator::generate_de;

/// Desktop environment management.
///
/// Subcommands:
///   detect    — Detect current DE and show info
///   generate  — Generate declarative config for a DE
///   list      — List all supported DEs
///   apply     — Enable the DE in system config
pub async fn run(
    config_dir: &str,
    subcommand: Option<String>,
    de_name: Option<String>,
    from_live: bool,
    verbose: bool,
    dry_run: bool,
) -> Result<()> {
    let config_path = Path::new(config_dir);
    let state = INixState::load(config_path)?;

    if state.version.is_empty() {
        anyhow::bail!("i-nix not initialized. Run `i-nix init` first.");
    }

    let username = std::env::var("USER").unwrap_or_else(|_| "user".to_string());

    match subcommand.as_deref() {
        Some("detect") => detect_current_de(verbose).await,
        Some("generate") => {
            let de = resolve_de_name(de_name.as_deref().unwrap_or(""));
            if dry_run {
                println!(
                    "{}",
                    format!("Would generate config for: {}", de.display_name()).cyan()
                );
                return Ok(());
            }
            generate_de(&de, config_path, &username, from_live,
            ).await
        }
        Some("list") => detect::list_supported().await,
        Some("apply") => apply_de(config_path, &state.hostname, &username, verbose).await,
        _ => {
            println!();
            println!("{}", "i-nix desktop".bold().underline());
            println!();
            println!("  {}  {}", "detect".bold(), "Detect current desktop environment");
            println!("  {}  {}", "generate".bold(), "Generate declarative config for a DE");
            println!("  {}  {}", "list".bold(), "List supported desktop environments");
            println!("  {}  {}", "apply".bold(), "Enable the configured DE");
            println!();
            println!(
                "  {} Usage: i-nix desktop <subcommand> [options]",
                "→".dimmed()
            );
            println!();
            Ok(())
        }
    }
}

async fn detect_current_de(verbose: bool) -> Result<()> {
    detect::run(verbose).await
}

fn resolve_de_name(name: &str) -> DesktopEnv {
    let lower = name.to_lowercase();
    match lower.as_str() {
        "gnome" => DesktopEnv::GNOME,
        "kde" | "plasma" => DesktopEnv::KDE,
        "hyprland" => DesktopEnv::Hyprland,
        "sway" => DesktopEnv::Sway,
        "i3" => DesktopEnv::i3,
        "niri" => DesktopEnv::Niri,
        "cinnamon" => DesktopEnv::Cinnamon,
        "xfce" => DesktopEnv::XFCE,
        "popos" | "pop" | "cosmic" => DesktopEnv::PopOS,
        "quickshell" => DesktopEnv::QuickShell,
        "noctalia" => DesktopEnv::Noctalia,
        "danklinux" => DesktopEnv::DankLinux,
        "" => {
            // Auto-detect
            detect_current()
        }
        _ => DesktopEnv::Unknown(lower),
    }
}

async fn apply_de(
    config_path: &Path,
    hostname: &str,
    username: &str,
    verbose: bool,
) -> Result<()> {
    use crate::engine::NixEngine;

    let engine = NixEngine::new(config_path, hostname, username);

    let desktop_dir = config_path.join(format!("flake/users/{}/desktop", username));
    let default_nix = desktop_dir.join("default.nix");

    if !default_nix.exists() {
        anyhow::bail!(
            "No desktop configuration found. Run `i-nix desktop generate <de>` first."
        );
    }

    let content = tokio::fs::read_to_string(&default_nix).await?;

    // Detect which DE is imported
    let de_name = if content.contains("gnome-settings") {
        "GNOME"
    } else if content.contains("kde-settings") {
        "KDE Plasma"
    } else if content.contains("hyprland-settings") {
        "Hyprland"
    } else if content.contains("sway-settings") {
        "Sway"
    } else if content.contains("i3-settings") {
        "i3"
    } else if content.contains("niri-settings") {
        "Niri"
    } else if content.contains("cinnamon-settings") {
        "Cinnamon"
    } else if content.contains("xfce-settings") {
        "XFCE"
    } else if content.contains("popos-settings") {
        "Pop!_OS"
    } else if content.contains("quickshell-settings") {
        "QuickShell"
    } else if content.contains("noctalia-settings") {
        "Noctalia"
    } else if content.contains("danklinux-settings") {
        "DankLinux"
    } else {
        anyhow::bail!("Could not determine DE from desktop/default.nix");
    };

    // Enable the programs.<de>.enable in system config
    let de_program = if content.contains("hyprland") {
        "programs.hyprland"
    } else if content.contains("sway") {
        "programs.sway"
    } else if content.contains("niri") {
        "programs.niri"
    } else if content.contains("i3") {
        "programs.i3"
    } else if content.contains("quickshell") {
        "programs.quickshell"
    } else if content.contains("noctalia") {
        "programs.noctalia"
    } else if content.contains("danklinux") {
        "programs.dank-shell"
    } else if content.contains("gnome") {
        // GNOME is a service, not a program
        println!();
        println!("  {} GNOME is enabled via `services.xserver.desktopManager.gnome.enable`.", "ℹ".cyan());
        println!("  {} Add that to your system config manually or use `i-nix enable gnome --service`.", "→".dimmed());
        println!();
        return Ok(());
    } else if content.contains("kde") {
        println!();
        println!("  {} KDE Plasma is enabled via `services.xserver.desktopManager.plasma6.enable`.", "ℹ".cyan());
        println!();
        return Ok(());
    } else {
        anyhow::bail!("DE program mapping not found for: {}", de_name);
    };

    engine.enable(de_program, false)?;

    println!();
    println!("✓ Enabled {} in system config", de_name);
    println!();
    println!("  {}", "Run `i-nix apply` to activate.".dimmed());
    println!();

    Ok(())
}
