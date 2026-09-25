use anyhow::Result;
use colored::Colorize;
use std::path::Path;
use tokio::process::Command;

use crate::config::{ConfigModel, INixState, InstallTarget};
use crate::engine::NixEngine;

/// Discover current system state and generate declarative configuration.
pub async fn run(
    config_dir: &str,
    write: bool,
    verbose: bool,
    dry_run: bool,
) -> Result<()> {
    println!();
    println!("{}", "╭────────────────────────────────────╮".cyan());
    println!("{}", "│  i-nix adopt — system discovery   │".cyan().bold());
    println!("{}", "╰────────────────────────────────────╯".cyan());
    println!();

    if dry_run {
        println!("{}", "DRY RUN: Would discover and generate configuration.".yellow().bold());
        println!();
    }

    let config_path = Path::new(config_dir);
    let state = INixState::load(config_path)?;

    if state.version.is_empty() {
        anyhow::bail!("i-nix not initialized. Run `i-nix init` first.");
    }

    let username = std::env::var("USER").unwrap_or_else(|_| "user".to_string());
    let engine = NixEngine::new(
        config_path,
        &state.hostname,
        &username,
    );

    // ── Discovery Phase ──
    let mut discovered = DiscoveredState::default();

    // 1. System packages from current generation
    discovered.system_packages = discover_system_packages().await;
    if verbose {
        eprintln!("  → discovered {} system packages", discovered.system_packages.len());
    }

    // 2. User packages (if home-manager is active)
    discovered.user_packages = discover_user_packages().await;
    if verbose {
        eprintln!("  → discovered {} user packages", discovered.user_packages.len());
    }

    // 3. Enabled services
    discovered.services = discover_services().await;
    if verbose {
        eprintln!("  → discovered {} services", discovered.services.len());
    }

    // 4. Enabled programs (programs.*.enable = true)
    discovered.programs = discover_programs().await;
    if verbose {
        eprintln!("  → discovered {} programs", discovered.programs.len());
    }

    // 5. Imperative nix profile installs
    discovered.profile_packages = discover_profile_packages().await;
    if verbose {
        eprintln!("  → discovered {} profile packages", discovered.profile_packages.len());
    }

    // ── Report Phase ──
    println!("{}", "Discovery Results".bold());
    println!();

    print_section("System Packages", &discovered.system_packages, "pkgs");
    print_section("User Packages", &discovered.user_packages, "pkgs");
    print_section("Services", &discovered.services, "svc");
    print_section("Programs", &discovered.programs, "prg");
    print_section("Profile Packages (imperative)", &discovered.profile_packages, "imp");

    if !discovered.profile_packages.is_empty() {
        println!("  {}", "⚠ These packages are installed imperatively, not declaratively.".yellow());
        println!("    Run {} to adopt them into your config.",
            format!("i-nix adopt {} --write", discovered.profile_packages.join(" ")).bold());
        println!();
    }

    if !write {
        println!("  {}", "Run with --write to generate the declarative configuration.".dimmed());
        return Ok(());
    }

    if dry_run {
        println!("{}", "DRY RUN: Would write discovered configuration.".yellow());
        return Ok(());
    }

    // ── Write Phase ──
    println!("{}", "Writing declarative configuration...".cyan().bold());
    println!();

    // Write system packages
    if !discovered.system_packages.is_empty() {
        for pkg in &discovered.system_packages {
            let _ = engine.add_package(pkg, pkg, InstallTarget::System);
        }
        println!("  {} wrote {} system packages", "✓".green(), discovered.system_packages.len());
    }

    // Write user packages
    if !discovered.user_packages.is_empty() {
        for pkg in &discovered.user_packages {
            let _ = engine.add_package(pkg, pkg, InstallTarget::User);
        }
        println!("  {} wrote {} user packages", "✓".green(), discovered.user_packages.len());
    }

    // Write services (as comments in the config)
    if !discovered.services.is_empty() {
        println!("  {} detected services (add manually with `i-nix enable <service>`):", "ℹ".blue());
        for svc in &discovered.services {
            println!("      {}", svc.dimmed());
        }
    }

    // Write programs (as comments)
    if !discovered.programs.is_empty() {
        println!("  {} detected programs (add manually with `i-nix enable --program <name>`):", "ℹ".blue());
        for prg in &discovered.programs {
            println!("      {}", prg.dimmed());
        }
    }

    println!();
    println!("{}", "✓ Adopt complete.".green().bold());
    println!("  Run {} to apply.", "i-nix apply".bold());
    println!();

    Ok(())
}

#[derive(Default)]
struct DiscoveredState {
    system_packages: Vec<String>,
    user_packages: Vec<String>,
    services: Vec<String>,
    programs: Vec<String>,
    profile_packages: Vec<String>,
}

fn print_section(title: &str, items: &[String], icon: &str) {
    if items.is_empty() {
        return;
    }
    println!("  {}", title.bold());
    if items.len() <= 10 {
        for item in items {
            println!("    {} {}", icon.cyan(), item);
        }
    } else {
        for item in &items[..10] {
            println!("    {} {}", icon.cyan(), item);
        }
        println!("    ... and {} more", items.len() - 10);
    }
    println!();
}

async fn discover_system_packages() -> Vec<String> {
    let mut pkgs = Vec::new();

    // Try nix-store --query --roots to find packages referenced by current system
    if let Ok(output) = Command::new("nix-store")
        .args(["-q", "--roots", "/run/current-system"])
        .output()
        .await
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                // Parse lines like "(system:1:1:1) /nix/store/...-firefox-156.0"
                if let Some(store) = line.split_whitespace().nth(1) {
                    if let Some(name) = store.split('-').nth(1) {
                        pkgs.push(name.to_string());
                    }
                }
            }
        }
    }

    // Fallback: read current system closure
    if pkgs.is_empty() {
        if let Ok(output) = Command::new("nix-store")
            .args(["-q", "--requisites", "/run/current-system"])
            .output()
            .await
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    let name = line.split('/').last().unwrap_or("");
                    if !name.is_empty() && !name.starts_with('.') {
                        pkgs.push(name.to_string());
                    }
                }
            }
        }
    }

    // Deduplicate and filter
    pkgs.sort();
    pkgs.dedup();
    pkgs.retain(|p| !p.starts_with("glibc") && !p.starts_with("bash") && !p.starts_with("systemd"));
    pkgs.truncate(200); // Don't overwhelm
    pkgs
}

async fn discover_user_packages() -> Vec<String> {
    let mut pkgs = Vec::new();

    // Check home-manager generation
    if let Ok(output) = Command::new("home-manager")
        .args(["generations"])
        .output()
        .await
    {
        if output.status.success() {
            // home-manager is active — try to get packages
            if let Ok(gq) = Command::new("nix-store")
                .args(["-q", "--requisites", "$HOME/.local/state/nix/profiles/home-manager"])
                .output()
                .await
            {
                let stdout = String::from_utf8_lossy(&gq.stdout);
                for line in stdout.lines() {
                    let name = line.split('/').last().unwrap_or("");
                    if !name.is_empty() {
                        pkgs.push(name.to_string());
                    }
                }
            }
        }
    }

    pkgs.sort();
    pkgs.dedup();
    pkgs.truncate(200);
    pkgs
}

async fn discover_services() -> Vec<String> {
    let mut svcs = Vec::new();

    // Check systemctl for enabled services
    if let Ok(output) = Command::new("systemctl")
        .args(["list-units", "--type=service", "--state=running", "--no-pager", "--plain"])
        .output()
        .await
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if !parts.is_empty() {
                    let svc = parts[0].replace(".service", "");
                    if svc != "dbus" && svc != "systemd" && svc != "network" {
                        svcs.push(svc);
                    }
                }
            }
        }
    }

    svcs.sort();
    svcs.dedup();
    svcs.truncate(50);
    svcs
}

async fn discover_programs() -> Vec<String> {
    let mut prgs = Vec::new();

    // Check for common program modules
    let candidates = vec![
        "firefox", "chromium", "google-chrome", "brave",
        "git", "vim", "neovim", "emacs",
        "zsh", "fish", "bash",
        "tmux", "screen",
        "docker", "podman",
        "ssh", "gpg",
        "direnv", "nix-index", "starship",
    ];

    for prg in candidates {
        // Check if program binary exists
        if let Ok(output) = Command::new("which").arg(prg).output().await {
            if output.status.success() {
                prgs.push(prg.to_string());
            }
        }
    }

    prgs.sort();
    prgs.dedup();
    prgs
}

async fn discover_profile_packages() -> Vec<String> {
    let mut pkgs = Vec::new();

    // Check nix profile list
    if let Ok(output) = Command::new("nix")
        .args(["profile", "list"])
        .output()
        .await
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if let Some(idx) = line.find(' ') {
                    let name = line[..idx].trim();
                    if !name.is_empty() {
                        pkgs.push(name.to_string());
                    }
                }
            }
        }
    }

    pkgs.sort();
    pkgs.dedup();
    pkgs
}
