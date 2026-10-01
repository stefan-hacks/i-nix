# ❄️ i-nix

> **Imperative UX for declarative Nix/NixOS systems.**

Modern, colorized, beautiful terminal output. Recursive directory discovery. Production-grade flake imports with deprecated-pattern warnings.

---

## 🚀 Quick Start

**With flakes enabled:**
```bash
nix run github:stefan-hacks/i-nix -- init --hostname my-machine --system
```

**Without flakes enabled:**
```bash
nix run --extra-experimental-features 'nix-command flakes' github:stefan-hacks/i-nix -- init --hostname my-machine --system
```

---

## ✨ What is i-nix?

`i-nix` is **not** another package manager. It is a **UX layer** that compiles imperative commands into declarative NixOS/Home Manager configuration — implemented natively in Rust with zero external dependencies.

Every action becomes part of a clean, reproducible Nix configuration in `~/.config/i-nix/`.

```
USER
 │
 ▼
┌───────────────────────────────┐
│            i-nix              │
│                               │
│  install   remove   search  │
│  enable    disable  list    │
│  adopt     import   doctor  │
│  apply     diff     sync    │
│  os        home     clean   │
│  desktop   shell     tui    │
└──────────────┬────────────────┘
               │
               ▼
┌───────────────────────────────┐
│      ~/.config/i-nix/        │
│                               │
│  flake.nix    flake.lock    │
│  hosts/        systems/      │
│  users/        profiles/    │
│  modules/      overlays/     │
│  pkgs/         lib/          │
└───────────────────────────────┘
```

---

## 🎨 Visual Features

i-nix uses a **Charmbracelet-inspired** color palette:

| Feature | Description |
|---------|-------------|
| 🎨 **Color palette** | Dracula/Gum-inspired: pink `#FF79C6`, purple `#BD93F9`, cyan `#8BE9FD`, green `#50FA7B` |
| 📊 **Progress bars** | Gradient bars with elapsed time |
| 🔄 **Spinners** | Animated `◐ ◓ ◑ ◒ → ✓` sequences |
| 🏷️ **Badges** | Colored completion badges (🟢 success, 🟡 warning, 🔴 error) |
| 📦 **Icons** | 25+ Unicode symbols for files, frameworks, and status |
| 🖼️ **Banners** | Centered boxed headers per command |
| 📋 **Tables** | Styled key-value and multi-column output |

---

## 📦 Installation

### Via Nix (recommended)

```bash
# With flakes enabled
nix run github:stefan-hacks/i-nix -- --help

# Without flakes enabled
nix run --extra-experimental-features 'nix-command flakes' github:stefan-hacks/i-nix -- --help
```

### Add to your flake

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    i-nix = {
      url = "github:stefan-hacks/i-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, i-nix, ... }@inputs: {
    nixosConfigurations.my-host = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      specialArgs = { inherit inputs; };
      modules = [ ./configuration.nix ];
    };
  };
}
```

Then in `configuration.nix`:
```nix
{ pkgs, inputs, ... }:
{
  environment.systemPackages = [ inputs.i-nix.packages.${pkgs.system}.default ];
}
```

### Build from source

```bash
git clone https://github.com/stefan-hacks/i-nix.git
cd i-nix
cargo build --release
sudo cp target/release/i-nix /usr/local/bin/
```

### Shell alias

| Shell | File | Alias |
|-------|------|-------|
| Bash | `~/.bashrc` | `alias i='i-nix'` |
| Zsh  | `~/.zshrc`  | `alias i='i-nix'` |
| Fish | `~/.config/fish/config.fish` | `abbr --add i i-nix` |

---

## 🛠️ Commands

### Core

| Command | Description | Example |
|---------|-------------|---------|
| `init` | Initialize flake structure | `i-nix init --hostname laptop --system` |
| `install` | Add packages to system config | `i-nix install firefox git vim` |
| `install --user` | Add packages to user config | `i-nix install --user kitty neovim` |
| `remove` | Remove packages from config | `i-nix remove vim` |
| `list` | Show all managed packages | `i-nix list` |
| `search` | Search nixpkgs | `i-nix search firefox` |

### Services & Programs

| Command | Description | Example |
|---------|-------------|---------|
| `enable` | Enable a service or program | `i-nix enable ssh` |
| `enable --user` | Enable a user program | `i-nix enable --user kitty` |
| `enable --program` | Enable a system program | `i-nix enable hyprland --program` |
| `disable` | Disable a service or program | `i-nix disable ssh` |

### System Operations (nh os)

| Command | Description | Example |
|---------|-------------|---------|
| `os switch` | Build, activate, and set boot default | `i-nix os switch --hostname laptop` |
| `os boot` | Build and set boot default | `i-nix os boot --hostname laptop` |
| `os test` | Build and activate (not boot default) | `i-nix os test --hostname laptop` |
| `os build` | Build only (no activation) | `i-nix os build --hostname laptop` |
| `os info` | List available generations | `i-nix os info --hostname laptop` |
| `os repl` | Load system config in Nix REPL | `i-nix os repl --hostname laptop` |

### Home Manager (nh home)

| Command | Description | Example |
|---------|-------------|---------|
| `home switch` | Build and activate HM config | `i-nix home switch --user stefan-hacks --hostname laptop` |
| `home build` | Build HM config only | `i-nix home build --user stefan-hacks --hostname laptop` |
| `home repl` | Load HM config in REPL | `i-nix home repl --user stefan-hacks --hostname laptop` |

### Cleanup (nh clean)

| Command | Description | Example |
|---------|-------------|---------|
| `clean all` | Clean all profiles | `i-nix clean all` |
| `clean user` | Clean current user's profiles | `i-nix clean user` |
| `clean profile` | Clean a specific profile | `i-nix clean profile --profile my-profile` |

### Desktop Environments

| Command | Description | Example |
|---------|-------------|---------|
| `desktop detect` | Auto-detect current DE | `i-nix desktop detect` |
| `desktop list` | Show all supported DEs | `i-nix desktop list` |
| `desktop generate` | Generate declarative config | `i-nix desktop generate gnome` |
| `desktop apply` | Wire DE into flake | `i-nix desktop apply` |

### Import (NEW)

| Command | Description | Example |
|---------|-------------|---------|
| `import` | Import existing flake repo into i-nix | `i-nix import ~/my-nix-config` |
| `import --output` | Specify output directory | `i-nix import ~/config --output ~/.config/i-nix` |
| `import --no-fmt` | Skip `nix fmt` on output | `i-nix import ~/config --no-fmt` |
| `import --mirror` | Mirror source file tree | `i-nix import ~/config --mirror` |

### Other

| Command | Description | Example |
|---------|-------------|---------|
| `diff` | Show pending changes | `i-nix diff` |
| `apply` | Evaluate, build, switch | `i-nix apply` |
| `apply --test` | Test config without switching | `i-nix apply --test` |
| `apply --remote` | Deploy to remote host | `i-nix apply --remote root@server` |
| `rollback` | Rollback to previous generation | `i-nix rollback` |
| `rollback -i` | Interactive generation browser | `i-nix rollback -i` |
| `rollback -g` | Rollback to specific generation | `i-nix rollback -g 42` |
| `update` | Update flake inputs | `i-nix update` |
| `edit` | Open config in $EDITOR | `i-nix edit packages` |
| `why` | Show dependency chain | `i-nix why firefox` |
| `adopt` | Discover current system state | `i-nix adopt --write` |
| `doctor` | Diagnose Nix/NixOS health | `i-nix doctor` |
| `sync` | Detect & re-import changes | `i-nix sync --yes` |
| `shell` | Start dev shell | `i-nix shell python nodejs` |
| `dev` | Scaffold project flake | `i-nix dev rust my-project` |
| `container` | Build OCI images | `i-nix container build .#my-image` |
| `tui` | Launch interactive terminal UI | `i-nix tui` |

---

## 📥 Importing Existing Configurations

Migrate any existing NixOS/Home Manager flake into i-nix's optimal structure:

```bash
# Import to ~/.config/i-nix (default)
i-nix import ~/my-existing-nix-config

# Import to specific directory
i-nix import ~/my-existing-nix-config --output ~/my-new-config

# Skip nix fmt
i-nix import ~/my-existing-nix-config --no-fmt

# Mirror source tree (copy all files, not just .nix)
i-nix import ~/my-existing-nix-config --mirror
```

**What the importer detects:**
- `flake.nix` inputs (nixpkgs, home-manager, flake-parts, disko, sops-nix, impermanence, etc.)
- **All directories and subdirectories** recursively (full dendritic tree display)
- NixOS host declarations and system configuration
- Home Manager user configurations
- Hardware configuration and disko layouts
- Desktop environments (GNOME, KDE, Hyprland, Sway, etc.)
- Shell configurations (bash, zsh, fish)

**Deprecated pattern warnings (NixOS 26.05/26.11):**
- Old scripted initrd (`boot.initrd.systemd.enable = false`)
- `linux_hardened` (removed in 26.05)
- `nodePackages.*` / `node2nix` (removed in 26.05)
- Pinned old kernels (`linuxPackages_6_15` etc.)
- `fetchFromSavannah` (deprecated)
- `nixexprs.tar.xz` (retiring 2027-12-31)

**What it generates:**
- A clean, `nix fmt`-ready `flake.nix`
- `hosts/default.nix` with all machine declarations
- `systems/<hostname>/` with `default.nix`, `hardware.nix`, `network.nix`
- `users/<username>/` with:
  - `packages/`, `programs/`, `services/`
  - `desktop/`, `shells/`, `secrets/`
- `profiles/` for composable feature bundles (core, desktop, dev, gaming, server)
- `modules/`, `overlays/`, `pkgs/`, `lib/` scaffolding
- `disko/` if disk partitioning config detected

---

## 🗂️ Configuration Structure

```text
~/.config/i-nix/
├── flake.nix              # flake-parts entrypoint
├── flake.lock             # pinned inputs
├── .gitignore
├── hosts/
│   └── default.nix        # multi-host registry
├── systems/
│   └── ghost/
│       ├── default.nix    # system config
│       ├── hardware.nix   # hardware-configuration
│       └── network.nix    # network settings
├── users/
│   └── stefan-hacks/
│       ├── default.nix    # home entrypoint
│       ├── packages/
│       │   └── default.nix
│       ├── programs/
│       │   └── default.nix
│       ├── services/
│       │   └── default.nix
│       ├── desktop/
│       │   └── default.nix
│       ├── shells/
│       │   └── default.nix
│       └── secrets/
│           └── default.nix
├── profiles/
│   ├── core.nix
│   ├── desktop.nix
│   ├── development.nix
│   ├── gaming.nix
│   └── server.nix
├── modules/
│   ├── system/
│   │   └── default.nix
│   └── home/
│       └── default.nix
├── overlays/
│   └── default.nix
├── pkgs/
│   └── default.nix
├── lib/
│   └── default.nix
└── disko/
    └── default.nix        # if disko detected
```

---

## 🧪 How It Works

### No Regex Parsing

i-nix uses bracket-depth tracking to safely parse and modify Nix files. It preserves comments, formatting, and structure.

### Dendritic Flake

Each host and user is isolated. Add a host by creating a new directory under `systems/`. The flake automatically picks it up.

### Desktop Environment Integration

Supports GNOME, KDE Plasma, Hyprland, Sway, i3, Niri, XFCE, Cinnamon, Pop!_OS, QuickShell, Noctalia, DankLinux.

---

## 📋 Requirements

- Nix or NixOS installed
- `nix-command` and `flakes` experimental features enabled (or pass `--extra-experimental-features`)
- Rust 1.70+ (for building from source)

---

## 📄 License

MIT
