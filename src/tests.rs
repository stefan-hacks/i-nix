#[cfg(test)]
mod tests {
    use std::path::Path;
    use tempfile::TempDir;

    /// Initialize a temporary i-nix flake for testing.
    async fn setup_test_flake() -> (TempDir, String) {
        let temp = TempDir::new().unwrap();
        let config_dir = temp.path().to_str().unwrap().to_string();

        // Initialize flake
        let _ = crate::commands::init::run(
            &config_dir,
            Some("test-host".to_string()),
            false,
            true,
            false,
            false,
        ).await;

        (temp, config_dir)
    }

    #[tokio::test]
    async fn test_init_creates_flake() {
        let (temp, config_dir) = setup_test_flake().await;
        let flake_path = Path::new(&config_dir).join("flake/flake.nix");
        assert!(flake_path.exists());
        drop(temp);
    }

    #[tokio::test]
    async fn test_install_adds_package() {
        let (temp, config_dir) = setup_test_flake().await;

        let _ = crate::commands::install::run(
            &config_dir,
            vec!["firefox".to_string()],
            false,
            false,
            None,
            false,
            false,
        ).await;

        let systems_nix = Path::new(&config_dir).join("flake/systems/test-host/default.nix");
        let content = std::fs::read_to_string(systems_nix).unwrap();
        assert!(content.contains("firefox"));
        drop(temp);
    }

    #[tokio::test]
    async fn test_remove_package() {
        let (temp, config_dir) = setup_test_flake().await;

        // Install then remove
        let _ = crate::commands::install::run(
            &config_dir,
            vec!["vim".to_string()],
            false,
            false,
            None,
            false,
            false,
        ).await;

        let _ = crate::commands::remove::run(
            &config_dir,
            vec!["vim".to_string()],
            false,
            false,
            false,
        ).await;

        let systems_nix = Path::new(&config_dir).join("flake/systems/test-host/default.nix");
        let content = std::fs::read_to_string(systems_nix).unwrap();
        assert!(!content.contains("vim"));
        drop(temp);
    }

    #[tokio::test]
    async fn test_enable_service() {
        let (temp, config_dir) = setup_test_flake().await;

        let _ = crate::commands::enable::run(
            &config_dir,
            Some("ssh".to_string()),
            false,
            true,
            false,
            false,
            false,
            false,
        ).await;

        let systems_nix = Path::new(&config_dir).join("flake/systems/test-host/default.nix");
        let content = std::fs::read_to_string(systems_nix).unwrap();
        assert!(content.contains("services.openssh.enable = true"));
        drop(temp);
    }

    #[tokio::test]
    async fn test_desktop_generate_gnome() {
        let (temp, config_dir) = setup_test_flake().await;

        let _ = crate::commands::desktop::run(
            &config_dir,
            Some("generate".to_string()),
            Some("gnome".to_string()),
            false,
            false,
            false,
        ).await;

        let gnome_dir = Path::new(&config_dir).join("flake/users/user/desktop/gnome-settings");
        assert!(gnome_dir.exists());
        drop(temp);
    }

    #[tokio::test]
    async fn test_desktop_detect() {
        let (temp, config_dir) = setup_test_flake().await;

        let result = crate::commands::desktop::run(
            &config_dir,
            Some("detect".to_string()),
            None,
            false,
            false,
            false,
        ).await;

        assert!(result.is_ok());
        drop(temp);
    }

    #[tokio::test]
    async fn test_dconf_parser_empty() {
        let doc = crate::desktop::gnome::parse_dconf_dump("").unwrap();
        assert!(doc.sections.is_empty());
    }

    #[tokio::test]
    async fn test_dconf_parser_simple_section() {
        let input = r#"[org/gnome/desktop/interface]
clock-format='24h'

"#;
        let doc = crate::desktop::gnome::parse_dconf_dump(input).unwrap();
        assert_eq!(doc.sections.len(), 1);
        assert_eq!(doc.sections[0].path, "org/gnome/desktop/interface");
        assert!(doc.sections[0].settings.contains_key("clock-format"));
    }

    #[tokio::test]
    async fn test_dconf_parser_typed_value() {
        let input = "[org/gnome/mutter]\ncheck-alive-timeout=uint32 5000\n";
        let doc = crate::desktop::gnome::parse_dconf_dump(input).unwrap();
        assert_eq!(doc.sections.len(), 1);
        assert!(doc.sections[0].settings.contains_key("check-alive-timeout"));
    }

    #[tokio::test]
    async fn test_dconf_parser_array() {
        let input = "[org/gnome/shell]\nfavorite-apps=['firefox.desktop', 'org.gnome.Nautilus.desktop']\n";
        let doc = crate::desktop::gnome::parse_dconf_dump(input).unwrap();
        assert_eq!(doc.sections.len(), 1);
        let val = doc.sections[0].settings.get("favorite-apps").unwrap();
        assert!(val.contains("firefox.desktop"));
    }

    #[tokio::test]
    async fn test_dconf_parser_bool() {
        let input = "[org/gtk/settings/file-chooser]\nsort-directories-first=true\n";
        let doc = crate::desktop::gnome::parse_dconf_dump(input).unwrap();
        assert_eq!(doc.sections.len(), 1);
        assert_eq!(doc.sections[0].settings.get("sort-directories-first"), Some(&"true".to_string()));
    }

    #[tokio::test]
    async fn test_nixast_package_roundtrip() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("test.nix");

        let source = r#"{ pkgs, ... }:

{
  environment.systemPackages = with pkgs; [
    firefox
    git
    vim
  ];
}
"#;
        std::fs::write(&file, source).unwrap();

        let packages = crate::nixast::read_system_packages(&file).unwrap();
        assert_eq!(packages, vec!["firefox", "git", "vim"]);

        let entries: Vec<crate::nixast::PackageEntry> = packages
            .into_iter()
            .map(|name| crate::nixast::PackageEntry { name, comment: None })
            .collect();

        crate::nixast::write_system_packages(&file, &entries).unwrap();
        let content = std::fs::read_to_string(&file).unwrap();
        assert!(content.contains("firefox"));
        assert!(content.contains("git"));
        assert!(content.contains("vim"));
    }
}
