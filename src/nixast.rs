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
}
