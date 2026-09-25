//! GNOME dconf → Nix declarative config generator.
//!
//! Port of nix-my-gnome's parser into native Rust.
//! Generates well-organized, split output (equivalent to `nmg -s`).

use anyhow::Result;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

/// A single dconf section (e.g. [org/gnome/desktop/interface]).
#[derive(Debug, Clone)]
pub struct DconfSection {
    pub path: String,
    pub settings: Vec<(String, String)>, // (key, nix_value)
}

/// Parse a dconf dump string into sections.
pub fn parse_dconf_dump(input: &str) -> Vec<DconfSection> {
    let mut sections: Vec<DconfSection> = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_settings: Vec<(String, String)> = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        // Section header: [org/gnome/desktop/interface]
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            // Save previous section
            if let Some(ref path) = current_path {
                sections.push(DconfSection {
                    path: path.clone(),
                    settings: std::mem::take(&mut current_settings),
                });
            }
            current_path = Some(trimmed[1..trimmed.len() - 1].to_string());
            continue;
        }

        // Key-value: key=value
        if let Some(pos) = trimmed.find('=') {
            if let Some(ref path) = current_path {
                let key = trimmed[..pos].trim().to_string();
                let raw_val = trimmed[pos + 1..].trim();
                let nix_val = gvariant_to_nix(raw_val);
                current_settings.push((key, nix_val));
            }
        }
    }

    // Save last section
    if let Some(ref path) = current_path {
        if !current_settings.is_empty() {
            sections.push(DconfSection {
                path: path.clone(),
                settings: current_settings,
            });
        }
    }

    sections
}

/// Convert a GVariant text value to a Nix expression string.
fn gvariant_to_nix(raw: &str) -> String {
    let val = raw.trim();

    // Empty string
    if val.is_empty() {
        return r#"""#.to_string();
    }

    // Single-quoted string
    if val.starts_with('\'') && val.ends_with('\'') && val.len() > 1 {
        let inner = &val[1..val.len() - 1];
        let escaped = inner.replace("\\", "\\\\").replace('"', "\\\"").replace("${", "\\${");
        return format!(r#""{}""#, escaped);
    }

    // Double-quoted string
    if val.starts_with('"') && val.ends_with('"') && val.len() > 1 {
        let inner = &val[1..val.len() - 1];
        let escaped = inner.replace("\\", "\\\\").replace('"', "\\\"").replace("${", "\\${");
        return format!(r#""{}""#, escaped);
    }

    // Typed values: uint32 1, int64 -5, double 3.14
    let typed_re = regex::Regex::new(
        r"^(uint64|uint32|uint16|int64|int32|int16|byte|handle|double)\s+(.+)$"
    ).unwrap();
    if let Some(caps) = typed_re.captures(val) {
        let type_name = caps.get(1).unwrap().as_str();
        let inner = caps.get(2).unwrap().as_str();
        let mk_fn = match type_name {
            "uint64" => "mkUint64",
            "uint32" => "mkUint32",
            "uint16" => "mkUint16",
            "int64" => "mkInt64",
            "int32" => "mkInt32",
            "int16" => "mkInt16",
            "byte" => "mkUint8",
            "handle" => "mkHandle",
            "double" => "mkDouble",
            _ => type_name,
        };
        let inner_nix = gvariant_to_nix(inner);
        return format!("(lib.hm.gvariant.{} {})", mk_fn, inner_nix);
    }

    // @type annotation: @as [], @a{sv} {}
    if val.starts_with('@') {
        // Skip the type token and parse the value
        if let Some(space_pos) = val.find(' ') {
            let actual = &val[space_pos + 1..];
            return gvariant_to_nix(actual);
        }
        return "[ ]".to_string(); // @as []
    }

    // Array [ ... ]
    if val.starts_with('[') && val.ends_with(']') {
        let inner = &val[1..val.len() - 1].trim();
        if inner.is_empty() {
            return "[ ]".to_string();
        }
        let items: Vec<String> = inner.split(',').map(|s| gvariant_to_nix(s.trim())).collect();
        return format!("[ {} ]", items.join(" "));
    }

    // Tuple ( ... )
    if val.starts_with('(') && val.ends_with(')') {
        let inner = &val[1..val.len() - 1].trim();
        let items: Vec<String> = inner.split(',').map(|s| gvariant_to_nix(s.trim())).collect();
        return format!("(lib.hm.gvariant.mkTuple [ {} ])", items.join(" "));
    }

    // Dict { ... }
    if val.starts_with('{') && val.ends_with('}') {
        let inner = &val[1..val.len() - 1].trim();
        if inner.is_empty() {
            return "[ ]".to_string();
        }
        let entries: Vec<String> = inner.split(',').map(|s| {
            let kv: Vec<&str> = s.splitn(2, ':').collect();
            if kv.len() == 2 {
                let k = gvariant_to_nix(kv[0].trim());
                let v = gvariant_to_nix(kv[1].trim());
                format!("(lib.hm.gvariant.mkDictionaryEntry [ {} {} ])", k, v)
            } else {
                "(lib.hm.gvariant.mkDictionaryEntry [ \"error\" \"\" ])".to_string()
            }
        }).collect();
        return format!("[ {} ]", entries.join(" "));
    }

    // Variant < ... >
    if val.starts_with('<') && val.ends_with('>') {
        let inner = &val[1..val.len() - 1].trim();
        let inner_nix = gvariant_to_nix(inner);
        return format!("(lib.hm.gvariant.mkVariant {})", inner_nix);
    }

    // Boolean
    if val == "true" || val == "false" {
        return val.to_string();
    }

    // Negative number (needs parentheses in Nix)
    if val.starts_with('-') {
        return format!("({})", val);
    }

    // Default: return as-is
    val.to_string()
}

/// Categorize a dconf section path into a file stem.
fn categorize(section: &str) -> &str {
    if section.starts_with("org/gnome/shell/extensions/") {
        "shell-extensions"
    } else if section.starts_with("org/gnome/shell/") {
        "shell"
    } else if section.starts_with("org/gnome/mutter/") {
        "mutter"
    } else if section.starts_with("org/gnome/desktop/wm/") {
        "window-manager"
    } else if section.starts_with("org/gnome/desktop/notifications/") {
        "notifications"
    } else if section.starts_with("org/gnome/desktop/a11y/") {
        "accessibility"
    } else if section.starts_with("org/gnome/desktop/peripherals/")
        || section.starts_with("org/gnome/desktop/input-sources")
        || section.starts_with("desktop/ibus/")
    {
        "input-devices"
    } else if section.starts_with("org/gnome/desktop/interface")
        || section.starts_with("org/gnome/desktop/background")
        || section.starts_with("org/gnome/desktop/sound")
        || section.starts_with("org/gnome/desktop/session")
        || section.starts_with("org/gtk/")
    {
        "gtk"
    } else if section.starts_with("org/gnome/desktop/app-folders") {
        "app-folders"
    } else if section.starts_with("org/gnome/nautilus/") {
        "nautilus"
    } else if section.starts_with("org/gnome/evolution")
        || section.starts_with("org/gnome/Contacts")
        || section.starts_with("org/freedesktop/folks")
    {
        "evolution"
    } else if section.starts_with("org/virt-manager/") {
        "virt-manager"
    } else if section.starts_with("org/gnome/settings-daemon/")
        || section.starts_with("org/gnome/control-center")
        || section.starts_with("org/gnome/portal/")
    {
        "settings-daemon"
    } else if section.starts_with("org/gnome/clocks")
        || section.starts_with("org/gnome/Weather")
        || section.starts_with("org/gnome/GWeather4")
        || section.starts_with("org/gnome/Music")
        || section.starts_with("de/haeckerfelix/Shortwave")
        || section.starts_with("org/gnome/Showtime")
        || section.starts_with("org/soundconverter")
        || section.starts_with("org/nickvision/")
    {
        "media"
    } else if section.starts_with("net/nokyan/Resources") {
        "system-monitor"
    } else {
        "misc"
    }
}

/// Generate split GNOME config files from dconf dump.
/// Returns {filename: content}.
pub fn generate_split(dconf_input: &str) -> BTreeMap<String, String> {
    let sections = parse_dconf_dump(dconf_input);
    let mut buckets: BTreeMap<&str, Vec<&DconfSection>> = BTreeMap::new();

    for section in &sections {
        let stem = categorize(&section.path);
        buckets.entry(stem).or_default().push(section);
    }

    let header = "# Generated by i-nix desktop generate gnome\n\
                  # Do not hand-edit; re-run `i-nix desktop generate gnome` instead.\n";

    let mut files: BTreeMap<String, String> = BTreeMap::new();

    for (stem, secs) in &buckets {
        let mut lines: Vec<String> = vec![
            header.to_string(),
            "{ lib, ... }:".to_string(),
            "".to_string(),
            "{".to_string(),
            "  dconf.settings = {".to_string(),
        ];

        for section in secs.iter() {
            lines.push(format!(r#"    "{}" = {{"#, section.path));
            for (key, val) in &section.settings {
                lines.push(format!("      {} = {};", key, val));
            }
            lines.push("    };".to_string());
            lines.push("".to_string());
        }

        lines.push("  };".to_string());
        lines.push("}".to_string());
        lines.push("".to_string());

        files.insert(format!("{}.nix", stem), lines.join("\n"));
    }

    // Generate default.nix
    let mut default_lines: Vec<String> = vec![
        header.to_string(),
        "{ ... }:".to_string(),
        "".to_string(),
        "{".to_string(),
        "  imports = [".to_string(),
    ];
    for stem in buckets.keys() {
        default_lines.push(format!("    ./{stem}.nix"));
    }
    default_lines.push("  ];".to_string());
    default_lines.push("}".to_string());
    default_lines.push("".to_string());

    files.insert("default.nix".to_string(), default_lines.join("\n"));

    files
}

/// Generate GNOME config from live dconf or provided text.
pub async fn generate_from_dconf(config_dir: &Path, user: &str) -> Result<()> {
    let desktop_dir = config_dir.join(format!("flake/users/{}/desktop", user));

    // Try to read live dconf dump
    let dconf_text = tokio::process::Command::new("dconf")
        .args(["dump", "/"])
        .output()
        .await?;

    let input = String::from_utf8_lossy(&dconf_text.stdout);

    if input.trim().is_empty() {
        anyhow::bail!(
            "dconf dump returned empty. Are you running inside a GNOME session?"
        );
    }

    let files = generate_split(&input);

    // Create gnome-settings directory
    let settings_dir = desktop_dir.join("gnome-settings");
    tokio::fs::create_dir_all(&settings_dir).await?;

    for (filename, content) in &files {
        let path = settings_dir.join(filename);
        tokio::fs::write(&path, content).await?;
    }

    // Update desktop/default.nix to import gnome-settings
    let default_nix = desktop_dir.join("default.nix");
    let content = format!(
        r#"# Desktop environment configuration for: {user}
{{ config, pkgs, lib, ... }}:

{{
  imports = [ ./gnome-settings ];
}}
"#,
        user = user
    );
    tokio::fs::write(&default_nix, content).await?;

    println!();
    println!("✓ Generated GNOME settings in {} files:", files.len() - 1);
    for filename in files.keys() {
        if filename != "default.nix" {
            println!("  • {}", filename);
        }
    }
    println!();
    println!("  Directory: {}", settings_dir.display());
    println!("  Run `i-nix apply` to activate.");
    println!();

    Ok(())
}
