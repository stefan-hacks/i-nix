use anyhow::Result;
use colored::Colorize;

pub async fn run(config_dir: &str, target: &str) -> Result<()> {
    println!("{} Opening {} in $EDITOR...", "i-nix edit:".cyan().bold(), target.cyan());
    Ok(())
}
