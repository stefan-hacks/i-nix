use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::path::Path;

use crate::config::INixState;
use crate::engine::NixEngine;

/// Enable a service or program declaratively.
///
/// Examples:
///   i-nix enable ssh                    # services.openssh.enable = true;
///   i-nix enable ssh --service          # explicit: services.openssh
///   i-nix enable --user kitty           # programs.kitty.enable = true;
///   i-nix enable --user --program kitty # programs.kitty.enable = true;
///   i-nix enable docker --service       # virtualisation.docker.enable = true;
///   i-nix enable --list                 # Show known services and programs
///
/// Rules:
///   --user → programs.<name>.enable = true;   (in users/<name>/programs/default.nix)
///   --service (default) → services.<name>.enable = true; (in systems/<host>/default.nix)
///   --program (system) → programs.<name>.enable = true; (in systems/<host>/default.nix)
pub async fn run(
    config_dir: &str,
    name: Option<String>,
    user: bool,
    service: bool,
    program: bool,
    list: bool,
    verbose: bool,
    dry_run: bool,
) -> Result<()> {
    if list {
        print_known_options();
        return Ok(());
    }

    let name = match name {
        Some(n) => n,
        None => anyhow::bail!("No service or program specified. Use --list to see available options."),
    };

    let config_path = Path::new(config_dir);
    let state = INixState::load(config_path)?;

    if state.version.is_empty() {
        anyhow::bail!("i-nix not initialized. Run `i-nix init` first.");
    }

    let username = std::env::var("USER").unwrap_or_else(|_| "user".to_string());
    let engine = NixEngine::new(config_path, &state.hostname, &username);

    let module_path = if user {
        if service {
            anyhow::bail!("User-level services are not yet supported. Use --program instead.");
        }
        resolve_program_name(&name)
    } else {
        if program {
            resolve_program_name(&name)
        } else {
            resolve_service_name(&name)
        }
    };

    if verbose {
        eprintln!("  {} {} → {}", "Enable:".dimmed(), &name, &module_path);
    }

    if dry_run {
        println!("{}", format!("Would enable: {}.enable = true;", module_path).green());
        return Ok(());
    }

    engine.enable(&module_path, user)?;

    println!();
    println!("{}", "✓".green().bold());
    println!("  {} {}", "Enabled:".bold(), &name);
    println!("  {} {}", "Module:".dimmed(), &module_path);
    println!();
    println!("  {}", "Run `i-nix apply` to activate.".dimmed());
    println!();

    Ok(())
}

/// Resolve a short name to a full NixOS service path.
fn resolve_service_name(name: &str) -> String {
    match name {
        "ssh" | "openssh" => "services.openssh".to_string(),
        "docker" => "virtualisation.docker".to_string(),
        "podman" => "virtualisation.podman".to_string(),
        "bluetooth" => "hardware.bluetooth".to_string(),
        "pipewire" => "services.pipewire".to_string(),
        "pulseaudio" => "services.pulseaudio".to_string(),
        "flatpak" => "services.flatpak".to_string(),
        "fstrim" => "services.fstrim".to_string(),
        "sshd" => "services.openssh".to_string(),
        "nginx" => "services.nginx".to_string(),
        "postgresql" => "services.postgresql".to_string(),
        "mysql" | "mariadb" => "services.mysql".to_string(),
        "caddy" => "services.caddy".to_string(),
        "syncthing" => "services.syncthing".to_string(),
        "tailscale" => "services.tailscale".to_string(),
        "wireguard" => "networking.wireguard".to_string(),
        "firewall" => "networking.firewall".to_string(),
        "avahi" | "mdns" => "services.avahi".to_string(),
        "printing" | "cups" => "services.printing".to_string(),
        "gnome-keyring" => "services.gnome.gnome-keyring".to_string(),
        _ => format!("services.{}", name),
    }
}

/// Resolve a short name to a full programs path.
fn resolve_program_name(name: &str) -> String {
    match name {
        "firefox" => "programs.firefox".to_string(),
        "git" => "programs.git".to_string(),
        "neovim" | "nvim" => "programs.neovim".to_string(),
        "vim" => "programs.vim".to_string(),
        "htop" => "programs.htop".to_string(),
        "dconf" => "programs.dconf".to_string(),
        "gnupg" | "gpg" => "programs.gnupg".to_string(),
        "ssh" => "programs.ssh".to_string(),
        "thunar" => "programs.thunar".to_string(),
        "zsh" => "programs.zsh".to_string(),
        "fish" => "programs.fish".to_string(),
        "bash" => "programs.bash".to_string(),
        "starship" => "programs.starship".to_string(),
        "kitty" => "programs.kitty".to_string(),
        "alacritty" => "programs.alacritty".to_string(),
        "steam" => "programs.steam".to_string(),
        "hyprland" => "programs.hyprland".to_string(),
        "sway" => "programs.sway".to_string(),
        "waybar" => "programs.waybar".to_string(),
        "tofi" => "programs.tofi".to_string(),
        "rofi" => "programs.rofi".to_string(),
        "dmenu" => "programs.dmenu".to_string(),
        _ => format!("programs.{}", name),
    }
}

fn print_known_options() {
    println!();
    println!("{}", "Known system services:".bold().underline());
    println!();
    println!("  {}", "ssh, openssh, sshd".cyan());
    println!("  {}", "docker, podman".cyan());
    println!("  {}", "bluetooth, pipewire, pulseaudio".cyan());
    println!("  {}", "flatpak, fstrim, avahi, printing".cyan());
    println!("  {}", "nginx, postgresql, mysql, caddy".cyan());
    println!("  {}", "syncthing, tailscale, wireguard".cyan());
    println!();
    println!("{}", "Known programs:".bold().underline());
    println!();
    println!("  {}", "firefox, git, neovim, vim, htop".yellow());
    println!("  {}", "zsh, fish, bash, starship".yellow());
    println!("  {}", "kitty, alacritty".yellow());
    println!("  {}", "steam, hyprland, sway, waybar".yellow());
    println!();
    println!("  {}", "Use `i-nix enable <name>` for services (default)".dimmed());
    println!("  {}", "Use `i-nix enable --user <name>` for user programs".dimmed());
    println!();
}

/// Disable a previously enabled service or program.
pub async fn disable(
    config_dir: &str,
    name: &str,
    user: bool,
    service: bool,
    program: bool,
    verbose: bool,
    dry_run: bool,
) -> Result<()> {
    let config_path = Path::new(config_dir);
    let state = INixState::load(config_path)?;

    if state.version.is_empty() {
        anyhow::bail!("i-nix not initialized. Run `i-nix init` first.");
    }

    let username = std::env::var("USER").unwrap_or_else(|_| "user".to_string());
    let engine = NixEngine::new(config_path, &state.hostname, &username);

    let module_path = if user {
        if service {
            anyhow::bail!("User-level services are not yet supported. Use --program instead.");
        }
        resolve_program_name(name)
    } else {
        if program {
            resolve_program_name(name)
        } else {
            resolve_service_name(name)
        }
    };

    if verbose {
        eprintln!("  {} {} → {}", "Disable:".dimmed(), &name, &module_path);
    }

    if dry_run {
        println!("{}", format!("Would disable: {}.enable = true;", module_path).red());
        return Ok(());
    }

    let removed = engine.disable(&module_path, user)?;

    if removed {
        println!();
        println!("{}", "✓".red().bold());
        println!("  {} {}", "Disabled:".bold(), &name);
        println!("  {} {}", "Module:".dimmed(), &module_path);
        println!();
        println!("  {}", "Run `i-nix apply` to activate.".dimmed());
        println!();
    } else {
        println!();
        println!("  {} {}", "⚠".yellow().bold(), format!("{} was not enabled — nothing to disable.", name));
        println!();
    }

    Ok(())
}
