
## [0.1.0] - 2025-10-15
- Initial release.
- Added `-v`/`--subvol` flag for specifying subvolumes.
- Implemented `create`, `list`, `delete`, `cleanup` commands.
- Used `nix` crate for root check.
- Improved root check to fail early with error for non-root users on non-help commands.
- Removed hard-coded path defaults; require `subvol-base` and `snap-dir` via config or CLI.
- Removed unused `error` import and fixed unused `subvol_base` and `subvol_names` assignments.
- Improved config file error messages for clarity.
- Changed TOML config keys to use hyphens (`subvol-base`, `snap-dir`, `subvol-names`).
- Added `.gitignore` to exclude `target` directory.
