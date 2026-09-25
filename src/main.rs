use clap::{Parser, Subcommand};
use colored::Colorize;
use std::process;

mod commands;
mod config;
mod desktop;
mod engine;
mod flake;
mod home;
mod nixast;
mod nixos;

use commands::*;

#[derive(Parser)]
#[command(name = "i-nix")]
#[command(about = "Imperative UX for declarative Nix/NixOS systems")]
#[command(version = "0.1.0")]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Path to the i-nix configuration directory
    #[arg(short, long, global = true, default_value = "~/.config/i-nix")]
    config_dir: String,

    /// Dry run — show what would be done without doing it
    #[arg(short = 'n', long, global = true)]
    dry_run: bool,

    /// Be verbose about what is happening
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new i-nix configuration
    Init {
        /// Host name for this machine
        #[arg(short = 'H', long)]
        hostname: Option<String>,
        /// Initialize for user-level (Home Manager) only
        #[arg(long)]
        user: bool,
        /// Initialize for NixOS (system-level)
        #[arg(long)]
        system: bool,
    },

    /// Install a package declaratively
    Install {
        /// Package name(s) to install
        #[arg(required = true)]
        packages: Vec<String>,
        /// Install as user package (home-manager) instead of system
        #[arg(short, long)]
        user: bool,
        /// Install as a program module (e.g. programs.firefox.enable)
        #[arg(long)]
        program: bool,
        /// Nixpkgs attribute path if different from package name
        #[arg(short, long)]
        attribute: Option<String>,
    },

    /// Remove a package from declarative configuration
    Remove {
        /// Package name(s) to remove
        #[arg(required = true)]
        packages: Vec<String>,
        /// Remove from user packages instead of system
        #[arg(short, long)]
        user: bool,
    },

    /// List declaratively managed packages
    List {
        /// Show only user packages
        #[arg(short, long)]
        user: bool,
        /// Show only system packages
        #[arg(short, long)]
        system: bool,
    },

    /// Search nixpkgs for a package
    Search {
        /// Search query
        query: String,
        /// Limit results
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },

    /// Show pending changes (diff between current config and last applied)
    Diff,

    /// Apply changes — evaluate and rebuild
    Apply {
        /// Don't ask for confirmation
        #[arg(short, long)]
        yes: bool,
        /// Build but don't switch (test the build)
        #[arg(long)]
        build: bool,
    },

    /// Rollback to the previous generation
    Rollback,

    /// Discover current system state and generate declarative config (adopt)
    Adopt {
        /// Write the discovered configuration
        #[arg(long)]
        write: bool,
    },

    /// Diagnose the Nix/NixOS/i-nix installation
    Doctor,

    /// Show why a package is installed
    Why {
        /// Package name
        package: String,
    },

    /// Open generated Nix configuration in $EDITOR
    Edit {
        /// What to edit: packages, system, home, flake
        #[arg(default_value = "packages")]
        target: String,
    },

    /// Enable a service or program declaratively
    Enable {
        /// Service or program to enable (e.g. ssh, firefox, docker)
        #[arg(required_unless_present = "list")]
        name: Option<String>,
        /// Enable as a user program (programs.<name>.enable) instead of system service
        #[arg(short, long)]
        user: bool,
        /// Treat as a service (services.<name>.enable) — this is the default for system targets
        #[arg(long)]
        service: bool,
        /// Treat as a program (programs.<name>.enable) — default for --user
        #[arg(long)]
        program: bool,
        /// List known services and programs
        #[arg(long)]
        list: bool,
    },

    /// Disable a service or program declaratively
    Disable {
        /// Service or program to disable (e.g. ssh, firefox, docker)
        #[arg(required = true)]
        name: String,
        /// Disable as a user program instead of system service
        #[arg(short, long)]
        user: bool,
        /// Treat as a service (default for system targets)
        #[arg(long)]
        service: bool,
        /// Treat as a program
        #[arg(long)]
        program: bool,
    },

    /// Manage desktop environments (GNOME, KDE, Hyprland, Sway, etc.)
    Desktop {
        /// Subcommand: detect, generate, list, apply
        #[arg(default_value = "")]
        subcommand: String,
        /// Desktop environment name (for generate)
        #[arg(default_value = "")]
        name: String,
        /// Generate from live dconf/settings instead of template
        #[arg(short, long)]
        live: bool,
    },

    /// Update flake inputs and show available upgrades
    Update {
        /// Update all inputs without asking
        #[arg(short, long)]
        all: bool,
    },

    /// Start a dev shell with packages (nix-shell / devShell)
    Shell {
        /// Packages to include in the shell (not required with --list or --flake)
        #[arg(required_unless_present_any = ["list", "flake"])]
        packages: Vec<String>,
        /// Enter current flake's devShell instead
        #[arg(long)]
        flake: bool,
        /// List known dev templates
        #[arg(long)]
        list: bool,
        /// Pure shell (ignore host environment)
        #[arg(long)]
        pure: bool,
    },

    /// Scaffold a new Nix development project
    Dev {
        /// Stack/language: rust, node, python, go, haskell, generic
        #[arg(default_value = "")]
        stack: String,
        /// Project name
        name: Option<String>,
        /// List available templates
        #[arg(long)]
        list: bool,
    },

    /// Build and manage OCI container images
    Container {
        /// Action: build, run, push, list, load
        action: String,
        /// Flake target or image name
        target: Option<String>,
        /// Additional packages to include
        #[arg(last = true)]
        packages: Vec<String>,
        /// Image tag
        #[arg(short, long)]
        tag: Option<String>,
        /// Container registry
        #[arg(short, long)]
        registry: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let config_dir = shellexpand::tilde(&cli.config_dir).to_string();

    if cli.verbose {
        eprintln!("{} {}", "i-nix:".cyan().bold(), format!("config dir = {}", config_dir).dimmed());
    }

    let result = match cli.command {
        Commands::Init { hostname, user, system } => {
            init::run(&config_dir, hostname, user, system, cli.verbose, cli.dry_run).await
        }
        Commands::Install { packages, user, program, attribute } => {
            install::run(&config_dir, packages, user, program, attribute, cli.verbose, cli.dry_run).await
        }
        Commands::Remove { packages, user } => {
            remove::run(&config_dir, packages, user, cli.verbose, cli.dry_run).await
        }
        Commands::List { user, system } => {
            list::run(&config_dir, user, system).await
        }
        Commands::Search { query, limit } => {
            search::run(&query, limit).await
        }
        Commands::Diff => {
            diff::run(&config_dir).await
        }
        Commands::Apply { yes, build } => {
            apply::run(&config_dir, yes, build, cli.verbose, cli.dry_run).await
        }
        Commands::Rollback => {
            rollback::run(&config_dir).await
        }
        Commands::Adopt { write } => {
            adopt::run(&config_dir, write, cli.verbose, cli.dry_run).await
        }
        Commands::Doctor => {
            doctor::run(&config_dir).await
        }
        Commands::Why { package } => {
            why::run(&package, cli.verbose).await
        }
        Commands::Edit { target } => {
            edit::run(&target).await
        }
        Commands::Enable { name, user, service, program, list } => {
            enable::run(&config_dir, name, user, service, program, list, cli.verbose, cli.dry_run).await
        }
        Commands::Disable { name, user, service, program } => {
            enable::disable(&config_dir, &name, user, service, program, cli.verbose, cli.dry_run).await
        }
        Commands::Desktop { subcommand, name, live } => {
            commands::desktop::run(&config_dir, Some(subcommand), Some(name), live, cli.verbose, cli.dry_run
            ).await
        }
        Commands::Update { all } => {
            update::run(&config_dir, all, cli.verbose, cli.dry_run).await
        }
        Commands::Shell { packages, flake, list, pure } => {
            shell::run(packages, flake, list, pure).await
        }
        Commands::Dev { stack, name, list } => {
            dev::run(&stack, name, list).await
        }
        Commands::Container { action, target, packages, tag, registry } => {
            container::run(&action, target, packages, tag, registry).await
        }
    };

    if let Err(e) = result {
        eprintln!("{} {}", "error:".red().bold(), e);
        if cli.verbose {
            let backtrace = e.backtrace();
            eprintln!("{}", backtrace);
        }
        process::exit(1);
    }
}
