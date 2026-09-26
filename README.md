# i-nix

**Imperative UX for declarative Nix/NixOS systems.**

> **Quick start:** `nix run github:stefan-hacks/i-nix -- init --hostname my-machine --system`

## What is i-nix?

`i-nix` is **not** another package manager. It is a **UX layer** that compiles imperative commands into declarative NixOS/Home Manager configuration — inspired by the beautiful workflow of `nh` (Yet Another Nix Helper) but implemented natively in Rust with no external dependencies.

**Key principle:** Every action you take becomes part of a clean, reproducible Nix configuration stored in `$HOME/.config/i-nix/flake/`.

```text
USER
 │
 ▼
┌─────────────┐
│    i-nix    │
│             │
│ install     │
│ remove      │
│ enable      │
│ disable     │
│ adopt       │
└──────┬──────┘
       │
       ▼
┌───────────────┐
│ Nix config    │
│ SOURCE OF     │
│ TRUTH         │
│               │
│ flake.nix     │
│ flake.lock    │
└───────┬───────┘
       │
       ▼
  nixos-rebuild
       │
       ▼
  NEW GENERATION
```

---

## Philosophy

**"Imperative commands. Declarative systems."**

You type traditional Linux commands. i-nix translates them into Nix language. The generated flake is the **source of truth**. You can edit it by hand; i-nix will detect and sync changes.

This creates a gradient for learning:

```text
Traditional Linux
      │
      ▼
i-nix install firefox
      │
      ▼
User sees the generated Nix
      │
      ▼
User starts modifying it
      │
      ▼
Full NixOS expertise
```

---

## Quick Start

```bash
# 1. Initialize i-nix
i-nix init --hostname my-machine --system

# 2. Install packages
i-nix install firefox git vim ripgrep
i-nix install --user kitty neovim starship

# 3. Enable services and programs
i-nix enable ssh
i-nix enable docker
i-nix enable --user kitty

# 4. See what changed
i-nix diff

# 5. Apply changes
i-nix apply
```

---

## Full Command Reference

| Command | Description | Example |
|---|---|---|
| `init` | Initialize flake structure | `i-nix init --hostname laptop --system` |
| `install` | Add packages to system config | `i-nix install firefox git vim` |
| `install --user` | Add packages to user config | `i-nix install --user kitty neovim` |
| `remove` | Remove packages from config | `i-nix remove vim` |
| `enable` | Enable a service or program | `i-nix enable ssh` |
| `enable --user` | Enable a user program | `i-nix enable --user kitty` |
| `enable --program` | Enable a system program | `i-nix enable hyprland --program` |
| `disable` | Disable a service or program | `i-nix disable ssh` |
| `disable --user` | Disable a user program | `i-nix disable --user kitty` |
| `search` | Search nixpkgs | `i-nix search firefox` |
| `list` | Show all managed packages | `i-nix list` |
| `diff` | Show pending changes | `i-nix diff` |
| `apply` | Evaluate, build, switch | `i-nix apply` |
| `rollback` | List generations / rollback | `i-nix rollback` |
| `update` | Update flake inputs | `i-nix update` |
| `edit` | Open config in $EDITOR | `i-nix edit packages` |
| `why` | Show dependency chain | `i-nix why firefox` |
| `adopt` | Discover current system state | `i-nix adopt --write` |
| `doctor` | Diagnose Nix/NixOS health | `i-nix doctor` |
| `shell` | Start dev shell | `i-nix shell python nodejs` |
| `dev` | Scaffold project flake | `i-nix dev rust my-project` |
| `container` | Build OCI images | `i-nix container build .#my-image` |
| `desktop` | Manage desktop environments | `i-nix desktop generate gnome` |
| `desktop detect` | Auto-detect current DE | `i-nix desktop detect` |
| `desktop list` | Show all supported DEs | `i-nix desktop list` |
| `desktop apply` | Wire DE into flake | `i-nix desktop apply` |
| `sync` | Detect & re-import changes | `i-nix sync --yes` |
| `tui` | Launch interactive terminal UI | `i-nix tui` |

---

## Directory Structure

Production-grade, dendritic, multi-host structure:

```text
$HOME/.config/i-nix/
└── flake/
    ├── flake.nix              # flake-parts entrypoint (imports ./hosts)
    ├── flake.lock             # Pinned inputs
    │
    ├── hosts/
    │   └── default.nix        # nixosConfigurations registry (all machines)
    │
    ├── systems/
    │   ├── _common.nix        # Shared: boot, locale, nix settings
    │   └── laptop/
    │       ├── default.nix    # Host-specific: packages, services, programs
    │       └── hardware.nix   # Hardware config (nixos-generate-config)
    │
    ├── users/
    │   └── lin/
    │       ├── default.nix    # Entrypoint: imports packages/programs/services/desktop/shells/secrets
    │       ├── packages/
    │       │   └── default.nix # home.packages (i-nix install --user)
    │       ├── programs/
    │       │   └── default.nix # programs.firefox.enable, programs.kitty.enable...
    │       ├── services/
    │       │   └── default.nix # User-level systemd (syncthing, mpd, ssh-agent)
    │       ├── desktop/
    │       │   ├── default.nix # Imports ONE of: gnome.nix / kde.nix / hyprland.nix / sway.nix
    │       │   ├── gnome.nix   # ← Generated by nix-my-gnome dconf import
    │       │   ├── kde.nix
    │       │   ├── hyprland.nix
    │       │   └── sway.nix
    │       ├── shells/
    │       │   └── default.nix # bash/zsh/fish config with starship, direnv
    │       ├── secrets/
    │       │   └── default.nix # sops-nix or agenix encrypted secrets
    │       └── dotfiles/        # Plain dotfiles NOT managed by HM
    │
    ├── modules/
    │   ├── system/
    │   │   └── default.nix     # Reusable NixOS modules (network, firewall, security)
    │   └── home/
    │       └── default.nix     # Reusable HM modules
    │
    ├── profiles/               # Composable feature bundles
    │   ├── core.nix            # Minimal base (ssh, curl, vim, htop)
    │   ├── desktop.nix         # Full desktop (fonts, pipewire, bluetooth, cups)
    │   ├── development.nix     # Dev tools (docker, vscode, git, gh)
    │   ├── gaming.nix          # Gaming (steam, proton, mangohud, gamemode)
    │   └── server.nix          # Server (nginx, postgresql, docker, fail2ban)
    │
    ├── overlays/
    │   └── default.nix         # Custom nixpkgs overrides
    ├── pkgs/
    │   └── default.nix         # Custom package definitions
    └── lib/
        └── helpers.nix         # Nix helper functions
```

### Multi-host scaling

```bash
# Add a second machine
i-nix init --hostname server --system
```

This creates `systems/server/` and adds the host to `hosts/default.nix`. Each host inherits `_common.nix` but has its own packages, services, and programs.

```text
systems/
├── _common.nix
├── laptop/
│   ├── default.nix      # i-nix install firefox → here
│   └── hardware.nix
└── server/
    ├── default.nix      # i-nix install nginx → here
    └── hardware.nix
```

---

## Desktop Environment Integration

i-nix includes per-DE configuration files compatible with `nix-my-gnome`:

```text
users/lin/desktop/
├── default.nix    # imports ONE of: gnome.nix / kde.nix / hyprland.nix / sway.nix
├── gnome.nix      # Generated from dconf dump
├── kde.nix
├── hyprland.nix
└── sway.nix
```

Switch desktops:

```bash
# Edit users/lin/desktop/default.nix:
imports = [ ./hyprland.nix ];   # Swap for ./gnome.nix, ./sway.nix, etc.
i-nix apply
```

---

## Profile Composition

Enable entire feature bundles in one line:

```nix
# In systems/laptop/default.nix
imports = [
  ../profiles/core.nix
  ../profiles/desktop.nix
  ../profiles/development.nix
  # ../profiles/gaming.nix   # Uncomment when needed
];
```

| Profile | What it adds |
|---|---|
| `core` | ssh, curl, vim, htop, git, tmux |
| `desktop` | fonts, pipewire, bluetooth, cups, printing |
| `development` | docker, vscode, nodejs, python, rust |
| `gaming` | steam, proton, mangohud, gamemode |
| `server` | nginx, postgresql, fail2ban, docker |

---

## Development Tooling

### Dev shells

```bash
# Enter a shell with packages
i-nix shell python nodejs rust

# Or use a project's flake.nix
i-nix shell --flake .#default
```

### Project scaffolding

```bash
i-nix dev rust my-project     # Cargo + flake.nix + devShell
i-nix dev node my-api         # package.json + flake.nix
i-nix dev python my-ml        # requirements.txt + flake.nix
i-nix dev go my-cli           # go.mod + flake.nix
i-nix dev haskell my-lib     # cabal + stack + flake.nix
```

### Container builds

```bash
i-nix container build .#my-image
i-nix container push .#my-image --registry ghcr.io
```

---

## The `adopt` Command

Turn a messy NixOS machine into a beautiful declarative configuration:

```bash
$ i-nix adopt --write

╭────────────────────────────────────╮
│ i-nix adopt                        │
├────────────────────────────────────┤
│                                    │
│ ✓ 147 packages discovered          │
│ ✓ GNOME detected                   │
│ ✓ PipeWire detected                │
│ ✓ Bluetooth detected               │
│ ✓ Docker detected                  │
│ ✓ SSH detected                     │
│ ✓ Kitty detected                   │
│                                    │
│ Configuration can be generated.    │
╰────────────────────────────────────╯
```

---

## The `doctor` Command

```bash
$ i-nix doctor

System
  ✓ NixOS 26.05
  ✓ nix 2.x
  ✓ flakes enabled

Configuration
  ✓ flake found
  ✓ configuration evaluates
  ⚠ 3 deprecated options
  ⚠ 1 unused input

Packages
  ✓ 84 declarative packages
  ⚠ 3 packages installed imperatively

Recommendation:
  i-nix adopt firefox
```

---

## Desktop Environments

i-nix includes dedicated per-desktop directories under `users/<name>/desktop/`.

### GNOME + nix-my-gnome

**Always use the `-s` (split) flag** so nmg creates a well-organized directory instead of one monolithic file:

```bash
# Inside your flake directory
cd users/lin/desktop

# Generate GNOME settings with SPLIT mode
dconf dump / | nix run github:stefan-hacks/nix-my-gnome -- -s -o ./gnome-settings
```

This produces:
```text
gnome-settings/
  default.nix         # imports all category modules
  shell-extensions.nix
  gtk.nix             # themes, fonts, interface
  mutter.nix          # window manager
  window-manager.nix  # keybindings
  input-devices.nix   # mouse, touchpad
  notifications.nix
  settings-daemon.nix
  ...
```

Then uncomment in `users/lin/desktop/default.nix`:
```nix
{
  imports = [ ./gnome-settings ];
}
```

### Other DEs

```nix
# KDE Plasma
imports = [ ./kde.nix ];

# Hyprland
imports = [ ./hyprland.nix ];

# Sway
imports = [ ./sway.nix ];
```

---

## `desktop` Command

```bash
i-nix desktop detect              # Detect current DE
i-nix desktop list                # List all supported DEs
i-nix desktop generate gnome    # Scaffold GNOME config (with dconf parser)
i-nix desktop generate hyprland # Scaffold Hyprland config
i-nix desktop apply             # Wire DE imports into flake
```

**Supported DEs:** GNOME, KDE Plasma, Hyprland, Sway, i3, Niri, XFCE, Cinnamon, Pop!_OS, QuickShell, Noctalia, DankLinux.

GNOME config uses the native Rust dconf parser (equivalent to `nix-my-gnome -s`) producing categorized modules:
```text
gnome-settings/
  ├── default.nix
  ├── gtk.nix
  ├── shell.nix
  └── shell-extensions.nix
```

## `sync` Command

```bash
i-nix sync                    # Show all divergences
i-nix sync packages          # Show imperative packages only
i-nix sync services          # Show enabled services only
i-nix sync --yes             # Auto-import all divergences
```

Detects and re-imports:
- **Imperative packages** (`nix-env -i` or `apt install`)
- **User-installed packages** (`home.packages`)
- **Enabled services** (`systemctl enable`)
- **Config file drift** (manual `.nix` edits)

---

## How it Works

### No Regex

i-nix uses bracket-depth tracking to safely parse and modify Nix files. It preserves comments, formatting, and structure. The AST engine (`nixast.rs`) handles:

- `environment.systemPackages` list manipulation
- `home.packages` list manipulation
- `services.*.enable = true` attribute insertion
- `programs.*.enable = true` attribute insertion

### Git Integration

Every `apply` stages changes, evaluates the flake, builds the system, and commits:

```bash
i-nix apply
# → git add -A
# → nix flake check
# → nixos-rebuild switch --flake .#hostname
# → git commit -m "i-nix apply: <changes>"
```

### Manual Edits

You can always edit the generated Nix directly:

```bash
nvim ~/.config/i-nix/flake/systems/laptop/default.nix
# ...make changes...
i-nix apply   # Detects and applies your manual changes
```

---

## Building from Source

```bash
git clone https://github.com/stefan-hacks/i-nix.git
cd i-nix
cargo build --release
sudo cp target/release/i-nix /usr/local/bin/
```

---

## Roadmap

- [x] `init`, `install`, `remove`, `list`, `search`
- [x] `enable`, `disable` (services and programs)
- [x] `diff`, `apply`, `rollback`
- [x] `adopt`, `doctor`
- [x] `shell`, `dev`, `container`
- [x] Multi-host support
- [x] Profile bundles
- [x] `os` (switch, boot, test, build, info) — nh-style NixOS operations
- [x] `home` (switch, build, repl) — Home Manager operations
- [x] `clean` (all, user, profile) — Enhanced garbage collection
- [x] `sync` — detect manual edits and sync state
- [ ] `template` — shareable configuration templates
- [ ] Web UI for browsing/searching packages

---

## License

MIT / Apache-2.0
