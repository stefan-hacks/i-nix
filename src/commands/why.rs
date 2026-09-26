use anyhow::Result;
use colored::Colorize;
use std::process::Command;

pub async fn run(package: &str, _verbose: bool) -> Result<()> {
    println!();
    println!("{}", format!("Why {}", package).bold());
    println!();

    // Check if nix why-depends is available
    match Command::new("nix")
        .args([
            "why-depends",
            "/run/current-system",
            &format!("nixpkgs#{}", package),
        ])
        .output()
    {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            println!("{}", stdout.trim());
        }
        _ => {
            println!("{}", format!("  Package: {}", package).dimmed());
            println!("  nix why-depends not available (not on NixOS or package not in closure).");
            println!();
            println!("  To see where a package is declared, check:");
            println!("    {} ~/.config/i-nix/flake/systems/default.nix", "→".cyan());
            println!("    {} ~/.config/i-nix/flake/home/default.nix", "→".cyan());
        }
    }

    println!();
    Ok(())
}
