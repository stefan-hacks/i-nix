# i-nix

**Imperative UX for declarative Nix/NixOS systems.**

> Imperative commands. Declarative systems.

Use NixOS like a traditional Linux distribution (`apt install`, `dnf install`, `pacman -S`) — but every action automatically becomes part of a clean, reproducible Nix configuration stored in `$HOME/.config/i-nix/flake/`.

---

## What is i-nix?

`i-nix` is **not** another package manager. It is a **UX layer** that compiles imperative commands into declarative NixOS/Home Manager configuration.

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
│ adopt       │
└──────┬──────┘
       │
       ▼
┌───────────────┐
│ Nix config    │
│ SOURCE OF     │
│ TRUTH         │
│ (flake.nix)   │
└───────┬───────┘
        │
        ▼
    nix eval
        │
        ▼
nixos-rebuild
        │
        ▼
  NEW GENERATION
```

---

## Philosophy

| Traditional Linux | i-nix | Pure Nix |
|---|---|---|
| `apt install firefox` | `i-nix install firefox` | Edit `configuration.nix`, `nixos-rebuild switch` |
| Easy to learn, hard to reproduce | Best of both worlds | Hard to learn, perfectly reproducible |

`i-nix` creates a **gradient**:

```text
Traditional Linux
      │
      ▼
i-nix install firefox
      │
      ▼
See generated configuration.nix
      │
      ▼
User starts modifying Nix directly
      │
      ▼
Full NixOS expertise
```

---

## Features

- ✅ **`i-nix init`** — Initialize a new declarative configuration (flake-parts + dendritic structure)
- ✅ **`i-nix install <pkg>`** — Add packages to `environment.systemPackages`
- ✅ **`i-nix install --user <pkg>`** — Add packages to `home.packages` (Home Manager)
- ✅ **`i-nix remove <pkg>`** — Remove packages from declarative config
- ✅ **`i-nix search <query>`** — Search nixpkgs
- ✅ **`i-nix list`** — Show all declaratively managed packages
- ✅ **`i-nix diff`** — Show pending changes
- ✅ **`i-nix apply`** — Evaluate and rebuild (with confirmation)
- ✅ **`i-nix rollback`** — Rollback to previous generation
- ✅ **`i-nix adopt`** — Discover current system and generate config (nixify your machine)
- ✅ **`i-nix doctor`** — Diagnose Nix/NixOS installation
- ✅ **`i-nix why <pkg>`** — Show why a package is installed
- ✅ **`i-nix edit <target>`** — Open generated config in `$EDITOR`
- ✅ **`i-nix update`** — Update flake inputs and show available upgrades

---

## Architecture

```text
┌────────────────────────────────────────────┐
│                  i-nix CLI                  │
│                                            │
│ install / remove / search / enable / etc. │
└──────────────────────┬─────────────────────┘
                       │
                       ▼
┌────────────────────────────────────────────┐
│             Intent Engine                    │
│                                            │
│ "install firefox"                          │
│          ↓                                  │
│ package: firefox                             │
│ target: home/system                          │
│ provider: nixpkgs                            │
└──────────────────────┬─────────────────────┘
                       │
                       ▼
┌────────────────────────────────────────────┐
│          Configuration Engine              │
│                                            │
│ Structured Nix config manipulation           │
│ (not regex!)                                 │
│                                              │
│ flake.nix                                    │
│ systems/default.nix                          │
│ home/default.nix                             │
│ modules/*.nix                                │
└──────────────────────┬─────────────────────┘
                       │
                       ▼
┌────────────────────────────────────────────┐
│                Nix Engine                  │
│                                            │
│ nix eval                                   │
│ nix build                                  │
│ nixos-rebuild                              │
│ home-manager                               │
└────────────────────────────────────────────┘
```

---

## Directory Structure

```text
$HOME/.config/i-nix/
├── flake/                    # The Nix flake (source of truth)
│   ├── flake.nix             # flake-parts entrypoint
│   ├── hosts/                # Per-host configurations
│   │   └── <hostname>.nix
│   ├── systems/              # System-level NixOS modules
│   │   └── default.nix       # environment.systemPackages, services, etc.
│   ├── home/                 # Home Manager configurations
│   │   └── default.nix       # home.packages, user programs, dotfiles
│   ├── modules/              # Reusable NixOS modules
│   │   └── system/
│   │   └── home/
│   ├── overlays/             # nixpkgs overlays
│   └── pkgs/                 # Custom package definitions
│   └── lib/                  # Nix helper functions
└── state.json                # i-nix metadata (NOT source of truth)
```

---

## Quick Start

```bash
# 1. Initialize i-nix
i-nix init --hostname my-machine --system

# 2. Install packages
i-nix install firefox git vim ripgrep
i-nix install --user kitty neovim starship

# 3. See what changed
i-nix diff

# 4. Apply changes
i-nix apply
```

---

## The Killer Feature: `i-nix adopt`

Got a messy NixOS machine? Turn it into a beautiful declarative configuration:

```bash
$ i-nix adopt --write

╭────────────────────────────────────╮
│ i-nix configuration discovery       │
├────────────────────────────────────┤
│                                    │
│ ✓ 147 packages discovered          │
│ ✓ GNOME detected                   │
│ ✓ PipeWire detected                │
│ ✓ Docker detected                  │
│ ✓ SSH detected                     │
│                                    │
│ Configuration generated in           │
│ ~/.config/i-nix/flake/             │
╰────────────────────────────────────╯
```

---

## Design Principles

1. **Nix remains the source of truth** — The generated `.nix` files are canonical. `i-nix` modifies them; experienced users edit them directly.
2. **One desired state. Two interfaces.** — Imperative CLI and declarative files both modify the same Nix configuration.
3. **No TOML/YAML state files** — Pure Nix. No secondary configuration language.
4. **Dotfiles where they belong** — For complex configs (editors, terminals), use plain dotfiles managed via `home.file`, not brittle HM options.
5. **Organic learning** — Start with `i-nix install firefox`, graduate to editing `configuration.nix` directly.

---

## License

MIT — See [LICENSE](LICENSE)

---

Built with 🦀 Rust, ❄️ Nix, and the dendritic flake-parts pattern.
