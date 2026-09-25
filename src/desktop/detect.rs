//! Detect the currently running Desktop Environment.

use anyhow::Result;
use colored::Colorize;
use std::env;
use std::fs;
use std::path::Path;
use tokio::process::Command;

/// A desktop environment we can detect and generate config for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopEnv {
    GNOME,
    KDE,
    Cinnamon,
    XFCE,
    PopOS,
    Hyprland,
    Sway,
    i3,
    Niri,
    QuickShell,
    Noctalia,
    DankLinux,
    Unknown(String),
}

impl DesktopEnv {
    pub fn as_str(&self) -> &str {
        match self {
            DesktopEnv::GNOME => "gnome",
            DesktopEnv::KDE => "kde",
            DesktopEnv::Cinnamon => "cinnamon",
            DesktopEnv::XFCE => "xfce",
            DesktopEnv::PopOS => "popos",
            DesktopEnv::Hyprland => "hyprland",
            DesktopEnv::Sway => "sway",
            DesktopEnv::i3 => "i3",
            DesktopEnv::Niri => "niri",
            DesktopEnv::QuickShell => "quickshell",
            DesktopEnv::Noctalia => "noctalia",
            DesktopEnv::DankLinux => "danklinux",
            DesktopEnv::Unknown(s) => s.as_str(),
        }
    }

    pub fn display_name(&self) -> String {
        match self {
            DesktopEnv::GNOME => "GNOME".to_string(),
            DesktopEnv::KDE => "KDE Plasma".to_string(),
            DesktopEnv::Cinnamon => "Cinnamon".to_string(),
            DesktopEnv::XFCE => "Xfce".to_string(),
            DesktopEnv::PopOS => "Pop!_OS COSMIC".to_string(),
            DesktopEnv::Hyprland => "Hyprland".to_string(),
            DesktopEnv::Sway => "Sway".to_string(),
            DesktopEnv::i3 => "i3".to_string(),
            DesktopEnv::Niri => "Niri".to_string(),
            DesktopEnv::QuickShell => "QuickShell".to_string(),
            DesktopEnv::Noctalia => "Noctalia".to_string(),
            DesktopEnv::DankLinux => "DankLinux Shell".to_string(),
            DesktopEnv::Unknown(s) => s.clone(),
        }
    }

    pub fn is_wayland(&self) -> bool {
        matches!(
            self,
            DesktopEnv::GNOME
                | DesktopEnv::KDE
                | DesktopEnv::Hyprland
                | DesktopEnv::Sway
                | DesktopEnv::Niri
                | DesktopEnv::QuickShell
                | DesktopEnv::Noctalia
                | DesktopEnv::DankLinux
        )
    }

    pub fn supported() -> Vec<DesktopEnv> {
        vec![
            DesktopEnv::GNOME,
            DesktopEnv::KDE,
            DesktopEnv::Cinnamon,
            DesktopEnv::XFCE,
            DesktopEnv::PopOS,
            DesktopEnv::Hyprland,
            DesktopEnv::Sway,
            DesktopEnv::i3,
            DesktopEnv::Niri,
            DesktopEnv::QuickShell,
            DesktopEnv::Noctalia,
            DesktopEnv::DankLinux,
        ]
    }
}

/// Detect the current desktop environment.
pub fn detect_current() -> DesktopEnv {
    // 1. Check XDG_CURRENT_DESKTOP (GNOME, KDE, XFCE, etc.)
    if let Ok(val) = env::var("XDG_CURRENT_DESKTOP") {
        let upper = val.to_uppercase();
        if upper.contains("GNOME") {
            return DesktopEnv::GNOME;
        }
        if upper.contains("KDE") {
            return DesktopEnv::KDE;
        }
        if upper.contains("CINNAMON") {
            return DesktopEnv::Cinnamon;
        }
        if upper.contains("XFCE") || upper.contains("X-FCE") {
            return DesktopEnv::XFCE;
        }
        if upper.contains("POP") || upper.contains("COSMIC") {
            return DesktopEnv::PopOS;
        }
    }

    // 2. Check DESKTOP_SESSION
    if let Ok(val) = env::var("DESKTOP_SESSION") {
        let lower = val.to_lowercase();
        if lower.contains("gnome") {
            return DesktopEnv::GNOME;
        }
        if lower.contains("plasma") || lower.contains("kde") {
            return DesktopEnv::KDE;
        }
        if lower.contains("cinnamon") {
            return DesktopEnv::Cinnamon;
        }
        if lower.contains("xfce") {
            return DesktopEnv::XFCE;
        }
        if lower.contains("pop") {
            return DesktopEnv::PopOS;
        }
        if lower.contains("hyprland") {
            return DesktopEnv::Hyprland;
        }
        if lower.contains("sway") {
            return DesktopEnv::Sway;
        }
        if lower.contains("i3") {
            return DesktopEnv::i3;
        }
        if lower.contains("niri") {
            return DesktopEnv::Niri;
        }
        if lower.contains("quickshell") {
            return DesktopEnv::QuickShell;
        }
        if lower.contains("noctalia") {
            return DesktopEnv::Noctalia;
        }
        if lower.contains("danklinux") {
            return DesktopEnv::DankLinux;
        }
    }

    // 3. Check WAYLAND_DISPLAY / XDG_SESSION_TYPE
    if let Ok(session) = env::var("XDG_SESSION_TYPE") {
        if session == "wayland" {
            // On wayland, check running processes for DE
            if Path::new("/proc").exists() {
                if let Ok(entries) = fs::read_dir("/proc") {
                    for entry in entries.flatten() {
                        let exe = entry.path().join("exe");
                        if let Ok(target) = fs::read_link(&exe) {
                            let path = target.to_string_lossy();
                            if path.contains("gnome-shell") {
                                return DesktopEnv::GNOME;
                            }
                            if path.contains("plasma") {
                                return DesktopEnv::KDE;
                            }
                            if path.contains("Hyprland") {
                                return DesktopEnv::Hyprland;
                            }
                            if path.contains("sway") {
                                return DesktopEnv::Sway;
                            }
                            if path.contains("niri") {
                                return DesktopEnv::Niri;
                            }
                        }
                    }
                }
            }
        }
    }

    // 4. Check for dconf database (strong GNOME signal)
    if Path::new("/home/lin/.config/dconf").exists()
        || Path::new("/home/lin/.config/dconf/user").exists()
    {
        return DesktopEnv::GNOME;
    }

    DesktopEnv::Unknown("unknown".to_string())
}

/// Show detection results to the user.
pub async fn run(verbose: bool) -> Result<()> {
    let de = detect_current();

    println!();
    println!("{}", "Desktop Environment Detection".bold().underline());
    println!();

    match &de {
        DesktopEnv::Unknown(s) => {
            println!(
                "  {} Could not detect desktop environment (reported: {})",
                "⚠".yellow(),
                s
            );
            println!();
            println!(
                "  {} Use `i-nix desktop generate <name>` to create a config manually.",
                "→".dimmed()
            );
        }
        _ => {
            println!(
                "  {} Detected: {}",
                "●".green(),
                de.display_name().bold()
            );
            println!(
                "  {} Protocol: {}",
                " ",
                if de.is_wayland() {
                    "Wayland".cyan()
                } else {
                    "X11".cyan()
                }
            );
            println!(
                "  {} Nix module: {}",
                " ",
                format!("programs.{}.enable", de.as_str()).dimmed()
            );
            println!();
            println!(
                "  {} Run `i-nix desktop generate` to create declarative config.",
                "→".dimmed()
            );
        }
    }

    if verbose {
        println!();
        println!("{}", "Environment variables:".dimmed());
        for var in &["XDG_CURRENT_DESKTOP", "DESKTOP_SESSION", "XDG_SESSION_TYPE"] {
            let val = env::var(var).unwrap_or_else(|_| "(not set)".to_string());
            println!("  {} = {}", var, val);
        }
    }

    println!();
    Ok(())
}

/// List all supported desktop environments.
pub async fn list_supported() -> Result<()> {
    println!();
    println!("{}", "Supported Desktop Environments".bold().underline());
    println!();

    let supported = DesktopEnv::supported();
    for de in supported {
        let proto = if de.is_wayland() {
            "Wayland".cyan()
        } else {
            "X11".cyan()
        };
        println!("  {:15} {} {}", de.as_str().bold(), "│".dimmed(), proto);
    }

    println!();
    println!(
        "  {}",
        "Use `i-nix desktop generate <name>` to scaffold a config.".dimmed()
    );
    println!();
    Ok(())
}
