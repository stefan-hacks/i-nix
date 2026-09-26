use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// i-nix internal state — NOT a separate source of truth from Nix.
/// This file tracks metadata (last applied generation, etc.)
/// The Nix configuration remains the sole source of truth.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct INixState {
    pub version: String,
    pub hostname: String,
    pub last_applied_generation: Option<u32>,
    pub last_applied_time: Option<String>,
    pub mode: INixMode,
    pub flake_path: PathBuf,
    pub has_home_manager: bool,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, Copy)]
pub enum INixMode {
    #[default]
    System,    // Full NixOS + Home Manager
    UserOnly,  // Home Manager only (non-NixOS)
    Standalone, // Only packages, no system or home-manager
}

impl INixState {
    pub fn load(config_dir: &Path) -> Result<Self> {
        let state_path = config_dir.join("state.json");
        if state_path.exists() {
            let contents = fs::read_to_string(&state_path)
                .with_context(|| format!("reading state file: {}", state_path.display()))?;
            let state: INixState = serde_json::from_str(&contents)
                .with_context(|| "parsing state JSON")?;
            Ok(state)
        } else {
            Ok(INixState::default())
        }
    }

    pub fn save(&self, config_dir: &Path) -> Result<()> {
        fs::create_dir_all(config_dir)?;
        let state_path = config_dir.join("state.json");
        let contents = serde_json::to_string_pretty(self)
            .with_context(|| "serializing state")?;
        fs::write(&state_path, contents)
            .with_context(|| format!("writing state file: {}", state_path.display()))?;
        Ok(())
    }
}

/// Represents a declarative intent — what the user wants installed/enabled.
/// i-nix translates this into Nix AST modifications.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Intent {
    InstallPackage {
        name: String,
        attribute: String,      // nixpkgs attribute path, e.g. "nixpkgs#firefox"
        target: InstallTarget,
    },
    RemovePackage {
        name: String,
        target: InstallTarget,
    },
    EnableService {
        name: String,
        module_path: String,  // e.g. "services.openssh"
        options: HashMap<String, serde_json::Value>,
    },
    EnableProgram {
        name: String,
        module_path: String,  // e.g. "programs.firefox"
        options: HashMap<String, serde_json::Value>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InstallTarget {
    System,  // environment.systemPackages
    User,    // home.packages
}

/// Parsed representation of the current Nix configuration state.
/// Built by reading the generated Nix files (source of truth).
#[derive(Debug, Default)]
pub struct ConfigModel {
    pub system_packages: Vec<String>,
    pub user_packages: Vec<String>,
    pub enabled_services: Vec<String>,
    pub enabled_programs: Vec<String>,
    #[allow(dead_code)]
    pub custom_options: HashMap<String, serde_json::Value>,
}

impl ConfigModel {
    pub fn from_flake(flake_path: &Path, hostname: &str, username: &str) -> Result<Self> {
        let mut model = ConfigModel::default();

        // Parse systems/<host>/default.nix
        let config_path = flake_path.join(format!("systems/{}/default.nix", hostname));
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            model.system_packages = crate::nixast::extract_packages(&content, "environment.systemPackages");
        }

        // Parse users/<user>/packages.nix
        let packages_path = flake_path.join(format!("users/{}/packages/default.nix", username));
        if packages_path.exists() {
            let content = fs::read_to_string(&packages_path)?;
            model.user_packages = crate::nixast::extract_packages(&content, "home.packages");
        }

        Ok(model)
    }

    #[allow(dead_code)]
    fn parse_system_config(&mut self, _content: &str) -> Result<()> {
        Ok(())
    }

    #[allow(dead_code)]
    fn parse_home_config(&mut self, _content: &str) -> Result<()> {
        Ok(())
    }
}

/// Where generated Nix files live within the config dir.
#[allow(dead_code)]
pub fn flake_dir(config_dir: &Path) -> PathBuf {
    config_dir.join("flake")
}

/// Check if we're on NixOS
pub fn is_nixos() -> bool {
    Path::new("/etc/NIXOS").exists()
}

/// Check if nix command is available
#[allow(dead_code)]
pub fn has_nix() -> bool {
    which::which("nix").is_ok()
}

/// Check if home-manager is available
#[allow(dead_code)]
pub fn has_home_manager() -> bool {
    which::which("home-manager").is_ok()
}

/// Get the hostname
pub fn hostname() -> String {
    std::fs::read_to_string("/etc/hostname")
        .unwrap_or_else(|_| "nixos".to_string())
        .trim()
        .to_string()
}
