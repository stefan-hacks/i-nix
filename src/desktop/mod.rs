//! Desktop Environment (DE) detection and declarative generation.
//!
//! This module ports nix-my-gnome's capabilities into native Rust,
//! then extends them to cover all major desktop environments.

pub mod detect;
pub mod gnome;
pub mod generator;
