# Changelog

## [0.2.0] - 2025-10-19

### Added

- Support for `cleanup.keep` in TOML configuration to specify retention duration
  for the `cleanup` command.
- Support for `BTRSNAP_CONFIG` environment variable to specify the TOML config
  file path, allowing commands without `--config`.

### Changed

- Bumped version to `0.2.0`.
- Updated `README.md` to document `cleanup.keep` option and BTRSNAP_CONFIG.

## [0.1.0] - 2025-10-01

### Added

- Initial release with `create`, `delete`, `list`, and `cleanup` commands.
- TOML configuration support for `subvol-base`, `snap-dir`, and `subvol-names`.
- CLI options for subvolume paths, snapshot directory, and retention duration.

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
