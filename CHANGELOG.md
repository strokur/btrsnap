# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-10-15

### Added

- Initial release of btrsnap, a BTRFS snapshot manager.
- CLI commands: `create`, `delete`, `list`, `cleanup`.
- Configuration via TOML file (`--config` flag) with keys `subvol-base`,
  `snap-dir`, `subvol-names`.
- CLI flags: `-v`/`--subvol` for specifying subvolumes, `-d`/`--snap-dir` for
  snapshot directory.
- Snapshot naming convention: `{subvol}-<timestamp>` (e.g., `@home-1760561182`).
- Snapshot modification time updated via `.ignore` file touch.
- Logging (use `RUST_LOG=info` for verbose output).
- Root privilege check, fail early for non-root users on non-help commands.
- Explicit path requirements: No hard-coded defaults; require paths via config
  or CLI.

### Changed

- Refactored `match cli.command` into `Commands::execute` method for cleaner
  `main()`.
- Moved command logic to module-level functions (`create`, `delete`, `list`,
  `cleanup`) for better organization.
- Reduced nesting to two levels by extracting helper functions.
- Improved error messages (e.g., "Failed to read config file:
  /etc/btrsnap.toml").
- TOML keys use hyphens (e.g., `snap-dir`, `subvol-base`, `subvol-names`).

### Dependencies

- `anyhow = "^1.0"`
- `btrfsutil = "^0.2"`
- `clap = { version = "^4.5", features = ["derive"] }`
- `chrono = { version = "^0.4", features = ["serde"] }`
- `env_logger = "^0.11"`
- `humantime = "^2.1"`
- `log = "^0.4"`
- `nix = { version = "^0.30", features = ["user"] }`
- `toml = "^0.8"`
- `walkdir = "^2.5"`

### Removed

- Hard-coded defaults.
- Environment variables as fallbacks.

### Notes

- **Configuration Example** (`/etc/btrsnap.toml`):
  ```toml
  subvol-base = "/mnt/btrfs"
  snap-dir = "/mnt/btrfs/.snapshots"
  subvol-names = ["@nixos", "@storage", "@dotfiles"]
  ```
