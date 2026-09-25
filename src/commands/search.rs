use anyhow::Result;
use colored::Colorize;
use serde_json::Value;
use std::process::Stdio;
use tokio::process::Command;

pub async fn run(query: &str, limit: usize) -> Result<()> {
    println!();
    println!("{} {}", "Searching nixpkgs for:".bold(), query.cyan());
    println!();

    // Use nix search for accurate results
    let output = Command::new("nix")
        .args([
            "search",
            "nixpkgs",
            query,
            "--json",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;

    match output {
        Ok(result) if result.status.success() => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            let json: Value = serde_json::from_str(&stdout)
                .unwrap_or_else(|_| Value::Object(Default::default()));

            if let Some(obj) = json.as_object() {
                let mut count = 0;
                for (key, val) in obj.iter().take(limit) {
                    let pname = val.get("pname").and_then(|v| v.as_str()).unwrap_or("?");
                    let version = val.get("version").and_then(|v| v.as_str()).unwrap_or("?");
                    let description = val
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("No description");

                    // Extract short attribute name
                    let attr = key.split('.').last().unwrap_or(key);

                    println!(
                        "  {} {} {}",
                        "●".green(),
                        attr.bold(),
                        format!("({} {})", pname, version).dimmed()
                    );
                    println!("    {}", description);
                    println!();

                    count += 1;
                }

                let total = obj.len();
                if total > limit {
                    println!(
                        "  {} Showing {} of {} results. Use --limit to show more.",
                        "ℹ".blue(),
                        limit,
                        total
                    );
                }

                if count == 0 {
                    println!("  {} No results found", "∅".dimmed());
                }
            } else {
                println!("  {} Could not parse search results", "✗".red());
            }
        }
        Ok(result) => {
            let stderr = String::from_utf8_lossy(&result.stderr);
            if stderr.contains("not found") || stderr.contains("no results") {
                println!("  {} No packages found matching '{}'", "∅".dimmed(), query);
            } else {
                eprintln!("  {} nix search failed: {}", "✗".red(), stderr.trim());
            }
        }
        Err(e) => {
            eprintln!(
                "  {} Could not run nix search: {}",
                "✗".red(),
                e
            );
            eprintln!("  Make sure nix is installed and flakes are enabled.");
        }
    }

    Ok(())
}
