use anyhow::Result;
use colored::Colorize;

pub async fn run(config_dir: &str, package: &str) -> Result<()> {
    println!("{} Why is '{}' installed?", "i-nix why:".cyan().bold(), package.cyan());
    Ok(())
}
