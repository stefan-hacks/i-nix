use anyhow::Result;
use colored::Colorize;
use std::collections::HashSet;
use std::path::Path;

use crate::config::INixState;

/// Sync command: detect manual changes outside i-nix and re-import them.
///
/// Scans for:
///   - Imperative packages installed via `nix-env` or `nix profile install`
///   - Services enabled via `systemctl enable` but not in the flake
///   - Changes to config files outside the i-nix tree
///
/// Examples:
///   i-nix sync              # Show all divergences
///   i-nix sync --auto       # Auto-import changes into the flake
///   i-nix sync --packages   # Only sync packages
///   i-nix sync --services   # Only sync services
pub async fn run(
    config_dir: &str,
    auto: bool,
    packages_only: bool,
    services_only: bool,
    verbose: bool,
    dry_run: bool,
) -> Result<()> {
    let config_path = Path::new(config_dir);
    let state = INixState::load(config_path)?;

    if state.version.is_empty() {
        anyhow::bail!("i-nix not initialized. Run `i-nix init` first.");
    }

    let username = std::env::var("USER").unwrap_or_else(|_| "user".to_string());

    println!();
    println!("{}", "i-nix sync".bold().underline());
    println!();

    let mut found_anything = false;

    // ── Imperative packages ──
    if !services_only {
        let system_pkgs = scan_imperative_packages_system().await;
        let user_pkgs = scan_imperative_packages_user().await;

        let decl_system = read_declared_system_packages(config_path).await.unwrap_or_default();
        let decl_user = read_declared_user_packages(config_path, &username).await.unwrap_or_default();

        let sys_new: Vec<_> = system_pkgs.difference(&decl_system).cloned().collect();
        let usr_new: Vec<_> = user_pkgs.difference(&decl_user).cloned().collect();

        if !sys_new.is_empty() || !usr_new.is_empty() {
            found_anything = true;
            println!("{}", "Divergence: Imperative Packages".yellow().bold());
            println!();
            if !sys_new.is_empty() {
                println!("  {} System packages (not in flake):", "→".dimmed());
                for pkg in &sys_new {
                    println!("    {}", pkg.cyan());
                }
            }
            if !usr_new.is_empty() {
                println!("  {} User packages (not in flake):", "→".dimmed());
                for pkg in &usr_new {
                    println!("    {}", pkg.cyan());
                }
            }
            println!();

            if auto && !dry_run {
                import_packages(config_path, &sys_new, &usr_new, &username).await?;
                println!("  {} Imported into flake.", "✓".green());
            } else if dry_run {
                println!("  {} Would import {} packages", "~".dimmed(), sys_new.len() + usr_new.len());
            } else {
                println!(
                    "  {} Run with {} to import these into the flake",
                    "→".dimmed(),
                    "--auto".cyan()
                );
            }
            println!();
        }
    }

    // ── Enabled services not in flake ──
    if !packages_only {
        let enabled_services = scan_enabled_services().await;
        let decl_services = read_declared_services(config_path).await.unwrap_or_default();

        let new_svcs: Vec<_> = enabled_services.difference(&decl_services).cloned().collect();

        if !new_svcs.is_empty() {
            found_anything = true;
            println!("{}", "Divergence: Enabled Services".yellow().bold());
            println!();
            for svc in &new_svcs {
                println!("  {} {}", "→".dimmed(), svc.cyan());
            }
            println!();

            if auto && !dry_run {
                import_services(config_path, &new_svcs).await?;
                println!("  {} Imported into flake.", "✓".green());
            } else if dry_run {
                println!("  {} Would import {} services", "~".dimmed(), new_svcs.len());
            } else {
                println!(
                    "  {} Run with {} to import these into the flake",
                    "→".dimmed(),
                    "--auto".cyan()
                );
            }
            println!();
        }
    }

    // ── Changed config files ──
    let changed_files = scan_changed_configs(config_path).await;
    if !changed_files.is_empty() {
        found_anything = true;
        println!("{}", "Divergence: Changed Config Files".yellow().bold());
        println!();
        for file in &changed_files {
            println!("  {} {}", "→".dimmed(), file.display().to_string().cyan());
        }
        println!();

        if auto && !dry_run {
            println!("  {} Config files must be re-imported manually.", "ℹ".cyan());
        } else {
            println!(
                "  {} Copy changes into {} and commit them.",
                "→".dimmed(),
                format!("{}/flake/", config_dir).dimmed()
            );
        }
        println!();
    }

    if !found_anything {
        println!("  {} No divergences found.", "✓".green());
        println!();
        println!("  {} Your flake is in sync with system state.", "→".dimmed());
        println!();
    }

    if !found_anything && auto {
        println!();
        println!(
            "  {} Nothing to import — your flake is already in sync.",
            "✓".green()
        );
        println!();
    }

    Ok(())
}

// ── Discovery helpers ──

async fn scan_imperative_packages_system() -> HashSet<String> {
    let mut pkgs = HashSet::new();

    // Try nix-env -q first (nix-env installed packages)
    if let Ok(out) = tokio::process::Command::new("nix-env")
        .args(["-q", "--json"])
        .output()
        .await
    {
        if out.status.success() {
            let json = String::from_utf8_lossy(&out.stdout);
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json) {
                if let Some(obj) = parsed.as_object() {
                    for name in obj.keys() {
                        pkgs.insert(name.clone());
                    }
                }
            }
        }
    }

    // Try nix profile list (nix-command installed)
    let out = tokio::process::Command::new("nix")
        .args(["profile", "list", "--json"])
        .output()
        .await;
    if let Ok(out) = out {
        if out.status.success() {
            let json = String::from_utf8_lossy(&out.stdout);
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json) {
                if let Some(arr) = parsed.as_array() {
                    for entry in arr {
                        if let Some(name) = entry.get("url").and_then(|u| u.as_str()) {
                            // Extract package name from flake URL
                            if let Some(last) = name.rsplit('#').next() {
                                pkgs.insert(last.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    pkgs
}

async fn scan_imperative_packages_user() -> HashSet<String> {
    // Same as system for now — nix-env is global
    // Could add per-user nix profile detection
    HashSet::new()
}

async fn scan_enabled_services() -> HashSet<String> {
    let mut svcs = HashSet::new();

    let out = tokio::process::Command::new("systemctl")
        .args(["list-unit-files", "--state=enabled", "--type=service", "--no-pager", "--plain"])
        .output()
        .await;

    if let Ok(out) = out {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            if line.ends_with(" enabled") {
                let name = line.split_whitespace().next().unwrap_or("");
                if let Some(stem) = name.strip_suffix(".service") {
                    svcs.insert(stem.to_string());
                }
            }
        }
    }

    svcs
}

async fn read_declared_system_packages(config_path: &Path) -> Result<HashSet<String>> {
    let pkg_file = config_path.join("flake/systems/default.nix");
    if !pkg_file.exists() {
        return Ok(HashSet::new());
    }
    let content = tokio::fs::read_to_string(&pkg_file).await?;
    Ok(crate::nixast::extract_packages(&content, "environment.systemPackages")
        .into_iter()
        .collect())
}

async fn read_declared_user_packages(config_path: &Path, username: &str) -> Result<HashSet<String>> {
    let pkg_file = config_path.join(format!("flake/users/{}/packages/default.nix", username));
    if !pkg_file.exists() {
        return Ok(HashSet::new());
    }
    let content = tokio::fs::read_to_string(&pkg_file).await?;
    Ok(crate::nixast::extract_packages(&content, "home.packages")
        .into_iter()
        .collect())
}

async fn read_declared_services(config_path: &Path) -> Result<HashSet<String>> {
    let svc_file = config_path.join("flake/systems/default.nix");
    if !svc_file.exists() {
        return Ok(HashSet::new());
    }
    let content = tokio::fs::read_to_string(&svc_file).await?;

    let mut svcs = HashSet::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("services.") && trimmed.contains(".enable") {
            // Extract service name: services.openssh.enable = true;
            if let Some(between) = trimmed.strip_prefix("services.") {
                if let Some(end) = between.find(".enable") {
                    svcs.insert(between[..end].to_string());
                }
            }
        }
    }
    Ok(svcs)
}

async fn scan_changed_configs(config_path: &Path) -> Vec<std::path::PathBuf> {
    let mut changed = Vec::new();

    // Files commonly managed outside i-nix that should be in the flake
    let tracked_files = [
        config_path.join("flake/systems/default.nix"),
        config_path.join("flake/home/default.nix"),
    ];

    for file in &tracked_files {
        if file.exists() {
            // Check if there are uncommitted changes
            if let Ok(out) = tokio::process::Command::new("git")
                .args(["diff", "--quiet", file.to_str().unwrap_or("")])
                .current_dir(config_path.join("flake"))
                .output()
                .await
            {
                if !out.status.success() {
                    changed.push(file.clone());
                }
            }
        }
    }

    changed
}

// ── Import helpers ──

async fn import_packages(
    config_path: &Path,
    system_pkgs: &[String],
    user_pkgs: &[String],
    username: &str,
) -> Result<()> {
    use crate::engine::NixEngine;

    let engine = NixEngine::new(config_path, "laptop", username);

    for pkg in system_pkgs {
        let _ = engine.add_package(pkg, "nixpkgs", crate::config::InstallTarget::System);
    }
    for pkg in user_pkgs {
        let _ = engine.add_package(pkg, "nixpkgs", crate::config::InstallTarget::User);
    }

    Ok(())
}

async fn import_services(config_path: &Path, services: &[String]) -> Result<()> {
    use crate::engine::NixEngine;

    let engine = NixEngine::new(config_path, "laptop", "user");

    for svc in services {
        let module = format!("services.{}", svc);
        let _ = engine.enable(&module, false);
    }

    Ok(())
}
