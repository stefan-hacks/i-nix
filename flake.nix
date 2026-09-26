{
  description = "i-nix — Yet Another Nix Helper. Imperative UX for declarative Nix/NixOS systems.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" ];
        };
        i-nix = pkgs.rustPlatform.buildRustPackage {
          pname = "i-nix";
          version = "0.1.0";
          src = ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          nativeBuildInputs = with pkgs; [ pkg-config ];
          buildInputs = with pkgs; [ openssl ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.darwin.apple_sdk.frameworks.Security
            pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
          ];
          meta = with pkgs.lib; {
            description = "Imperative UX for declarative Nix/NixOS systems";
            homepage = "https://github.com/stefan-hacks/i-nix";
            license = licenses.mit;
            maintainers = [ ];
            mainProgram = "i-nix";
          };
        };
      in
      {
        packages.default = i-nix;
        packages.i-nix = i-nix;

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            cargo-watch
            clippy
            rustfmt
            pkg-config
            openssl
          ];
        };

        apps.default = flake-utils.lib.mkApp {
          drv = i-nix;
        };
      });
}