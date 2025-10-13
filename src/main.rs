use anyhow::{Context, Result};
use btrfsutil::subvolume::{DeleteFlags, SnapshotFlags, Subvolume};
use chrono::{Duration, TimeZone, Utc};
use clap::{Parser, Subcommand};
use humantime::Duration as HumanDuration;
use log::{debug, error, info};
use std::env;
use std::fs;
use std::path::PathBuf;
use toml::Value;
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "btrsnap", about = "Manage BTRFS snapshots")]
struct Cli {
    /// Path to configuration file (TOML)
    #[arg(long)]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a snapshot of the given subvolume(s)
    Create {
        /// Path to subvolume (repeatable; if none, use config or default)
        #[arg(short = 's', long, value_parser = parse_path)]
        subvol: Vec<PathBuf>,
        /// Snapshot dir (if not set, use config/env/default)
        #[arg(short = 'd', long, value_parser = parse_path)]
        snap_dir: Option<PathBuf>,
    },
    /// Delete specific snapshot(s)
    Delete {
        /// Path to snapshot (repeatable, e.g., --snap /mnt/top-level/.snapshots/@nixos-<timestamp>)
        #[arg(short, long, value_parser = parse_path)]
        snap: Vec<PathBuf>,
    },
    /// List snapshots with info
    List {
        /// Snapshot dir to scan (if not set, use config/env/default)
        #[arg(short, long, value_parser = parse_path)]
        dir: Option<PathBuf>,
    },
    /// Cleanup snapshots older than duration (e.g., 7d)
    Cleanup {
        /// Snapshot dir to scan (if not set, use config/env/default)
        #[arg(short, long, value_parser = parse_path)]
        dir: Option<PathBuf>,
        /// Retention duration (e.g., 7d, 30m)
        #[arg(short, long)]
        keep: HumanDuration,
    },
}

fn default_subvol_base() -> PathBuf {
    env::var("BTRFS")
        .ok()
        .and_then(|s| PathBuf::from(s).canonicalize().ok())
        .unwrap_or_else(|| PathBuf::from("/mnt/top-level"))
}

fn default_snap_dir() -> PathBuf {
    env::var("SNAPSHOTS")
        .ok()
        .and_then(|s| PathBuf::from(s).canonicalize().ok())
        .unwrap_or_else(|| default_subvol_base().join(".snapshots"))
}

fn main() -> Result<()> {
    env_logger::init();
    info!("Starting btrsnap");

    let cli = Cli::parse();
    if env::var("SUDO_UID").is_err() {
        eprintln!("Warning: Run with sudo for BTRFS ops.");
    }

    // Load defaults from env/hardcoded
    let mut subvol_base = default_subvol_base();
    let mut snap_dir_default = default_snap_dir();
    let mut subvol_names: Vec<String> = vec![];
    let mut default_subvols: Vec<PathBuf> = vec![];

    // If --config provided, override with file values
    if let Some(config_path) = cli.config {
        if !config_path.exists() {
            error!("Config file not found: {}", config_path.display());
            anyhow::bail!("Config file not found: {}", config_path.display());
        }
        let content = fs::read_to_string(&config_path).context("Invalid TOML in config file")?;
        let config_toml: Value = toml::from_str(&content).context("Invalid TOML in config file")?;

        if let Some(base_str) = config_toml.get("subvol_base").and_then(|v| v.as_str()) {
            if let Ok(base_path) = PathBuf::from(base_str).canonicalize() {
                subvol_base = base_path;
            }
        }

        if let Some(snap_str) = config_toml.get("snap_dir").and_then(|v| v.as_str()) {
            if let Ok(snap_path) = PathBuf::from(snap_str).canonicalize() {
                snap_dir_default = snap_path;
            }
        }

        if let Some(names_arr) = config_toml.get("subvol_names").and_then(|v| v.as_array()) {
            subvol_names = names_arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }

        // Compute default subvols from base + names
        default_subvols = subvol_names
            .iter()
            .map(|name| subvol_base.join(name))
            .collect();
    }

    match cli.command {
        Commands::Create { subvol, snap_dir } => {
            let subvols_to_snap = if subvol.is_empty() {
                if default_subvols.is_empty() {
                    error!("No subvolumes specified, and no 'subvol_names' in config");
                    anyhow::bail!("No subvolumes specified, and no 'subvol_names' in config. Provide --subvol or configure.");
                }
                default_subvols
            } else {
                subvol
            };
            let snap_dir = snap_dir.unwrap_or(snap_dir_default);
            if !snap_dir.exists() {
                error!("Snapshot directory {} does not exist", snap_dir.display());
                anyhow::bail!("Snapshot directory {} does not exist", snap_dir.display());
            }
            info!("Creating snapshots in {}", snap_dir.display());

            let ts = Utc::now().timestamp();
            for sv in &subvols_to_snap {
                let subvol_name = sv.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
                debug!("Processing subvolume: {}", sv.display());
                let snap_name = format!("{}-{}", subvol_name, ts);
                let snap_path = snap_dir.join(&snap_name);
                let subvol = Subvolume::get(sv.as_path())
                    .context(format!("Failed to get subvolume {}", sv.display()))?;
                subvol
                    .snapshot(snap_path.as_path(), SnapshotFlags::empty(), None)
                    .context(format!(
                        "Failed to create snapshot {} for subvolume {}",
                        snap_path.display(),
                        sv.display()
                    ))?;
                println!("Created snapshot: {}", snap_path.display());

                // Touch .ignore file to update modification time
                let ignore_path = snap_path.join(".ignore");
                fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .open(ignore_path.as_path())
                    .context(format!(
                        "Failed to touch .ignore in snapshot {}",
                        snap_path.display()
                    ))?;
            }
        }
        Commands::Delete { snap } => {
            if snap.is_empty() {
                error!("No snapshots specified for deletion");
                anyhow::bail!("No snapshots specified. Use --snap <path> to specify snapshots to delete, e.g., --snap /mnt/top-level/.snapshots/@nixos-<timestamp>");
            }
            for s in snap {
                debug!("Deleting snapshot: {}", s.display());
                let subvol = Subvolume::get(s.as_path())
                    .context(format!("Failed to get subvolume {}", s.display()))?;
                subvol
                    .delete(DeleteFlags::empty())
                    .context(format!("Failed to delete snapshot {}", s.display()))?;
                println!("Deleted: {}", s.display());
            }
        }
        Commands::List { dir } => {
            let dir = dir.unwrap_or(snap_dir_default);
            if !dir.exists() {
                error!("Snapshot directory {} does not exist", dir.display());
                anyhow::bail!("Snapshot directory {} does not exist", dir.display());
            }
            info!("Listing snapshots in {}", dir.display());
            for entry in WalkDir::new(&dir)
                .max_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_dir() && e.path() != dir.as_path())
            {
                debug!("Checking path: {}", entry.path().display());
                if let Ok(subvol) = Subvolume::get(entry.path()) {
                    let subvol_info = subvol.info()?;
                    println!(
                        "{}: gen={}, otime={}",
                        entry.path().display(),
                        subvol_info.generation,
                        subvol_info.otransid
                    );
                } else {
                    debug!("Path {} is not a subvolume", entry.path().display());
                }
            }
        }
        Commands::Cleanup { dir, keep } => {
            let dir = dir.unwrap_or(snap_dir_default);
            if !dir.exists() {
                error!("Snapshot directory {} does not exist", dir.display());
                anyhow::bail!("Snapshot directory {} does not exist", dir.display());
            }
            info!(
                "Cleaning snapshots in {} older than {}",
                dir.display(),
                keep
            );
            let cutoff = Utc::now() - Duration::from_std(keep.into())?;
            for entry in WalkDir::new(&dir)
                .max_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_dir() && e.path() != dir.as_path())
            {
                debug!("Checking path: {}", entry.path().display());
                if let Some(ts_str) = entry.path().file_name().and_then(|n| n.to_str()) {
                    if let Some(ts) = parse_timestamp_from_name(ts_str) {
                        if Utc.timestamp_opt(ts, 0).single() < Some(cutoff) {
                            if let Ok(subvol) = Subvolume::get(entry.path()) {
                                subvol.delete(DeleteFlags::empty())?;
                                println!("Cleaned: {}", entry.path().display());
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn parse_path(s: &str) -> Result<PathBuf> {
    PathBuf::from(s).canonicalize().context("Invalid path")
}

fn parse_timestamp_from_name(name: &str) -> Option<i64> {
    let prefixes = ["@nixos-", "@storage-", "@dotfiles-"];
    for prefix in prefixes {
        if let Some(ts_str) = name.strip_prefix(prefix) {
            return ts_str.parse::<i64>().ok();
        }
    }
    None
}
