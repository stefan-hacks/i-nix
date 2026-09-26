# ❄️ i-nix

**Imperative UX for declarative Nix/NixOS systems.**

> **Quick start:** `nix run github:stefan-hacks/i-nix -- init --hostname my-machine --system`

---

## What is i-nix?

`i-nix` is **not** another package manager. It is a **UX layer** that compiles imperative commands into declarative NixOS/Home Manager configuration — inspired by my tool `pdrx` (Portable Declarative Linux) and the workflow of `nh` (Yet Another Nix Helper) but implemented natively in Rust with zero external dependencies.

**Key principle:** Every action you take becomes part of a clean, reproducible Nix configuration stored in `~/.config/i-nix/flake/`.

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
│ ~/.config/    │
│ i-nix/flake/  │
│               │
│ flake.nix     │
│ systems/      │
│ users/        │
│ desktop/      │
└───────┬───────┘
        │
        ▼
┌───────────────┐
│ nixos-rebuild │
│ home-manager  │
│ switch        │
└───────────────┘
```

---

## Installation

### Via Nix (recommended)

```bash
nix run github:stefan-hacks/i-nix -- --help
```

### Build from source

```bash
git clone https://github.com/stefan-hacks/i-nix.git
cd i-nix
cargo build --release
sudo cp target/release/i-nix /usr/local/bin/
```

---

## Commands

### Core

| Command | Description | Example |
|---|---|---|
| `init` | Initialize flake structure | `i-nix init --hostname laptop --system` |
| `install` | Add packages to system config | `i-nix install firefox git vim` |
| `install --user` | Add packages to user config | `i-nix install --user kitty neovim` |
| `remove` | Remove packages from config | `i-nix remove vim` |
| `list` | Show all managed packages | `i-nix list` |
| `search` | Search nixpkgs | `i-nix search firefox` |

### Services & Programs

| Command | Description | Example |
|---|---|---|
| `enable` | Enable a service or program | `i-nix enable ssh` |
| `enable --user` | Enable a user program | `i-nix enable --user kitty` |
| `enable --program` | Enable a system program | `i-nix enable hyprland --program` |
| `disable` | Disable a service or program | `i-nix disable ssh` |

### System Operations (nh os)

| Command | Description | Example |
|---|---|---|
| `os switch` | Build, activate, and set boot default | `i-nix os switch --hostname laptop` |
| `os boot` | Build and set boot default | `i-nix os boot --hostname laptop` |
| `os test` | Build and activate (not boot default) | `i-nix os test --hostname laptop` |
| `os build` | Build only (no activation) | `i-nix os build --hostname laptop` |
| `os info` | List available generations | `i-nix os info --hostname laptop` |
| `os repl` | Load system config in Nix REPL | `i-nix os repl --hostname laptop` |

### Home Manager (nh home)

| Command | Description | Example |
|---|---|---|
| `home switch` | Build and activate HM config | `i-nix home switch --user lin --hostname laptop` |
| `home build` | Build HM config only | `i-nix home build --user lin --hostname laptop` |
| `home repl` | Load HM config in REPL | `i-nix home repl --user lin --hostname laptop` |

### Cleanup (nh clean)

| Command | Description | Example |
|---|---|---|
| `clean all` | Clean all profiles | `i-nix clean all` |
| `clean user` | Clean current user's profiles | `i-nix clean user` |
| `clean profile` | Clean a specific profile | `i-nix clean profile --profile my-profile` |

### Desktop Environments

| Command | Description | Example |
|---|---|---|
| `desktop detect` | Auto-detect current DE | `i-nix desktop detect` |
| `desktop list` | Show all supported DEs | `i-nix desktop list` |
| `desktop generate` | Generate declarative config | `i-nix desktop generate gnome` |
| `desktop apply` | Wire DE into flake | `i-nix desktop apply` |

### Other

| Command | Description | Example |
|---|---|---|
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

## Configuration Structure

```text
~/.config/i-nix/flake/
├── flake.nix              # Entry point
├── flake.lock             # Pin inputs
├── hosts/
│   └── default.nix        # Multi-host registry
├── systems/
│   └── laptop/
│       ├── default.nix    # System config
│       ├── packages.nix   # System packages
│       └── services.nix   # Enabled services
└── users/
    └── lin/
        ├── default.nix    # User config
        ├── packages.nix   # User packages
        └── programs.nix   # Enabled programs
```

---

## How It Works

### No Regex

i-nix uses bracket-depth tracking to safely parse and modify Nix files. It preserves comments, formatting, and structure.

### Dendritic Flake

Each host and user is isolated. Add a host by creating a new directory under `systems/`. The flake automatically picks it up.

### Desktop Environment Integration

Supports GNOME, KDE Plasma, Hyprland, Sway, i3, Niri, XFCE, Cinnamon, Pop!_OS, QuickShell, Noctalia, DankLinux.

---

## License

MIT
