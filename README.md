# btrsnap

**btrsnap** is a command-line tool for managing BTRFS snapshots on Linux
systems, designed for simplicity and integration with systemd services. It
supports creating, deleting, listing, and cleaning up snapshots of specified
subvolumes, with configuration via TOML files or CLI arguments.

## Features

- **Create Snapshots**: Snapshot one or more BTRFS subvolumes with timestamped
  names (e.g., `@home-1760561182`).
- **Delete Snapshots**: Remove specific snapshots by path.
- **List Snapshots**: Display snapshot details (path, generation, otime).
- **Cleanup Snapshots**: Remove snapshots older than a specified duration (e.g.,
  `7d`), configurable via TOML or CLI.
- **TOML Configuration**: Define subvolumes, snapshot directories, and cleanup
  retention in a TOML file.
- **Environment Variable**: Use `BTRSNAP_CONFIG` to specify the TOML file path.
- **CLI Flexibility**: Override config with flags like `-v`/`--subvol`,
  `-d`/`--snap-dir`, and `-k`/`--keep`.
- **Systemd Integration**: Run as a systemd service for automated snapshot
  management.
- **Root Check**: Ensures commands run with `sudo` for BTRFS operations.

## Installation

### Prerequisites

- **Rust**: Version 1.82.0 or later (uses Rust 2024 edition).
- **BTRFS Filesystem**: A BTRFS filesystem with subvolumes (e.g.,
  `/mnt/btrfs/@nixos`).
- **Linux System**: Tested on NixOS; should work on other Linux distributions.
- **Dependencies**: Managed via `Cargo.toml` (e.g., `clap`, `btrfsutil`,
  `toml`).

### Build from Source

1. Clone the repository:
   ````bash
   git clone https://gitlab.com/0FGk3Zb2sY/btrsnap.git
   cd btrsnap  ```
   ````
