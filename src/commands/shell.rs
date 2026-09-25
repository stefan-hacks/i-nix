use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};
use tokio::process::Command;

/// Run a development shell (nix-shell or devShell) with the specified packages.
///
/// Examples:
///   i-nix shell rust                # Quick Rust dev env
///   i-nix shell nodejs typescript   # Node dev env
///   i-nix shell --flake             # Enter current project's devShell
///   i-nix shell --list              # List known dev templates
///
pub async fn run(
    packages: Vec<String>,
    flake: bool,
    list: bool,
    pure: bool,
) -> Result<()> {
    if list {
        print_shell_templates();
        return Ok(());
    }

    if flake {
        // Enter the current directory's devShell
        println!("{}", "Entering devShell for current flake...".cyan().bold());
        let mut cmd = Command::new("nix");
        cmd.args(["develop", "-c", "$SHELL"]);
        if pure {
            cmd.arg("--ignore-environment");
        }
        cmd.status().await?;
        return Ok(());
    }

    if packages.is_empty() {
        anyhow::bail!("No packages specified. Use `i-nix shell <packages...>` or `i-nix shell --flake`");
    }

    // Build a temporary nix-shell expression
    println!("{}", format!("Starting shell with: {}", packages.join(", ")).cyan().bold());
    println!();

    let shell_expr = build_shell_expr(&packages);
    let temp_file = tempfile::NamedTempFile::with_suffix(".nix")?;
    fs::write(temp_file.path(), shell_expr)?;

    let mut cmd = Command::new("nix-shell");
    cmd.arg(temp_file.path());
    if pure {
        cmd.arg("--pure");
    }
    cmd.arg("--run").arg(std::env::var("SHELL").unwrap_or_else(|_| "bash".to_string()));

    println!("  {}", "Exit the shell with Ctrl+D or `exit`".dimmed());
    println!();

    cmd.status().await?;
    Ok(())
}

fn build_shell_expr(packages: &[String]) -> String {
    let pkgs_list = packages
        .iter()
        .map(|p| format!("      pkgs.{}", p))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"{{ pkgs ? import <nixpkgs> {{}} }}:

pkgs.mkShell {{
  name = "i-nix-shell";
  buildInputs = with pkgs; [
{}
  ];

  shellHook = ''
    echo "i-nix dev shell ready. Packages loaded: {}"
  '';
}}
"#,
        pkgs_list,
        packages.join(", ")
    )
}

fn print_shell_templates() {
    println!();
    println!("{}", "Dev Shell Templates".bold());
    println!();
    println!("  {} {}", "i-nix shell".cyan().bold(), "rust cargo clippy rustfmt");
    println!("    → Rust development environment");
    println!();
    println!("  {} {}", "i-nix shell".cyan().bold(), "nodejs yarn typescript nodePackages.typescript-language-server");
    println!("    → Node.js/TypeScript development");
    println!();
    println!("  {} {}", "i-nix shell".cyan().bold(), "python3 python3Packages.pip python3Packages.virtualenv");
    println!("    → Python development");
    println!();
    println!("  {} {}", "i-nix shell".cyan().bold(), "go gopls delve");
    println!("    → Go development");
    println!();
    println!("  {} {}", "i-nix shell".cyan().bold(), "docker podman skopeo");
    println!("    → Container tooling");
    println!();
    println!("  {} {}", "i-nix shell".cyan().bold(), "postgresql redis sqlite");
    println!("    → Database development");
    println!();
    println!("  {} {}", "i-nix shell".cyan().bold(), "terraform awscli2 kubectl");
    println!("    → DevOps/Cloud tooling");
    println!();
    println!("  {} {}", "i-nix shell".cyan().bold(), "nixfmt alejandra deadnix statix");
    println!("    → Nix tooling");
    println!();
    println!("  {} {}", "i-nix shell".cyan().bold(), "--flake");
    println!("    → Enter current project's devShell from flake.nix");
    println!();
    println!("  {} {}", "i-nix shell".cyan().bold(), "--pure rust cargo");
    println!("    → Pure shell (isolated from host PATH)");
    println!();
}
