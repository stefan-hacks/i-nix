use anyhow::Result;
use colored::Colorize;
use std::path::Path;

pub async fn run(config_dir: &str) -> Result<()> {
    println!("{} Running system diagnostics...", "i-nix doctor:".cyan().bold());
    Ok(())
}
