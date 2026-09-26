use anyhow::Result;
use colored::Colorize;
use std::path::Path;
use tokio::process::Command;

use crate::config::INixState;

/// Diagnose the Nix/NixOS/i-nix installation.
/// Inspired by `brew doctor` and `nix-health`.
pub async fn run(config_dir: &str) -> Result<()> {
    println!();
    println!("{}", "╭───────────────────────────────────────────╮".cyan());
    println!("{}", "│  i-nix doctor — system diagnostics       │".cyan().bold());
    println!("{}", "╰───────────────────────────────────────────╯".cyan());
    println!();

    let mut issues = Vec::new();
    let mut warnings = Vec::new();

    // ── Section 1: Nix Installation ──
    println!("{}", "Nix Installation".bold().underline());

    // nix command
    let nix_version = check_command("nix", &["--version"]).await;
    match nix_version {
        Ok(v) => println!("  {} {}", "✓".green(), v.trim()),
        Err(_) => {
            println!("  {} nix command not found", "✗".red());
            issues.push("Nix is not installed or not on PATH".to_string());
        }
    }

    // flakes enabled
    let flakes_enabled = check_flakes_enabled().await;
    match flakes_enabled {
        Ok(true) => println!("  {} flakes enabled", "✓".green()),
        Ok(false) => {
            println!("  {} flakes not enabled", "⚠".yellow());
            warnings.push("Add `experimental-features = nix-command flakes` to nix.conf".to_string());
        }
        Err(_) => println!("  {} could not check flake status", "?".dimmed()),
    }

    // nixpkgs channel
    let channel = check_nixpkgs_channel().await;
    match channel {
        Ok(c) => println!("  {} nixpkgs channel: {}", "✓".green(), c.dimmed()),
        Err(_) => {
            println!("  {} no nixpkgs channel configured", "⚠".yellow());
            warnings.push("Add nixpkgs channel: `nix-channel --add https://nixos.org/channels/nixpkgs-unstable nixpkgs`".to_string());
        }
    }
    println!();

    // ── Section 2: NixOS / System ──
    println!("{}", "NixOS System".bold().underline());

    let is_nixos = Path::new("/etc/NIXOS").exists();
    if is_nixos {
        let os_release = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
        if let Some(version) = os_release.lines().find(|l| l.starts_with("PRETTY_NAME")) {
            println!("  {} {}", "✓".green(), version.replace("PRETTY_NAME=", "").replace('"', ""));
        } else {
            println!("  {} NixOS detected", "✓".green());
        }

        // Check bootloader
        if Path::new("/boot/loader/entries").exists() {
            println!("  {} systemd-boot configured", "✓".green());
        } else if Path::new("/boot/grub").exists() {
            println!("  {} GRUB configured", "✓".green());
        }

        // Check generations
        let generations = count_generations().await;
        match generations {
            Ok(n) => println!("  {} {} generations available", "✓".green(), n),
            Err(_) => println!("  {} could not count generations", "?".dimmed()),
        }

        // Free space in store
        let store_space = check_store_space().await;
        match store_space {
            Ok((used, total)) => {
                let pct = (used as f64 / total as f64 * 100.0) as u64;
                if pct > 80 {
                    println!("  {} nix store {}% full ({} / {})", "⚠".yellow(), pct, used, total);
                    warnings.push(format!("nix store is {}% full — run `nix-collect-garbage -d`", pct));
                } else {
                    println!("  {} nix store {}% full", "✓".green(), pct);
                }
            }
            Err(_) => println!("  {} could not check store space", "?".dimmed()),
        }
    } else {
        println!("  {} Not running NixOS (Home Manager / nix profile mode)", "ℹ".blue());
    }
    println!();

    // ── Section 3: i-nix Configuration ──
    println!("{}", "i-nix Configuration".bold().underline());

    let config_path = Path::new(config_dir);
    if config_path.exists() {
        println!("  {} config directory exists", "✓".green());

        let state = INixState::load(config_path)?;
        println!("  {} version: {}", "✓".green(), state.version);
        println!("  {} hostname: {}", "✓".green(), state.hostname);

        if state.last_applied_generation.is_some() {
            println!("  {} last applied: generation {}", "✓".green(),
                state.last_applied_generation.unwrap());
        } else {
            println!("  {} never applied", "⚠".yellow());
            warnings.push("Run `i-nix apply` to activate your configuration".to_string());
        }

        // Check flake.nix is valid
        let flake_file = config_path.join("flake/flake.nix");
        if flake_file.exists() {
            let eval = check_flake_eval(&flake_file).await;
            match eval {
                Ok(true) => println!("  {} flake.nix evaluates", "✓".green()),
                Ok(false) => {
                    println!("  {} flake.nix has errors", "✗".red());
                    issues.push("flake.nix fails to evaluate — check syntax".to_string());
                }
                Err(_) => println!("  {} could not evaluate flake.nix", "?".dimmed()),
            }
        } else {
            println!("  {} flake.nix missing", "✗".red());
            issues.push("flake.nix not found — re-run `i-nix init`".to_string());
        }

        // Check git repo
        let git_dir = config_path.join("flake/.git");
        if git_dir.exists() {
            println!("  {} git repository tracking", "✓".green());
        } else {
            println!("  {} not a git repository", "⚠".yellow());
            warnings.push("Config directory is not versioned — run `git init`".to_string());
        }
    } else {
        println!("  {} config directory not found: {}", "✗".red(), config_dir);
        issues.push(format!("Run `i-nix init` to create {}", config_dir));
    }
    println!();

    // ── Section 4: Imperative vs Declarative ──
    println!("{}", "Imperative vs Declarative".bold().underline());

    let profile_pkgs = count_profile_packages().await;
    match profile_pkgs {
        Ok(0) => println!("  {} no imperative packages", "✓".green()),
        Ok(n) => {
            println!("  {} {} packages installed imperatively", "⚠".yellow(), n);
            warnings.push(format!("{} packages in nix profile — run `i-nix adopt` to declarativize", n));
        }
        Err(_) => println!("  {} could not check nix profile", "?".dimmed()),
    }
    println!();

    // ── Summary ──
    println!("{}", "───────────────────────────────────────────".dimmed());
    if issues.is_empty() && warnings.is_empty() {
        println!("{} {}", "✓".green().bold(), "Your system is healthy.".green().bold());
    } else {
        if !issues.is_empty() {
            println!("{} {} issue(s) found:", "✗".red().bold(), issues.len());
            for issue in &issues {
                println!("    {} {}", "→".red(), issue);
            }
        }
        if !warnings.is_empty() {
            println!("{} {} warning(s) found:", "⚠".yellow().bold(), warnings.len());
            for warning in &warnings {
                println!("    {} {}", "→".yellow(), warning);
            }
        }
    }
    println!();

    // Return error if there are critical issues
    if !issues.is_empty() {
        anyhow::bail!("{} issue(s) need attention. See above.", issues.len());
    }

    Ok(())
}

async fn check_command(cmd: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(cmd).args(args).output().await?;
    if !output.status.success() {
        anyhow::bail!("command failed");
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

async fn check_flakes_enabled() -> Result<bool> {
    let output = Command::new("nix")
        .args(["eval", "--expr", "builtins.currentSystem"])
        .output()
        .await;
    match output {
        Ok(o) if o.status.success() => Ok(true),
        _ => Ok(false),
    }
}

async fn check_nixpkgs_channel() -> Result<String> {
    let output = Command::new("nix-channel").args(["--list"]).output().await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.contains("nixpkgs") {
            return Ok(line.to_string());
        }
    }
    anyhow::bail!("no nixpkgs channel")
}

async fn count_generations() -> Result<u32> {
    let output = Command::new("nixos-rebuild")
        .args(["list-generations"])
        .output()
        .await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let count = stdout.lines().filter(|l| l.contains("current")).count() as u32;
    Ok(count.max(1))
}

async fn check_store_space() -> Result<(u64, u64)> {
    let output = Command::new("df")
        .args(["-B1", "/nix/store"])
        .output()
        .await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            if let (Ok(used), Ok(total)) = (parts[2].parse::<u64>(), parts[1].parse::<u64>()) {
                return Ok((used, total));
            }
        }
    }
    anyhow::bail!("could not parse df output")
}

async fn check_flake_eval(flake_file: &Path) -> Result<bool> {
    let output = Command::new("nix")
        .args(["eval", "--json", "--file", flake_file.to_str().unwrap_or("flake.nix"), "description"])
        .output()
        .await?;
    Ok(output.status.success())
}

async fn count_profile_packages() -> Result<usize> {
    let output = Command::new("nix")
        .args(["profile", "list"])
        .output()
        .await?;
    if !output.status.success() {
        return Ok(0);
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().filter(|l| !l.is_empty()).count())
}
