use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::path::Path;

/// Build OCI container images from a Nix expression or the current flake.
///
/// Wraps nix2container / dockerTools for declarative, reproducible container builds.
///
/// Examples:
///   i-nix container build .#my-image
///   i-nix container build --from-shell nodejs express
///   i-nix container run .#my-image
///   i-nix container push .#my-image docker://registry.example.com/my-image
///   i-nix container list
///
pub async fn run(
    action: &str,
    target: Option<String>,
    packages: Vec<String>,
    tag: Option<String>,
    registry: Option<String>,
) -> Result<()> {
    match action {
        "build" => build_container(target, packages, tag).await,
        "run" => run_container(target).await,
        "push" => push_container(target, registry).await,
        "list" | "ls" => list_images().await,
        "load" => load_container(target).await,
        _ => {
            anyhow::bail!(
                "Unknown action '{}'. Available: build, run, push, list, load",
                action
            )
        }
    }
}

async fn build_container(
    target: Option<String>,
    packages: Vec<String>,
    tag: Option<String>,
) -> Result<()> {
    if let Some(flake_ref) = target {
        // Build from flake reference
        println!();
        println!("{}", format!("Building container: {}", flake_ref).cyan().bold());
        println!();

        let mut cmd = tokio::process::Command::new("nix");
        cmd.args(["build", &flake_ref]);
        let status = cmd.status().await?;

        if !status.success() {
            anyhow::bail!("Container build failed");
        }

        println!("  {} container built successfully", "✓".green());
        println!();
        println!("  Load into Docker: {} {}", "i-nix container load".dimmed(), flake_ref.dimmed());
        println!("  Or run directly:   {} {}", "i-nix container run".dimmed(), flake_ref.dimmed());
        println!();
        return Ok(());
    }

    // Build from inline package list (nix2container / dockerTools)
    if packages.is_empty() {
        anyhow::bail!("Either specify a flake target (e.g. .#my-image) or provide packages to include.");
    }

    println!();
    println!("{}", format!("Building container with: {}", packages.join(", ")).cyan().bold());
    println!();

    let container_expr = build_container_expr(&packages, tag.as_deref());
    let temp_file = tempfile::NamedTempFile::with_suffix(".nix")?;
    fs::write(temp_file.path(), container_expr)?;

    println!("  Building...");
    let mut cmd = tokio::process::Command::new("nix-build");
    cmd.arg(temp_file.path());
    let status = cmd.status().await?;

    if !status.success() {
        anyhow::bail!("Container build failed");
    }

    println!("  {} container built", "✓".green());
    println!();
    println!("  Load: {}", "nix run nixpkgs#skopeo -- copy docker-archive:./result docker-daemon:my-image".dimmed());
    println!();

    Ok(())
}

async fn run_container(target: Option<String>) -> Result<()> {
    let flake_ref = target.ok_or_else(|| anyhow::anyhow!("Specify a container target, e.g. .#my-image"))?;
    println!("{}", format!("Running container: {}", flake_ref).cyan().bold());
    println!();
    println!("  {}", "Note: This loads the Nix-built image into Docker/Podman and runs it.".dimmed());
    println!();
    println!("  For now, run manually:");
    println!("    nix build {}  ", flake_ref);
    println!("    ./result | docker load");
    println!("    docker run -it my-image");
    println!();
    Ok(())
}

async fn push_container(target: Option<String>, registry: Option<String>) -> Result<()> {
    let flake_ref = target.ok_or_else(|| anyhow::anyhow!("Specify a container target"))?;
    let registry_url = registry.unwrap_or_else(|| "docker.io".to_string());

    println!();
    println!("{}", format!("Pushing {} to {}", flake_ref, registry_url).cyan().bold());
    println!();

    // Use skopeo for declarative push
    let mut cmd = tokio::process::Command::new("nix");
    cmd.args([
        "run", "nixpkgs#skopeo", "--",
        "copy",
        &format!("docker-archive:./result"),
        &format!("docker://{}/my-image:latest", registry_url),
    ]);
    let status = cmd.status().await?;

    if !status.success() {
        anyhow::bail!("Push failed");
    }

    println!("  {} pushed successfully", "✓".green());
    Ok(())
}

async fn list_images() -> Result<()> {
    println!();
    println!("{}", "Locally built Nix containers".bold());
    println!();

    // Try podman
    let podman = tokio::process::Command::new("podman")
        .args(["images", "--filter", "reference=*nix*"])
        .output()
        .await;
    if let Ok(o) = podman {
        if o.status.success() {
            let out = String::from_utf8_lossy(&o.stdout);
            if !out.trim().is_empty() {
                println!("{}", "  Podman images:".bold());
                println!("{}", out);
            }
        }
    }

    // Try docker
    let docker = tokio::process::Command::new("docker")
        .args(["images", "--filter", "reference=*nix*"])
        .output()
        .await;
    if let Ok(o) = docker {
        if o.status.success() {
            let out = String::from_utf8_lossy(&o.stdout);
            if !out.trim().is_empty() {
                println!("{}", "  Docker images:".bold());
                println!("{}", out);
            }
        }
    }

    println!("  {}", "Build containers with `i-nix container build .#my-image`".dimmed());
    println!();
    Ok(())
}

async fn load_container(target: Option<String>) -> Result<()> {
    println!();
    println!("{}", "Loading Nix container into Docker...".cyan().bold());
    println!();

    let mut cmd = tokio::process::Command::new("sh");
    cmd.arg("-c").arg("./result | docker load");
    let status = cmd.status().await?;

    if !status.success() {
        anyhow::bail!("Failed to load container");
    }

    println!("  {} container loaded into Docker", "✓".green());
    println!();
    Ok(())
}

fn build_container_expr(packages: &[String], tag: Option<&str>) -> String {
    let pkgs_list = packages
        .iter()
        .map(|p| format!("      pkgs.{}", p))
        .collect::<Vec<_>>()
        .join("\n");

    let image_tag = tag.unwrap_or("latest");

    format!(
        r#"{{ pkgs ? import <nixpkgs> {{}} }}:

let
  myEnv = pkgs.buildEnv {{
    name = "my-container-env";
    paths = with pkgs; [
{}
    ];
  }};
in
pkgs.dockerTools.buildImage {{
  name = "my-image";
  tag = "{}";
  copyToRoot = pkgs.buildEnv {{
    name = "image-root";
    paths = [ myEnv pkgs.bash pkgs.coreutils ];
    pathsToLink = [ "/bin" ];
  }};
  config = {{
    Cmd = [ "/bin/bash" ];
  }};
}}
"#,
        pkgs_list, image_tag
    )
}
