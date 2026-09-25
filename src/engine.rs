use anyhow::Result;
use std::path::Path;

use crate::config::InstallTarget;
use crate::nixast::{PackageEntry, read_system_packages, read_home_packages, write_system_packages, write_home_packages};

/// NixEngine — the bridge between user intent and Nix configuration.
pub struct NixEngine<'a> {
    config_dir: &'a Path,
    hostname: &'a str,
    username: &'a str,
}

impl<'a> NixEngine<'a> {
    pub fn new(config_dir: &'a Path, hostname: &'a str, username: &'a str) -> Self {
        NixEngine {
            config_dir,
            hostname,
            username,
        }
    }

    fn systems_nix(&self) -> std::path::PathBuf {
        self.config_dir.join("flake/systems/default.nix")
    }

    fn home_nix(&self) -> std::path::PathBuf {
        self.config_dir.join("flake/home/default.nix")
    }

    /// Add a package to the declarative configuration.
    pub fn add_package(&self, name: &str, _attr: &str, target: InstallTarget) -> Result<()> {
        match target {
            InstallTarget::System => {
                let path = self.systems_nix();
                let mut packages = read_system_packages(&path)?;
                if !packages.contains(&name.to_string()) {
                    packages.push(name.to_string());
                }
                let entries: Vec<PackageEntry> = packages.into_iter()
                    .map(|n| PackageEntry { name: n, comment: None })
                    .collect();
                write_system_packages(&path, &entries)?;
            }
            InstallTarget::User => {
                let path = self.home_nix();
                let mut packages = read_home_packages(&path)?;
                if !packages.contains(&name.to_string()) {
                    packages.push(name.to_string());
                }
                let entries: Vec<PackageEntry> = packages.into_iter()
                    .map(|n| PackageEntry { name: n, comment: None })
                    .collect();
                write_home_packages(&path, &entries)?;
            }
        }
        Ok(())
    }

    /// Remove a package from the declarative configuration.
    pub fn remove_package(&self, name: &str, target: InstallTarget) -> Result<bool> {
        let removed = match target {
            InstallTarget::System => {
                let path = self.systems_nix();
                let mut packages = read_system_packages(&path)?;
                let before = packages.len();
                packages.retain(|p| p != name);
                let after = packages.len();
                if after < before {
                    let entries: Vec<PackageEntry> = packages.into_iter()
                        .map(|n| PackageEntry { name: n, comment: None })
                        .collect();
                    write_system_packages(&path, &entries)?;
                    true
                } else {
                    false
                }
            }
            InstallTarget::User => {
                let path = self.home_nix();
                let mut packages = read_home_packages(&path)?;
                let before = packages.len();
                packages.retain(|p| p != name);
                let after = packages.len();
                if after < before {
                    let entries: Vec<PackageEntry> = packages.into_iter()
                        .map(|n| PackageEntry { name: n, comment: None })
                        .collect();
                    write_home_packages(&path, &entries)?;
                    true
                } else {
                    false
                }
            }
        };
        Ok(removed)
    }
}
