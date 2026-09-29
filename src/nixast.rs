//! Nix AST manipulation — line-based but structured.
//! Avoids regex by tracking bracket depth and context.

use anyhow::Result;
use std::fs;
use std::path::Path;

/// A package entry to write into a Nix list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageEntry {
    pub name: String,
    pub comment: Option<String>,
}

/// Parse a Nix file with rnix to validate syntax.
pub fn validate_nix_syntax(source: &str) -> Result<()> {
    let parse = rnix::Root::parse(source);
    if let Some(err) = parse.errors().first() {
        anyhow::bail!("Nix syntax error: {:?}", err);
    }
    Ok(())
}

/// Extract package identifiers from a `with pkgs; [ ... ]` list
/// by searching for a specific attribute path.
pub fn extract_packages(source: &str, attr_path: &str) -> Vec<String> {
    let mut packages = Vec::new();
    let mut in_target = false;
    let mut bracket_depth = 0;

    for line in source.lines() {
        let trimmed = line.trim();

        if !in_target {
            if trimmed.starts_with(attr_path) && trimmed.contains("=") {
                in_target = true;
                bracket_depth = 0;

                // Count brackets on this line
                for c in line.chars() {
                    if c == '[' {
                        bracket_depth += 1;
                    }
                    if c == ']' {
                        bracket_depth -= 1;
                    }
                }

                // Single-line list: attr = with pkgs; [ a b ];
                if bracket_depth <= 0 {
                    if let Some(start) = line.find('[') {
                        if let Some(end) = line.rfind(']') {
                            let inner = &line[start + 1..end];
                            for item in inner.split_whitespace() {
                                let item = item.trim();
                                if !item.is_empty()
                                    && item != "with"
                                    && item != "pkgs;"
                                    && !item.starts_with('#')
                                {
                                    packages.push(item.to_string());
                                }
                            }
                        }
                    }
                    in_target = false;
                }
                continue;
            }
        } else {
            // Inside the list
            for c in line.chars() {
                if c == '[' {
                    bracket_depth += 1;
                }
                if c == ']' {
                    bracket_depth -= 1;
                }
            }

            if bracket_depth <= 0 {
                // End of list
                if let Some(end) = line.find(']') {
                    let before = &line[..end];
                    for item in before.split_whitespace() {
                        let item = item.trim();
                        if !item.is_empty() && !item.starts_with('#') {
                            packages.push(item.to_string());
                        }
                    }
                }
                in_target = false;
                continue;
            }

            // Regular list item
            let item = trimmed.trim_end_matches(',').to_string();
            if !item.is_empty() && !item.starts_with('#') {
                packages.push(item);
            }
        }
    }

    packages
}

/// Replace or append a package list for a given attribute.
pub fn update_package_list(
    source: &str,
    attr_path: &str,
    packages: &[PackageEntry],
) -> String {
    let mut lines: Vec<String> = source.lines().map(|s| s.to_string()).collect();
    let mut in_target = false;
    let mut target_start: Option<usize> = None;
    let mut bracket_depth = 0;
    let mut found = false;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if !in_target {
            if trimmed.starts_with(attr_path) && trimmed.contains("=") {
                in_target = true;
                target_start = Some(i);
                bracket_depth = 0;
                for c in line.chars() {
                    if c == '[' {
                        bracket_depth += 1;
                    }
                    if c == ']' {
                        bracket_depth -= 1;
                    }
                }
                if bracket_depth <= 0 {
                    // Single-line: replace entire line
                    lines[i] = format_package_line(attr_path, packages);
                    in_target = false;
                    found = true;
                    break;
                }
            }
        } else {
            for c in line.chars() {
                if c == '[' {
                    bracket_depth += 1;
                }
                if c == ']' {
                    bracket_depth -= 1;
                }
            }
            if bracket_depth <= 0 {
                // End of list — replace from target_start to i
                let replacement = format_multiline_packages(attr_path, packages);
                let new_lines: Vec<String> = replacement.lines().map(|s| s.to_string()).collect();
                let start = target_start.unwrap();
                let end = i;
                lines.splice(start..=end, new_lines);
                found = true;
                break;
            }
        }
    }

    if !found {
        // Not found: append before last `}`
        if let Some(last) = lines.iter().rposition(|l| l.trim() == "}") {
            lines.insert(last, format_multiline_packages(attr_path, packages));
        } else {
            lines.push(format_multiline_packages(attr_path, packages));
        }
    }

    lines.join("\n")
}

/// Read system packages from a Nix file.
pub fn read_system_packages(path: &Path) -> Result<Vec<String>> {
    let source = fs::read_to_string(path)?;
    Ok(extract_packages(&source, "environment.systemPackages"))
}

/// Write system packages to a Nix file.
pub fn write_system_packages(path: &Path, packages: &[PackageEntry]) -> Result<()> {
    let source = fs::read_to_string(path)?;
    let new_source = update_package_list(&source, "environment.systemPackages", packages);
    validate_nix_syntax(&new_source)?;
    fs::write(path, new_source)?;
    Ok(())
}

/// Read home packages from a Nix file.
pub fn read_home_packages(path: &Path) -> Result<Vec<String>> {
    let source = fs::read_to_string(path)?;
    Ok(extract_packages(&source, "home.packages"))
}

/// Write home packages to a Nix file.
pub fn write_home_packages(path: &Path, packages: &[PackageEntry]) -> Result<()> {
    let source = fs::read_to_string(path)?;
    let new_source = update_package_list(&source, "home.packages", packages);
    validate_nix_syntax(&new_source)?;
    fs::write(path, new_source)?;
    Ok(())
}

fn format_package_line(attr_path: &str, packages: &[PackageEntry]) -> String {
    if packages.is_empty() {
        format!("  {} = with pkgs; [];", attr_path)
    } else {
        let items: Vec<String> = packages.iter().map(|p| p.name.clone()).collect();
        format!("  {} = with pkgs; [ {} ];", attr_path, items.join(" "))
    }
}

fn format_multiline_packages(attr_path: &str, packages: &[PackageEntry]) -> String {
    let mut out = format!("  {} = with pkgs; [\n", attr_path);
    for pkg in packages {
        if let Some(c) = &pkg.comment {
            out.push_str(&format!("    # {}\n", c));
        }
        out.push_str(&format!("    {}\n", pkg.name));
    }
    out.push_str("  ];\n");
    out
}

/// Add an enable line (e.g. services.openssh.enable = true;) to a Nix file.
/// Looks for a section comment, or appends before the last `}`.
pub fn add_enable_line(source: &str, module_path: &str) -> Result<String> {
    if source.contains(&format!("{}.enable", module_path)) {
        return Ok(source.to_string());
    }

    let mut lines: Vec<String> = source.lines().map(|s| s.to_string()).collect();

    let section_comment = if module_path.starts_with("services.") {
        "# ── Services ──"
    } else {
        "# ── Programs ──"
    };

    let mut insert_idx = None;
    let mut last_brace = None;

    for (i, line) in lines.iter().enumerate() {
        if line.trim() == section_comment.trim() {
            insert_idx = Some(i + 1);
        }
        if line.trim() == "}" {
            last_brace = Some(i);
        }
    }

    let enable_line = format!("  {}.enable = true;", module_path);

    if let Some(idx) = insert_idx {
        lines.insert(idx, enable_line);
    } else if let Some(idx) = last_brace {
        lines.insert(idx, enable_line);
    } else {
        lines.push(enable_line);
    }

    let new_source = lines.join("\n");
    validate_nix_syntax(&new_source)?;
    Ok(new_source)
}

/// Remove an enable line from a Nix file.
pub fn remove_enable_line(source: &str, module_path: &str) -> Result<String> {
    let target = format!("{}.enable", module_path);
    let lines: Vec<String> = source.lines().map(|s| s.to_string()).collect();
    let mut new_lines = Vec::new();
    let mut removed = false;

    for line in lines {
        if line.contains(&target) {
            removed = true;
            continue;
        }
        new_lines.push(line);
    }

    if removed {
        let new_source = new_lines.join("\n");
        validate_nix_syntax(&new_source)?;
        Ok(new_source)
    } else {
        Ok(source.to_string())
    }
}

/// Hardware-related option prefixes that belong in hardware.nix.
const HARDWARE_PREFIXES: &[&str] = &[
    "boot.loader.",
    "boot.initrd.",
    "boot.kernel.",
    "boot.extraModule",
    "boot.supportedFilesystems",
    "boot.zfs.",
    "fileSystems.",
    "swapDevices",
    "hardware.",
    "virtualisation.",
    "services.fstrim",
    "services.btrfs",
    "services.smartd",
    "powerManagement.",
    "nix.settings.max-jobs",
    "nix.settings.cores",
];

/// Network-related option prefixes that belong in network.nix.
const NETWORK_PREFIXES: &[&str] = &[
    "networking.hostName",
    "networking.hosts",
    "networking.networkmanager",
    "networking.wireless",
    "networking.wifi",
    "networking.proxy",
    "networking.useDHCP",
    "networking.interfaces",
    "networking.nameservers",
    "networking.defaultGateway",
    "networking.firewall",
    "networking.nat",
    "networking.bridges",
    "networking.vlans",
    "networking.macvlans",
    "networking.wgquick",
    "networking.wireguard",
];

/// Extract hardware-related settings from an existing NixOS configuration.nix.
/// Returns a string of the extracted settings (empty if file not found).
pub fn extract_hardware_settings(path: &Path) -> String {
    extract_by_prefixes(path, HARDWARE_PREFIXES)
}

/// Extract network-related settings from an existing NixOS configuration.nix.
pub fn extract_network_settings(path: &Path) -> String {
    extract_by_prefixes(path, NETWORK_PREFIXES)
}

/// Generic block extractor: given a Nix config file and a list of attribute
/// prefixes, extract every top-level attribute block that matches any prefix.
fn extract_by_prefixes(path: &Path, prefixes: &[&str]) -> String {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };
    let lines: Vec<&str> = content.lines().collect();
    let mut extracted = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim_start();

        // Skip comments, empty lines, and the imports block
        if trimmed.is_empty()
            || trimmed.starts_with('#')
            || trimmed.starts_with("imports")
        {
            i += 1;
            continue;
        }

        // Check if this line starts any of the target prefixes
        let is_target = prefixes.iter().any(|p| trimmed.starts_with(p));

        if !is_target {
            i += 1;
            continue;
        }

        // Capture this block — find its full extent
        let mut block = Vec::new();
        block.push(line.to_string());

        let leading_spaces = line.len() - line.trim_start().len();
        let mut j = i + 1;
        let mut brace_depth =
            trimmed.matches('{').count() as i32 - trimmed.matches('}').count() as i32;
        let mut bracket_depth =
            trimmed.matches('[').count() as i32 - trimmed.matches(']').count() as i32;
        let mut in_multiline_string = trimmed.matches("''").count() % 2 == 1;

        while j < lines.len() {
            let next_line = lines[j];
            let next_trimmed = next_line.trim_start();

            // Empty line followed by top-level attribute = end of block
            let next_spaces = next_line.len() - next_line.trim_start().len();
            if next_spaces <= leading_spaces
                && !next_trimmed.is_empty()
                && !in_multiline_string
                && brace_depth <= 0
                && bracket_depth <= 0
            {
                break;
            }

            brace_depth += next_trimmed.matches('{').count() as i32
                - next_trimmed.matches('}').count() as i32;
            bracket_depth += next_trimmed.matches('[').count() as i32
                - next_trimmed.matches(']').count() as i32;

            // Toggle multiline string state on odd counts of ''
            if next_trimmed.contains("''") {
                let count = next_trimmed.matches("''").count();
                if count % 2 == 1 {
                    in_multiline_string = !in_multiline_string;
                }
            }

            block.push(next_line.to_string());
            j += 1;
        }

        extracted.push(block.join("\n"));
        i = j;
    }

    extracted.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_packages_single_line() {
        let source = r#"{ config, pkgs, ... }: {
  environment.systemPackages = with pkgs; [ git vim ];
}"#;
        let pkgs = extract_packages(source, "environment.systemPackages");
        assert_eq!(pkgs, vec!["git", "vim"]);
    }

    #[test]
    fn test_extract_packages_multi_line() {
        let source = r#"{ config, pkgs, ... }: {
  environment.systemPackages = with pkgs; [
    git
    vim
    firefox
  ];
}"#;
        let pkgs = extract_packages(source, "environment.systemPackages");
        assert_eq!(pkgs, vec!["git", "vim", "firefox"]);
    }

    #[test]
    fn test_update_package_list() {
        let source = r#"{ config, pkgs, ... }: {
  environment.systemPackages = with pkgs; [
    git
    vim
  ];
}"#;
        let new_pkgs = vec![
            PackageEntry {
                name: "git".to_string(),
                comment: None,
            },
            PackageEntry {
                name: "firefox".to_string(),
                comment: Some("browser".to_string()),
            },
            PackageEntry {
                name: "vim".to_string(),
                comment: None,
            },
        ];
        let result = update_package_list(source, "environment.systemPackages", &new_pkgs);
        assert!(result.contains("firefox"));
        assert!(result.contains("browser"));
        assert!(result.contains("git"));
    }

    #[test]
    fn test_validate_nix_syntax() {
        let good = r#"{ pkgs }: { a = 1; }"#;
        assert!(validate_nix_syntax(good).is_ok());
    }

    #[test]
    fn test_extract_hardware_settings_luks() {
        let config = r#"
{ config, pkgs, ... }:

{
  imports = [ ./hardware-configuration.nix ];

  boot.initrd.luks.devices."nixos-enc" = {
    device = "/dev/disk/by-uuid/abc123";
    preLVM = true;
  };

  environment.systemPackages = [ vim ];
}
"#;
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp.path(), config).unwrap();
        let extracted = extract_hardware_settings(temp.path());
        assert!(extracted.contains("boot.initrd.luks"), "Should extract LUKS settings");
        assert!(
            !extracted.contains("environment.systemPackages"),
            "Should NOT extract non-hardware settings"
        );
    }

    #[test]
    fn test_extract_hardware_settings_boot_loader() {
        let config = r#"
{ config, pkgs, ... }:

{
  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;

  users.users.alice.isNormalUser = true;
}
"#;
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp.path(), config).unwrap();
        let extracted = extract_hardware_settings(temp.path());
        assert!(extracted.contains("boot.loader.systemd-boot"), "Should extract boot loader");
        assert!(
            !extracted.contains("users.users.alice"),
            "Should NOT extract user settings"
        );
    }

    #[test]
    fn test_extract_hardware_settings_fileSystems() {
        let config = r#"
{ config, pkgs, ... }:

{
  fileSystems."/" = {
    device = "/dev/disk/by-uuid/xxx";
    fsType = "ext4";
  };

  fileSystems."/boot" = {
    device = "/dev/disk/by-uuid/yyy";
    fsType = "vfat";
  };

  services.openssh.enable = true;
}
"#;
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp.path(), config).unwrap();
        let extracted = extract_hardware_settings(temp.path());
        assert!(extracted.contains("fileSystems.\"/\""), "Should extract root filesystem");
        assert!(extracted.contains("fileSystems.\"/boot\""), "Should extract boot filesystem");
        assert!(
            !extracted.contains("services.openssh"),
            "Should NOT extract ssh service"
        );
    }

    #[test]
    fn test_extract_network_settings_real_config() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let config_path = temp.path();
        std::fs::write(
            config_path,
            r#"{ config, pkgs, ... }:
{
  imports = [ ./hardware-configuration.nix ];

  boot.loader.grub.enable = true;
  boot.loader.grub.device = "/dev/vda";

  networking.hostName = "nixvm";
  networking.networkmanager.enable = true;

  time.timeZone = "America/Sao_Paulo";
}
"#,
        )
        .unwrap();

        let hw = extract_hardware_settings(config_path);
        assert!(
            hw.contains("boot.loader.grub"),
            "must extract boot.loader settings"
        );
        assert!(
            !hw.contains("networking.hostName"),
            "must NOT include hostName in hardware"
        );

        let net = extract_network_settings(config_path);
        assert!(
            net.contains("networking.hostName = \"nixvm\""),
            "must extract hostName"
        );
        assert!(
            net.contains("networking.networkmanager.enable = true"),
            "must extract networkmanager"
        );
        assert!(
            !net.contains("boot.loader"),
            "must NOT include boot in network"
        );
    }

    #[test]
    fn test_extract_hardware_settings_not_found() {
        let result = extract_hardware_settings(Path::new("/nonexistent/path"));
        assert!(result.is_empty(), "Should return empty string for missing file");
    }
}
