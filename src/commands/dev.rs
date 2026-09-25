use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::path::Path;

/// Scaffold a new Nix development project (flake) for a given stack.
///
/// Examples:
///   i-nix dev init rust my-project
///   i-nix dev init node my-api
///   i-nix dev init python my-ml-app
///   i-nix dev init go my-cli
///   i-nix dev init haskell my-lib
///   i-nix dev list
///
pub async fn run(stack: &str, name: Option<String>, list: bool) -> Result<()> {
    if list || stack == "list" {
        print_dev_templates();
        return Ok(());
    }

    if stack.is_empty() {
        print_dev_templates();
        return Ok(());
    }

    let project_name = name.unwrap_or_else(|| "my-project".to_string());
    let project_dir = Path::new(&project_name);

    if project_dir.exists() {
        anyhow::bail!("Directory '{}' already exists.", project_name);
    }

    println!();
    println!("{}", format!("Scaffolding {} project: {}", stack, project_name).cyan().bold());
    println!();

    match stack {
        "rust" => scaffold_rust(&project_name)?,
        "node" | "nodejs" => scaffold_node(&project_name)?,
        "python" => scaffold_python(&project_name)?,
        "go" => scaffold_go(&project_name)?,
        "haskell" => scaffold_haskell(&project_name)?,
        "generic" | "flake" => scaffold_generic(&project_name)?,
        _ => {
            anyhow::bail!(
                "Unknown stack '{}'. Run `i-nix dev list` to see available templates.",
                stack
            );
        }
    }

    println!("  {} created {}", "✓".green(), project_name);
    println!();
    println!("  Next steps:");
    println!("    cd {}", project_name);
    println!("    nix develop          # Enter dev shell");
    println!("    nix build            # Build the project");
    println!("    nix run              # Run the project");
    println!();

    Ok(())
}

fn scaffold_rust(name: &str) -> Result<()> {
    fs::create_dir_all(name)?;
    fs::create_dir_all(format!("{}/src", name))?;

    // Cargo.toml
    fs::write(
        format!("{}/Cargo.toml", name),
        format!(
            r#"[package]
name = "{}"
version = "0.1.0"
edition = "2024"

[dependencies]
"#,
            name
        ),
    )?;

    // src/main.rs
    fs::write(
        format!("{}/src/main.rs", name),
        r#"fn main() {
    println!("Hello from i-nix + Rust!");
}
"#,
    )?;

    // flake.nix
    fs::write(
        format!("{}/flake.nix", name),
        r#"{
  description = "Rust project with i-nix";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs = inputs @ { self, nixpkgs, flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" "aarch64-linux" ];
      perSystem = { pkgs, ... }: {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustc cargo clippy rustfmt
          ];
        };
      };
    };
}
"#,
    )?;

    // .gitignore
    fs::write(format!("{}/.gitignore", name), "target/\n")?;

    Ok(())
}

fn scaffold_node(name: &str) -> Result<()> {
    fs::create_dir_all(name)?;

    fs::write(
        format!("{}/package.json", name),
        format!(
            r#"{{
  "name": "{}",
  "version": "0.1.0",
  "scripts": {{
    "dev": "node index.js",
    "build": "echo 'Build complete'"
  }}
}}"#,
            name
        ),
    )?;

    fs::write(
        format!("{}/index.js", name),
        r#"console.log('Hello from i-nix + Node.js!');
"#,
    )?;

    fs::write(
        format!("{}/flake.nix", name),
        r#"{
  description = "Node.js project with i-nix";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
    in {
      devShells.${system}.default = pkgs.mkShell {
        buildInputs = with pkgs; [ nodejs yarn ];
      };
    };
}
"#,
    )?;

    fs::write(format!("{}/.gitignore", name), "node_modules/\n")?;

    Ok(())
}

fn scaffold_python(name: &str) -> Result<()> {
    fs::create_dir_all(name)?;

    fs::write(format!("{}/main.py", name), "print('Hello from i-nix + Python!')\n")?;

    fs::write(
        format!("{}/flake.nix", name),
        r#"{
  description = "Python project with i-nix";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
    in {
      devShells.${system}.default = pkgs.mkShell {
        buildInputs = with pkgs; [
          python3 python3Packages.pip python3Packages.virtualenv
        ];
      };
    };
}
"#,
    )?;

    fs::write(format!("{}/.gitignore", name), "__pycache__/\nvenv/\n")?;

    Ok(())
}

fn scaffold_go(name: &str) -> Result<()> {
    fs::create_dir_all(name)?;
    fs::create_dir_all(format!("{}/cmd/", name))?;

    fs::write(
        format!("{}/go.mod", name),
        format!("module {}\n\ngo 1.23\n", name),
    )?;

    fs::write(
        format!("{}/cmd/main.go", name),
        r#"package main

import "fmt"

func main() {
    fmt.Println("Hello from i-nix + Go!")
}
"#,
    )?;

    fs::write(
        format!("{}/flake.nix", name),
        r#"{
  description = "Go project with i-nix";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
    in {
      devShells.${system}.default = pkgs.mkShell {
        buildInputs = with pkgs; [ go gopls delve ];
      };
    };
}
"#,
    )?;

    Ok(())
}

fn scaffold_haskell(name: &str) -> Result<()> {
    fs::create_dir_all(name)?;

    fs::write(
        format!("{}/Main.hs", name),
        r#"main = putStrLn "Hello from i-nix + Haskell!"
"#,
    )?;

    fs::write(
        format!("{}/flake.nix", name),
        r#"{
  description = "Haskell project with i-nix";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs = inputs @ { self, nixpkgs, flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" "aarch64-linux" ];
      perSystem = { pkgs, ... }: {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [ ghc cabal-install stack ];
        };
      };
    };
}
"#,
    )?;

    Ok(())
}

fn scaffold_generic(name: &str) -> Result<()> {
    fs::create_dir_all(name)?;

    fs::write(
        format!("{}/flake.nix", name),
        r#"{
  description = "Generic project with i-nix";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
    in {
      devShells.${system}.default = pkgs.mkShell {
        buildInputs = with pkgs; [
          # Add your dependencies here
        ];
      };
    };
}
"#,
    )?;

    fs::write(
        format!("{}/README.md", name),
        "# Project scaffolded by i-nix\n",
    )?;

    Ok(())
}

fn print_dev_templates() {
    println!();
    println!("{}", "Dev Templates".bold());
    println!();
    println!("  {} {}", "i-nix dev init rust".cyan().bold(), "my-app");
    println!("    → Rust with Cargo, clippy, rustfmt");
    println!();
    println!("  {} {}", "i-nix dev init node".cyan().bold(), "my-api");
    println!("    → Node.js with npm/yarn");
    println!();
    println!("  {} {}", "i-nix dev init python".cyan().bold(), "my-ml");
    println!("    → Python with pip, virtualenv");
    println!();
    println!("  {} {}", "i-nix dev init go".cyan().bold(), "my-cli");
    println!("    → Go with gopls, delve");
    println!();
    println!("  {} {}", "i-nix dev init haskell".cyan().bold(), "my-lib");
    println!("    → Haskell with GHC, cabal, stack");
    println!();
    println!("  {} {}", "i-nix dev init generic".cyan().bold(), "my-project");
    println!("    → Empty flake.nix template");
    println!();
}
