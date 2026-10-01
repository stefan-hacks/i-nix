use anyhow::{Context, Result};
use colored::Colorize;
use regex::Regex;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::engine::NixEngine;
use crate::style;

/// Import an existing flake/flake-parts/home-manager/disko/etc. repository
/// into an i-nix optimal configuration structure.
#[allow(unused)]
pub async fn run(
    source_path: &str,
    output_dir: &str,
    verbose: bool,
    dry_run: bool,
    no_fmt: bool,
) -> Result<()> {
    let src = Path::new(source_path);
    if !src.exists() {
        anyhow::bail!("Source path does not exist: {}", source_path);
    }

    // ── Banner ──
    style::banner("import", "migrate existing Nix configuration");

    // ── 1. Discovery ──
    let discovery = discover(src, verbose).await?;

    style::section("", "Discovery Results");
    for fw in &discovery.frameworks {
        style::item_label("discovered", fw);
    }

    if let Some(ref flake) = discovery.flake_path {
        style::kv("flake.nix", &flake.display().to_string());
    }
    style::kv("NixOS hosts", &discovery.nixos_hosts.len().to_string());
    style::kv("Home configs", &discovery.home_configs.len().to_string());
    style::kv("Additional .nix files", &discovery.nix_files.len().to_string());

    if dry_run {
        style::warning("Dry run — would generate i-nix structure (no changes made).");
        return Ok(());
    }

    // ── 2. Determine target info ──
    let hostname = guess_hostname(&discovery).unwrap_or_else(crate::config::hostname);
    let username =
        guess_username(&discovery).unwrap_or_else(|| "stefan-hacks".to_string());

    // ── 3. Generate i-nix structure ──
    let out = Path::new(output_dir);
    let engine = NixEngine::new(out, &hostname, &username);

    style::step_simple("Generating i-nix structure...");

    // Base init (creates dirs + standard boilerplate)
    engine.init("system", discovery.has_home_manager)?;

    // Overwrite flake.nix with merged inputs from discovered repo
    let merged_flake = build_merged_flake(&discovery, &hostname, &username);
    fs::write(out.join("flake.nix"), merged_flake)
        .with_context(|| format!("writing {}", out.join("flake.nix").display()))?;

    // Overwrite hosts/default.nix with discovered hosts
    let hosts_nix = build_hosts_nix(&discovery, &username);
    fs::write(out.join("hosts/default.nix"), hosts_nix)
        .with_context(|| format!("writing hosts/default.nix"))?;

    // System configs per discovered host
    for (host_name, host_info) in &discovery.nixos_hosts {
        let sys_dir = out.join(format!("systems/{}", host_name));
        fs::create_dir_all(&sys_dir)?;

        let sys_default = build_system_nix(host_name, &username, host_info, &discovery);
        fs::write(sys_dir.join("default.nix"), sys_default)?;

        // Try to copy hardware-configuration.nix if found
        if let Some(hw_src) = find_hardware_config(src, host_name) {
            let hw_content = fs::read_to_string(&hw_src)
                .with_context(|| format!("reading hardware config: {}", hw_src.display()))?;
            fs::write(sys_dir.join("hardware.nix"), hw_content)?;
        }

        // Network config
        let net_nix = build_network_nix(host_name, host_info);
        fs::write(sys_dir.join("network.nix"), net_nix)?;
    }

    // User configs per discovered home config
    for (user_name, home_info) in &discovery.home_configs {
        let user_dir = out.join(format!("users/{}", user_name));

        // Ensure all subdirs exist (user may differ from default)
        fs::create_dir_all(&user_dir)?;
        fs::create_dir_all(user_dir.join("packages"))?;
        fs::create_dir_all(user_dir.join("programs"))?;
        fs::create_dir_all(user_dir.join("services"))?;
        fs::create_dir_all(user_dir.join("desktop"))?;
        fs::create_dir_all(user_dir.join("shells"))?;
        fs::create_dir_all(user_dir.join("secrets"))?;

        let user_default = build_user_default_nix(user_name, home_info, &discovery);
        fs::write(user_dir.join("default.nix"), user_default)?;

        // Packages
        let packages_nix = build_packages_nix(&home_info.packages);
        fs::write(user_dir.join("packages/default.nix"), packages_nix)?;

        // Programs
        let programs_nix = build_programs_nix(&home_info.programs);
        fs::write(user_dir.join("programs/default.nix"), programs_nix)?;

        // Services
        let services_nix = build_services_nix(&home_info.services);
        fs::write(user_dir.join("services/default.nix"), services_nix)?;

        // Desktop
        fs::write(user_dir.join("desktop/default.nix"), build_desktop_default_nix(&home_info.desktop))?;

        // Shells
        fs::write(
            user_dir.join("shells/default.nix"),
            build_shells_default_nix(&home_info.shell),
        )?;

        // Secrets
        fs::write(user_dir.join("secrets/default.nix"), build_secrets_default_nix())?;
    }

    // Profiles based on discovered frameworks
    write_profile(out, "core.nix", build_core_profile())?;
    write_profile(out, "desktop.nix", build_desktop_profile(&discovery))?;
    write_profile(out, "development.nix", build_dev_profile(&discovery))?;
    write_profile(out, "gaming.nix", build_gaming_profile(&discovery))?;
    write_profile(out, "server.nix", build_server_profile(&discovery))?;

    // Modules
    write_module(out, "system/default.nix", build_system_module(&discovery))?;
    write_module(out, "home/default.nix", build_home_module(&discovery))?;

    // Overlays / pkgs / lib
    write_overlays(out, &discovery)?;
    write_pkgs(out, &discovery)?;
    write_lib(out, &discovery)?;

    // Disko config if discovered
    if discovery.has_disko {
        let disko_dir = out.join("disko");
        fs::create_dir_all(&disko_dir)?;
        if let Some(ref disko_src) = discovery.disko_source {
            let disko_content = fs::read_to_string(disko_src)
                .with_context(|| format!("reading disko config: {}", disko_src.display()))?;
            fs::write(disko_dir.join("default.nix"), disko_content)?;
        } else {
            fs::write(
                disko_dir.join("default.nix"),
                build_disko_placeholder(),
            )?;
        }
    }

    // .gitignore
    fs::write(
        out.join(".gitignore"),
        "result\nresult-*\n*.qcow2\n*.vmdk\n",
    )?;

    // ── 4. Format with nix fmt ──
    if !no_fmt {
        style::step_simple("Running nix fmt...");
        match std::process::Command::new("nix")
            .args([
                "fmt",
                "--extra-experimental-features",
                "nix-command",
                "--extra-experimental-features",
                "flakes",
            ])
            .current_dir(out)
            .output()
        {
            Ok(output) if output.status.success() => {
                style::success("Formatted with nix fmt");
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                style::warning(format!("nix fmt: {}", stderr.trim()).as_str());
            }
            Err(e) => {
                style::warning(format!("Could not run nix fmt: {}", e).as_str());
            }
        }
    }

    // ── 5. State file ──
    let state = crate::config::INixState {
        version: env!("CARGO_PKG_VERSION").to_string(),
        hostname: hostname.clone(),
        last_applied_generation: None,
        last_applied_time: None,
        mode: if discovery.nixos_hosts.is_empty() {
            crate::config::INixMode::UserOnly
        } else {
            crate::config::INixMode::System
        },
        flake_path: out.to_path_buf(),
        has_home_manager: discovery.has_home_manager,
    };
    state.save(out)?;

    println!();
    println!("{}", "✓ Import complete".green().bold());
    println!();
    println!("  Output directory: {}", output_dir.dimmed());
    println!("  Hostname: {}", hostname.dimmed());
    println!("  Username: {}", username.dimmed());
    println!();
    println!("  {}", "Next steps:".bold());
    println!("    cd {} && git init", output_dir.dimmed());
    println!(
        "    i-nix apply              {}",
        "# Apply the imported configuration".dimmed()
    );
    println!();

    Ok(())
}

// ═════════════════════════════════════════════════════════════════════════════
// Discovery
// ═════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Default)]
struct Discovery {
    flake_path: Option<PathBuf>,
    frameworks: Vec<String>,
    nixos_hosts: BTreeMap<String, NixosHostInfo>,
    home_configs: BTreeMap<String, HomeConfigInfo>,
    nix_files: Vec<PathBuf>,
    inputs: BTreeMap<String, InputSpec>,
    has_home_manager: bool,
    has_disko: bool,
    has_sops: bool,
    has_agenix: bool,
    has_impermanence: bool,
    has_nix_darwin: bool,
    has_deploy_rs: bool,
    has_nur: bool,
    disko_source: Option<PathBuf>,
}

#[allow(dead_code)]
#[derive(Debug, Default)]
struct NixosHostInfo {
    system: String,
    modules: Vec<String>,
    system_packages: Vec<String>,
    services: Vec<String>,
    programs: Vec<String>,
    users: Vec<String>,
    imports: Vec<String>,
    extra_config: String,
}

#[allow(dead_code)]
#[derive(Debug, Default)]
struct HomeConfigInfo {
    packages: Vec<String>,
    programs: Vec<String>,
    services: Vec<String>,
    desktop: Option<String>, // "gnome", "kde", "hyprland", "sway"
    shell: Option<String>,     // "bash", "zsh", "fish"
    imports: Vec<String>,
    extra_config: String,
}

#[derive(Debug, Clone)]
struct InputSpec {
    url: String,
    follows: Option<String>,
}

async fn discover(src: &Path, verbose: bool) -> Result<Discovery> {
    let mut d = Discovery::default();

    // Find flake.nix
    let flake_candidate = src.join("flake.nix");
    if flake_candidate.exists() {
        d.flake_path = Some(flake_candidate);
    } else {
        // Search one level deep
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let maybe = path.join("flake.nix");
                if maybe.exists() {
                    d.flake_path = Some(maybe);
                    break;
                }
            }
        }
    }

    if d.flake_path.is_none() {
        anyhow::bail!(
            "No flake.nix found in {}. This doesn't look like a Nix flake repository.",
            src.display()
        );
    }

    let flake_content = fs::read_to_string(d.flake_path.as_ref().unwrap())?;

    // Parse inputs
    d.inputs = parse_inputs(&flake_content);
    if verbose {
        eprintln!("  → discovered {} flake inputs", d.inputs.len());
    }

    // Detect frameworks from inputs
    for (name, _spec) in &d.inputs {
        let lower = name.to_lowercase();
        match lower.as_str() {
            "home-manager" => {
                d.has_home_manager = true;
                d.frameworks.push("home-manager".to_string());
            }
            "disko" => {
                d.has_disko = true;
                d.frameworks.push("disko".to_string());
            }
            "sops-nix" | "sops" => {
                d.has_sops = true;
                d.frameworks.push("sops-nix".to_string());
            }
            "agenix" => {
                d.has_agenix = true;
                d.frameworks.push("agenix".to_string());
            }
            "impermanence" => {
                d.has_impermanence = true;
                d.frameworks.push("impermanence".to_string());
            }
            "nix-darwin" | "darwin" => {
                d.has_nix_darwin = true;
                d.frameworks.push("nix-darwin".to_string());
            }
            "deploy-rs" | "deploy" => {
                d.has_deploy_rs = true;
                d.frameworks.push("deploy-rs".to_string());
            }
            "nur" => {
                d.has_nur = true;
                d.frameworks.push("NUR".to_string());
            }
            "flake-parts" | "flakeparts" => {
                d.frameworks.push("flake-parts".to_string());
            }
            _ => {}
        }
    }

    // Detect from flake content (not just inputs)
    if flake_content.contains("flake-parts.lib.mkFlake")
        || flake_content.contains("flake-parts.lib.mkFlake")
    {
        if !d.frameworks.contains(&"flake-parts".to_string()) {
            d.frameworks.push("flake-parts".to_string());
        }
    }
    if flake_content.contains("homeConfigurations")
        || flake_content.contains("home-manager.nixosModules")
    {
        if !d.has_home_manager {
            d.has_home_manager = true;
            d.frameworks.push("home-manager".to_string());
        }
    }

    // Detect NixOS hosts
    d.nixos_hosts = discover_nixos_hosts(src, &flake_content, verbose).await?;

    // Detect home configurations
    d.home_configs = discover_home_configs(src, &flake_content, verbose).await?;

    // Collect all .nix files
    for entry in WalkDir::new(src).max_depth(5) {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map(|e| e == "nix").unwrap_or(false) {
            // Exclude result links, nix store paths
            let s = path.to_string_lossy();
            if !s.contains("/nix/store/") && !s.starts_with("result") {
                d.nix_files.push(path.to_path_buf());
            }
        }
    }

    // Try to find disko config source
    if d.has_disko {
        d.disko_source = find_disko_config(src);
    }

    Ok(d)
}

fn parse_inputs(content: &str) -> BTreeMap<String, InputSpec> {
    let mut inputs = BTreeMap::new();

    // Regex to match input declarations:
    // name.url = "...";
    // name = { url = "..."; inputs.foo.follows = "bar"; };
    let url_re = Regex::new(r#"([\w-]+)\.url\s*=\s*"([^"]+)";"#).unwrap();
    for cap in url_re.captures_iter(content) {
        let name = cap[1].to_string();
        let url = cap[2].to_string();
        inputs.insert(
            name.clone(),
            InputSpec {
                url,
                follows: None,
            },
        );
    }

    // Also detect inline { url = "..."; } blocks
    let block_re = Regex::new(r#"([\w-]+)\s*=\s*\{\s*url\s*=\s*"([^"]+)""#).unwrap();
    for cap in block_re.captures_iter(content) {
        let name = cap[1].to_string();
        let url = cap[2].to_string();
        if !inputs.contains_key(&name) {
            inputs.insert(
                name,
                InputSpec {
                    url,
                    follows: None,
                },
            );
        }
    }

    // Detect follows
    let follows_re = Regex::new(r#"inputs\.(\w+)\.follows\s*=\s*"([^"]+)""#).unwrap();
    for cap in follows_re.captures_iter(content) {
        let name = cap[1].to_string();
        let follows = cap[2].to_string();
        if let Some(spec) = inputs.get_mut(&name) {
            spec.follows = Some(follows);
        }
    }

    inputs
}

async fn discover_nixos_hosts(
    src: &Path,
    flake_content: &str,
    verbose: bool,
) -> Result<BTreeMap<String, NixosHostInfo>> {
    let mut hosts = BTreeMap::new();

    // Strategy 1: find nixosConfigurations block in flake.nix
    let nixos_re = Regex::new(
        r#"nixosConfigurations\s*=\s*\{([^}]*(?:\{[^}]*\}[^}]*)*)\};"#,
    )
    .unwrap();
    if let Some(cap) = nixos_re.captures(flake_content) {
        let block = &cap[1];
        // Extract host names: hostName = ...;
        let host_re = Regex::new(r#"(\w+)\s*=\s*(?:inputs\.)?nixpkgs\.lib\.nixosSystem"#)
            .unwrap();
        for hcap in host_re.captures_iter(block) {
            let name = hcap[1].to_string();
            if verbose {
                eprintln!("  → found NixOS host: {}", name);
            }
            hosts.entry(name).or_insert_with(NixosHostInfo::default);
        }
    }

    // Strategy 2: look in hosts/ directory for .nix files
    let hosts_dir = src.join("hosts");
    if hosts_dir.exists() {
        for entry in fs::read_dir(&hosts_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "nix").unwrap_or(false) {
                let name = path.file_stem().unwrap().to_string_lossy().to_string();
                if name == "default" || name == "flake" {
                    continue;
                }
                let info = hosts.entry(name).or_insert_with(NixosHostInfo::default);
                let content = fs::read_to_string(&path).unwrap_or_default();
                parse_nixos_host_content(&content, info);
            }
        }
    }

    // Strategy 3: look in systems/ or machines/
    for dir_name in &["systems", "machines", "nodes"] {
        let dir = src.join(dir_name);
        if dir.exists() {
            for entry in fs::read_dir(&dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().unwrap().to_string_lossy().to_string();
                    let info = hosts.entry(name).or_insert_with(NixosHostInfo::default);

                    // Read default.nix or configuration.nix
                    for fname in &["default.nix", "configuration.nix"] {
                        let f = path.join(fname);
                        if f.exists() {
                            let content = fs::read_to_string(&f).unwrap_or_default();
                            parse_nixos_host_content(&content, info);
                            break;
                        }
                    }
                }
            }
        }
    }

    // If no hosts found at all, create a default one
    if hosts.is_empty() {
        let hostname = crate::config::hostname();
        let mut info = NixosHostInfo::default();
        info.system = "x86_64-linux".to_string();
        hosts.insert(hostname, info);
    }

    Ok(hosts)
}

fn parse_nixos_host_content(content: &str, info: &mut NixosHostInfo) {
    // Extract system architecture
    let sys_re = Regex::new(r#"system\s*=\s*"([^"]+)""#).unwrap();
    if let Some(cap) = sys_re.captures(content) {
        info.system = cap[1].to_string();
    }

    // Extract system packages from environment.systemPackages
    info.system_packages =
        crate::nixast::extract_packages(content, "environment.systemPackages");

    // Extract imports
    let imports_re = Regex::new(r#"imports\s*=\s*\[([^\]]+)\]"#).unwrap();
    if let Some(cap) = imports_re.captures(content) {
        let block = &cap[1];
        for line in block.lines() {
            let trimmed = line.trim().trim_end_matches(';');
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                info.imports.push(trimmed.to_string());
            }
        }
    }

    // Extract enabled services
    let svc_re = Regex::new(r#"services\.(\w+)\.enable\s*=\s*true"#).unwrap();
    for cap in svc_re.captures_iter(content) {
        info.services.push(cap[1].to_string());
    }

    // Extract enabled programs
    let prg_re = Regex::new(r#"programs\.(\w+)\.enable\s*=\s*true"#).unwrap();
    for cap in prg_re.captures_iter(content) {
        info.programs.push(cap[1].to_string());
    }

    // Extract users
    let user_re = Regex::new(r#"users\.users\.(\w+)\s*=\s*\{"#).unwrap();
    for cap in user_re.captures_iter(content) {
        info.users.push(cap[1].to_string());
    }
}

async fn discover_home_configs(
    src: &Path,
    flake_content: &str,
    verbose: bool,
) -> Result<BTreeMap<String, HomeConfigInfo>> {
    let mut configs: BTreeMap<String, HomeConfigInfo> = BTreeMap::new();

    // Strategy 1: homeConfigurations in flake.nix
    let home_re = Regex::new(
        r#"homeConfigurations\s*=\s*\{([^}]*(?:\{[^}]*\}[^}]*)*)\};"#,
    )
    .unwrap();
    if let Some(cap) = home_re.captures(flake_content) {
        let block = &cap[1];
        let user_re = Regex::new(r#"(\w+)\s*=\s*(?:inputs\.)?home-manager\.lib\.homeManagerConfiguration"#)
            .unwrap();
        for hcap in user_re.captures_iter(block) {
            let name = hcap[1].to_string();
            if verbose {
                eprintln!("  → found home config: {}", name);
            }
            configs.entry(name).or_insert_with(HomeConfigInfo::default);
        }
    }

    // Strategy 2: look in home/ or users/ for home.nix / default.nix
    for dir_name in &["home", "users", "homes"] {
        let dir = src.join(dir_name);
        if dir.exists() {
            for entry in WalkDir::new(&dir).max_depth(3) {
                let entry = entry?;
                let path = entry.path();
                if path.file_name().map(|f| f == "home.nix").unwrap_or(false)
                    || path.file_name().map(|f| f == "default.nix").unwrap_or(false)
                {
                    let parent = path.parent().unwrap();
                    let user_name = parent.file_name().unwrap().to_string_lossy().to_string();
                    if user_name == "users" || user_name == "home" || user_name == "homes" {
                        continue;
                    }
                    let info = configs
                        .entry(user_name.clone())
                        .or_insert_with(HomeConfigInfo::default);
                    let content = fs::read_to_string(&path).unwrap_or_default();
                    parse_home_config_content(&content, info);
                }
            }
        }
    }

    Ok(configs)
}

fn parse_home_config_content(content: &str, info: &mut HomeConfigInfo) {
    // Extract packages
    info.packages = crate::nixast::extract_packages(content, "home.packages");

    // Extract programs
    let prg_re = Regex::new(r#"programs\.(\w+)\.enable\s*=\s*true"#).unwrap();
    for cap in prg_re.captures_iter(content) {
        info.programs.push(cap[1].to_string());
    }

    // Extract services
    let svc_re = Regex::new(r#"services\.(\w+)\.enable\s*=\s*true"#).unwrap();
    for cap in svc_re.captures_iter(content) {
        info.services.push(cap[1].to_string());
    }

    // Detect desktop environment
    let desktop_re = Regex::new(
        r#"(wayland\.windowManager\.(hyprland|sway)\.enable|services\.xserver\.desktopManager\.(gnome|kde|plasma)\.enable|programs\.hyprland\.enable)"#,
    )
    .unwrap();
    for cap in desktop_re.captures_iter(content) {
        let de = cap.get(2).or_else(|| cap.get(3)).map(|m| m.as_str());
        if let Some(de) = de {
            info.desktop = Some(de.to_lowercase());
        }
    }

    // Detect shell
    for shell in &["zsh", "bash", "fish"] {
        if content.contains(&format!("programs.{}.enable", shell)) {
            info.shell = Some(shell.to_string());
        }
    }

    // Extract imports
    let imports_re = Regex::new(r#"imports\s*=\s*\[([^\]]+)\]"#).unwrap();
    if let Some(cap) = imports_re.captures(content) {
        let block = &cap[1];
        for line in block.lines() {
            let trimmed = line.trim().trim_end_matches(';');
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                info.imports.push(trimmed.to_string());
            }
        }
    }
}

fn find_hardware_config(src: &Path, _host_name: &str) -> Option<PathBuf> {
    let candidates = [
        src.join("hardware-configuration.nix"),
        src.join("hosts/hardware-configuration.nix"),
        src.join("systems/hardware-configuration.nix"),
    ];
    for c in &candidates {
        if c.exists() {
            return Some(c.clone());
        }
    }
    // Search recursively for hardware-configuration.nix
    for entry in WalkDir::new(src).max_depth(4) {
        let entry = entry.ok()?;
        if entry.file_name() == "hardware-configuration.nix" {
            return Some(entry.path().to_path_buf());
        }
    }
    None
}

fn find_disko_config(src: &Path) -> Option<PathBuf> {
    for entry in WalkDir::new(src).max_depth(4) {
        let entry = entry.ok()?;
        let name = entry.file_name().to_string_lossy();
        if name.contains("disko") && name.ends_with(".nix") {
            return Some(entry.path().to_path_buf());
        }
    }
    None
}

fn guess_hostname(d: &Discovery) -> Option<String> {
    d.nixos_hosts.keys().next().cloned()
}

fn guess_username(d: &Discovery) -> Option<String> {
    d.home_configs.keys().next().cloned()
}

// ═════════════════════════════════════════════════════════════════════════════
// Builders
// ═════════════════════════════════════════════════════════════════════════════

fn build_merged_flake(d: &Discovery, _hostname: &str, _username: &str) -> String {
    let mut inputs = String::new();
    for (name, _spec) in &d.inputs {
        let spec = &d.inputs[name];
        if name == "nixpkgs" {
            inputs.push_str(&format!(
                "    {}.url = \"{}\";\n",
                name, spec.url
            ));
        } else if let Some(ref follows) = spec.follows {
            inputs.push_str(&format!(
                "    {} = {{\n      url = \"{}\";\n      inputs.nixpkgs.follows = \"{}\";\n    }};\n",
                name, spec.url, follows
            ));
        } else {
            inputs.push_str(&format!(
                "    {} = {{\n      url = \"{}\";\n      inputs.nixpkgs.follows = \"nixpkgs\";\n    }};\n",
                name, spec.url
            ));
        }
    }

    // Ensure required inputs exist
    if !d.inputs.contains_key("nixpkgs") {
        inputs.push_str("    nixpkgs.url = \"github:NixOS/nixpkgs/nixos-unstable\";\n");
    }
    if d.has_home_manager && !d.inputs.contains_key("home-manager") {
        inputs.push_str("    home-manager = {\n      url = \"github:nix-community/home-manager\";\n      inputs.nixpkgs.follows = \"nixpkgs\";\n    };\n");
    }
    if d.has_disko && !d.inputs.contains_key("disko") {
        inputs.push_str("    disko = {\n      url = \"github:nix-community/disko\";\n      inputs.nixpkgs.follows = \"nixpkgs\";\n    };\n");
    }
    if !d.inputs.contains_key("i-nix") {
        inputs.push_str("    i-nix = {\n      url = \"github:stefan-hacks/i-nix\";\n      inputs.nixpkgs.follows = \"nixpkgs\";\n    };\n");
    }
    if d.frameworks.contains(&"flake-parts".to_string()) && !d.inputs.contains_key("flake-parts") {
        inputs.push_str("    flake-parts = {\n      url = \"github:hercules-ci/flake-parts\";\n      inputs.nixpkgs.follows = \"nixpkgs\";\n    };\n");
    }

    // Build perSystem packages/apps if flake-parts
    let flake_parts_body = if d.frameworks.contains(&"flake-parts".to_string()) {
        r#"flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" "aarch64-linux" ];

      flake = {
        nixosConfigurations = import ./hosts {
          inherit inputs self;
        };
      };

      perSystem = { config, pkgs, system, ... }: {
        packages = import ./pkgs { inherit pkgs; };
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [ nixpkgs-fmt nixd ];
        };
      };
    }"#
        .to_string()
    } else {
        r#"{
      systems = [ "x86_64-linux" "aarch64-linux" ];

      nixosConfigurations = import ./hosts {
        inherit inputs self;
      };

      packages = import ./pkgs { inherit pkgs; };
    }"#
        .to_string()
    };

    format!(
        r#"{{
  description = "Declarative NixOS configuration managed by i-nix (imported)";

  inputs = {{
{inputs}
  }};

  outputs = inputs @ {{ self, nixpkgs, flake-parts, ... }}:
    {flake_parts_body};
}}
"#
    )
}

fn build_hosts_nix(d: &Discovery, username: &str) -> String {
    let mut entries = String::new();
    for (host, info) in &d.nixos_hosts {
        let system = if info.system.is_empty() {
            "x86_64-linux"
        } else {
            &info.system
        };

        let mut extra_modules = String::new();
        if d.has_disko {
            extra_modules.push_str("      inputs.disko.nixosModules.disko\n");
        }
        if d.has_sops {
            extra_modules.push_str("      inputs.sops-nix.nixosModules.sops\n");
        }
        if d.has_impermanence {
            extra_modules.push_str("      inputs.impermanence.nixosModules.impermanence\n");
        }

        entries.push_str(&format!(
            r#"  {host} = inputs.nixpkgs.lib.nixosSystem {{
    system = "{system}";
    specialArgs = {{ inherit inputs self; }};
    modules = [
      ../systems/_common.nix
      ../systems/{host}
{extra_modules}      ({{ pkgs, ... }}: {{
        nixpkgs.overlays = [ (import ../overlays) ];
        environment.systemPackages = [ inputs.i-nix.packages.${{pkgs.system}}.default ];
      }})
      inputs.home-manager.nixosModules.home-manager
      {{
        home-manager.useGlobalPkgs = true;
        home-manager.useUserPackages = true;
        home-manager.users.{username} = import ../users/{username};
      }}
    ];
  }};
"#,
        host = host,
            system = system,
            extra_modules = extra_modules,
            username = username,
        ));
    }

    format!(
        r#"# i-nix hosts registry (imported)
# Each attribute defines one physical machine.

{{ inputs, self, ... }}: {{
{}}}}};
"#,
        entries
    )
}

fn build_system_nix(
    host_name: &str,
    username: &str,
    info: &NixosHostInfo,
    d: &Discovery,
) -> String {
    let pkgs: Vec<String> = info
        .system_packages
        .iter()
        .map(|p| format!("    {}", p))
        .collect();
    let pkgs_block = if pkgs.is_empty() {
        "    # (no system packages discovered)".to_string()
    } else {
        pkgs.join("\n")
    };

    let services: Vec<String> = info
        .services
        .iter()
        .map(|s| format!("  services.{}.enable = true;", s))
        .collect();
    let services_block = if services.is_empty() {
        "  # (no services discovered)".to_string()
    } else {
        services.join("\n")
    };

    let programs: Vec<String> = info
        .programs
        .iter()
        .map(|p| format!("  programs.{}.enable = true;", p))
        .collect();
    let programs_block = if programs.is_empty() {
        "  # (no programs discovered)".to_string()
    } else {
        programs.join("\n")
    };

    let extra_imports: Vec<String> = info
        .imports
        .iter()
        .filter(|i| !i.contains("hardware-configuration") && !i.contains("home-manager"))
        .map(|i| format!("    {}", i))
        .collect();

    let imports_block = if extra_imports.is_empty() {
        "    ./hardware.nix\n    ./network.nix".to_string()
    } else {
        format!(
            "    ./hardware.nix\n    ./network.nix\n{}",
            extra_imports.join("\n")
        )
    };

    let mut extra = String::new();
    if d.has_disko {
        extra.push_str("  # Disko disk layout — configure in ../disko/\n");
        extra.push_str("  # imports = [ ../disko/default.nix ];\n\n");
    }
    if d.has_sops {
        extra.push_str("  # SOPS secrets — configure in users/<name>/secrets/\n");
    }
    if d.has_impermanence {
        extra.push_str("  # Impermanence — configure persistence paths\n");
    }

    format!(
        r#"# i-nix system configuration for: {host}
# Imported from existing repository.

{{ config, pkgs, lib, inputs, ... }}:

{{
  imports = [
{imports}
  ];

  # ── User account ──
  users.users.{user} = {{
    isNormalUser = true;
    group = "{user}";
    extraGroups = [ "wheel" "networkmanager" ];
  }};
  users.groups.{user} = {{ }};

  # ── System packages ──
  environment.systemPackages = with pkgs; [
{pkgs_block}
  ];

  # ── Services ──
{services_block}

  # ── Programs ──
{programs_block}

{extra}
  # ── Profiles ──
  # imports = [ ../profiles/desktop.nix ];
}}
"#,
        host = host_name,
        user = username,
        imports = imports_block,
        pkgs_block = pkgs_block,
        services_block = services_block,
        programs_block = programs_block,
        extra = extra,
    )
}

fn build_network_nix(host_name: &str, info: &NixosHostInfo) -> String {
    format!(
        r#"# Network configuration for: {host}

{{ config, pkgs, lib, ... }}:

{{
  networking.hostName = "{host}";
}}
"#,
        host = host_name,
    )
}

fn build_user_default_nix(
    user_name: &str,
    home_info: &HomeConfigInfo,
    d: &Discovery,
) -> String {
    let mut imports = vec![
        "./packages".to_string(),
        "./programs".to_string(),
        "./services".to_string(),
        "./desktop".to_string(),
        "./shells".to_string(),
        "./secrets".to_string(),
    ];

    // Add extra imports from discovered config
    for imp in &home_info.imports {
        if !imp.contains("home-manager") && !imp.contains("nixpkgs") {
            imports.push(imp.clone());
        }
    }

    let imports_block = imports
        .iter()
        .map(|i| format!("    {}", i))
        .collect::<Vec<_>>()
        .join("\n");

    let mut extra = String::new();
    if d.has_sops || d.has_agenix {
        extra.push_str("\n  # Secrets managed via sops-nix or agenix — see ./secrets/\n");
    }

    format!(
        r#"# i-nix home configuration for: {user}
# Imported from existing repository.

{{ config, pkgs, lib, inputs, ... }}:

{{
  home.username = "{user}";
  home.stateVersion = "24.11";

  home.packages = [ inputs.i-nix.packages.${{pkgs.system}}.default ];

  imports = [
{imports}
  ];{extra}
}}
"#,
        user = user_name,
        imports = imports_block,
        extra = extra,
    )
}

fn build_packages_nix(packages: &[String]) -> String {
    let pkgs: Vec<String> = packages.iter().map(|p| format!("    {}", p)).collect();
    let pkgs_block = if pkgs.is_empty() {
        "    # (no packages discovered)".to_string()
    } else {
        pkgs.join("\n")
    };

    format!(
        r#"# User packages for: (imported)

{{ config, pkgs, lib, ... }}:

{{
  home.packages = with pkgs; [
{pkgs}
  ];
}}
"#,
        pkgs = pkgs_block,
    )
}

fn build_programs_nix(programs: &[String]) -> String {
    let lines: Vec<String> = programs
        .iter()
        .map(|p| format!("  programs.{}.enable = true;", p))
        .collect();
    let block = if lines.is_empty() {
        "  # (no programs discovered)".to_string()
    } else {
        lines.join("\n")
    };

    format!(
        r#"# User programs configuration (imported)

{{ config, pkgs, lib, ... }}:

{{
{block}
}}
"#,
        block = block,
    )
}

fn build_services_nix(services: &[String]) -> String {
    let lines: Vec<String> = services
        .iter()
        .map(|s| format!("  services.{}.enable = true;", s))
        .collect();
    let block = if lines.is_empty() {
        "  # (no services discovered)".to_string()
    } else {
        lines.join("\n")
    };

    format!(
        r#"# User services configuration (imported)

{{ config, pkgs, lib, ... }}:

{{
{block}
}}
"#,
        block = block,
    )
}

fn build_desktop_default_nix(desktop: &Option<String>) -> String {
    let comment = match desktop.as_deref() {
        Some("gnome") => "# Detected GNOME — import ./gnome.nix if needed",
        Some("kde") | Some("plasma") => "# Detected KDE Plasma — import ./kde.nix if needed",
        Some("hyprland") => "# Detected Hyprland — import ./hyprland.nix if needed",
        Some("sway") => "# Detected Sway — import ./sway.nix if needed",
        _ => "# No desktop environment detected",
    };

    format!(
        r#"# Desktop environment configuration (imported)

{{ config, pkgs, lib, ... }}:

{{
  {comment}
}}
"#
    )
}

fn build_shells_default_nix(shell: &Option<String>) -> String {
    let shell_line = match shell.as_deref() {
        Some("zsh") => "  programs.zsh.enable = true;",
        Some("bash") => "  programs.bash.enable = true;",
        Some("fish") => "  programs.fish.enable = true;",
        _ => "  # (no shell detected)",
    };

    format!(
        r#"# Shell configuration (imported)

{{ config, pkgs, lib, ... }}:

{{
{shell_line}

  # Alias: i → i-nix
  # programs.zsh.shellAliases.i = "i-nix";
}}
"#,
        shell_line = shell_line,
    )
}

fn build_secrets_default_nix() -> String {
    r#"# Secrets configuration (imported)

{ config, pkgs, lib, ... }:

{
  # Configure sops-nix or agenix here
}
"#
    .to_string()
}

fn build_disko_placeholder() -> String {
    r#"# Disko disk layout (imported)
# See: https://github.com/nix-community/disko

{ config, pkgs, lib, ... }:

{
  # Replace with actual disko configuration
}
"#
    .to_string()
}

// ═════════════════════════════════════════════════════════════════════════════
// Profile builders
// ═════════════════════════════════════════════════════════════════════════════

fn build_core_profile() -> String {
    r#"# Core profile — minimal headless system

{ config, pkgs, lib, ... }:

{
  services.openssh.enable = true;
  services.openssh.settings.PermitRootLogin = "prohibit-password";
  networking.firewall.enable = true;
}
"#
    .to_string()
}

fn build_desktop_profile(d: &Discovery) -> String {
    let mut extra = String::new();
    if d.inputs.contains_key("hyprland") {
        extra.push_str("  # programs.hyprland.enable = true;\n");
    }

    format!(
        r#"# Desktop profile — graphical system (imported)

{{ config, pkgs, lib, ... }}:

{{
{extra}  # services.xserver.enable = true;
  # services.xserver.displayManager.gdm.enable = true;
  # services.xserver.desktopManager.gnome.enable = true;
}}
"#,
        extra = extra,
    )
}

fn build_dev_profile(_d: &Discovery) -> String {
    r#"# Development profile — dev tools (imported)

{ config, pkgs, lib, ... }:

{
  # programs.git.enable = true;
  # virtualisation.docker.enable = true;
}
"#
    .to_string()
}

fn build_gaming_profile(d: &Discovery) -> String {
    let mut extra = String::new();
    if d.inputs.contains_key("steam") || d.inputs.contains_key("nix-gaming") {
        extra.push_str("  # programs.steam.enable = true;\n");
    }

    format!(
        r#"# Gaming profile (imported)

{{ config, pkgs, lib, ... }}:

{{
{extra}  # hardware.graphics.enable = true;
}}
"#,
        extra = extra,
    )
}

fn build_server_profile(d: &Discovery) -> String {
    let mut extra = String::new();
    if d.has_deploy_rs {
        extra.push_str("  # deploy-rs configuration — see deploy.nix\n");
    }

    format!(
        r#"# Server profile — services and daemons (imported)

{{ config, pkgs, lib, ... }}:

{{
{extra}  # services.nginx.enable = true;
}}
"#,
        extra = extra,
    )
}

// ═════════════════════════════════════════════════════════════════════════════
// Module / overlay / pkgs / lib writers
// ═════════════════════════════════════════════════════════════════════════════

fn write_profile(out: &Path, name: &str, content: String) -> Result<()> {
    fs::write(out.join("profiles").join(name), content)?;
    Ok(())
}

fn write_module(out: &Path, name: &str, content: String) -> Result<()> {
    let parts: Vec<&str> = name.split('/').collect();
    let mut dir = out.join("modules");
    for part in &parts[..parts.len() - 1] {
        dir = dir.join(part);
    }
    fs::create_dir_all(&dir)?;
    fs::write(dir.join(parts.last().unwrap()), content)?;
    Ok(())
}

fn write_overlays(out: &Path, d: &Discovery) -> Result<()> {
    let mut content = String::from("final: prev: {\n");
    if d.has_nur {
        content.push_str("  # NUR overlay\n");
        content.push_str("  # nur = inputs.nur.legacyPackages.${prev.system};\n");
    }
    content.push_str("}\n");
    fs::write(out.join("overlays/default.nix"), content)?;
    Ok(())
}

fn write_pkgs(out: &Path, _d: &Discovery) -> Result<()> {
    fs::write(
        out.join("pkgs/default.nix"),
        "{ pkgs }: {\n  # Custom packages go here\n}\n",
    )?;
    Ok(())
}

fn write_lib(out: &Path, _d: &Discovery) -> Result<()> {
    fs::write(
        out.join("lib/helpers.nix"),
        "{ lib, ... }: {\n  # Helper functions go here\n}\n",
    )?;
    Ok(())
}

fn build_system_module(d: &Discovery) -> String {
    let mut extra = String::new();
    if d.has_disko {
        extra.push_str("  # disko module placeholder\n");
    }

    format!(
        r#"# Reusable NixOS system modules (imported)

{{ config, pkgs, lib, ... }}:

{{
{extra}}}
"#,
        extra = extra,
    )
}

fn build_home_module(d: &Discovery) -> String {
    let mut extra = String::new();
    if d.has_sops || d.has_agenix {
        extra.push_str("  # secrets module placeholder\n");
    }

    format!(
        r#"# Reusable Home Manager modules (imported)

{{ config, pkgs, lib, ... }}:

{{
{extra}}}
"#,
        extra = extra,
    )
}
