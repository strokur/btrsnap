use anyhow::{bail, Context, Result};
use btrfsutil::subvolume::{DeleteFlags, SnapshotFlags, Subvolume};
use chrono::{Duration, TimeZone, Utc};
use clap::{Parser, Subcommand};
use humantime::Duration as HumanDuration;
use log::{debug, info};
use nix::unistd::Uid;
use std::fs;
use std::path::PathBuf;
use toml::Value;
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "btrsnap", about = "Manage BTRFS snapshots")]
struct Cli {
    /// Path to configuration file (TOML)
    #[arg(short = 'c', long)]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a snapshot of the given subvolume(s)
    Create {
        /// Path to subvolume (repeatable; if none, use config)
        #[arg(short = 'v', long, value_parser = parse_path)]
        subvol: Vec<PathBuf>,
        /// Snapshot dir (if not set, use config)
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
        /// Snapshot dir to scan (if not set, use config)
        #[arg(short, long, value_parser = parse_path)]
        dir: Option<PathBuf>,
    },
    /// Cleanup snapshots older than duration (e.g., 7d)
    Cleanup {
        /// Snapshot dir to scan (if not set, use config)
        #[arg(short, long, value_parser = parse_path)]
        dir: Option<PathBuf>,
        /// Retention duration (e.g., 7d, 30m)
        #[arg(short, long)]
        keep: HumanDuration,
    },
}

fn main() -> Result<()> {
    env_logger::init();
    info!("Starting btrsnap");

    let cli = Cli::parse();

    if !Uid::effective().is_root() {
        bail!("Error: Must run with sudo or as root for BTRFS operations");
    }

    // Initialize paths and subvolume names
    let mut snap_dir: Option<PathBuf> = None;
    let mut toml_subvols: Vec<PathBuf> = vec![];

    // Load from config file if provided
    if let Some(config_path) = cli.config {
        if !config_path.exists() {
            bail!("Config file not found: {}", config_path.display());
        }
        let content = fs::read_to_string(&config_path).context("Invalid TOML in config file")?;
        let config_toml: Value = toml::from_str(&content).context("Invalid TOML in config file")?;

        // Load snap_dir (required)
        let snap_str = config_toml
            .get("snap_dir")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'snap_dir' in config file"))?;
        snap_dir = Some(
            PathBuf::from(snap_str)
                .canonicalize()
                .context("Invalid 'snap_dir' path in config")?,
        );

        // Load subvol_names and subvol_base if subvol_names is present
        if let Some(names_arr) = config_toml.get("subvol_names").and_then(|v| v.as_array()) {
            let subvol_names: Vec<String> = names_arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            if !subvol_names.is_empty() {
                // Require subvol_base only if subvol_names is non-empty
                let base_str = config_toml
                    .get("subvol_base")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'subvol_base' in config file (required when 'subvol_names' is provided)"))?;
                let subvol_base = PathBuf::from(base_str)
                    .canonicalize()
                    .context("Invalid 'subvol_base' path in config")?;
                toml_subvols = subvol_names
                    .iter()
                    .map(|name| subvol_base.join(name))
                    .collect();
            }
        }
    }

    match cli.command {
        Commands::Create {
            subvol: cli_subvols,
            snap_dir: cli_snap_dir,
        } => {
            // Require snap_dir (from CLI or config)
            let snap_dir = cli_snap_dir.or(snap_dir).ok_or_else(|| {
                anyhow::anyhow!(
                    "Snapshot directory must be specified via --snap-dir or config file"
                )
            })?;
            if !snap_dir.exists() {
                bail!("Snapshot directory {} does not exist", snap_dir.display());
            }

            // Require subvolumes (from CLI or config)
            let subvols_to_snap = if !cli_subvols.is_empty() {
                cli_subvols
            } else if !toml_subvols.is_empty() {
                toml_subvols
            } else {
                bail!("No subvolumes specified. Provide --subvol or 'subvol_names' in config.");
            };

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
                bail!(
                    "No snapshots specified. Use --snap <path> to specify snapshots to delete, e.g., --snap /mnt/top-level/.snapshots/@nixos-<timestamp>"
                );
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
        Commands::List { dir: cli_dir } => {
            let dir = cli_dir.or(snap_dir).ok_or_else(|| {
                anyhow::anyhow!("Snapshot directory must be specified via --dir or config file")
            })?;
            if !dir.exists() {
                bail!("Snapshot directory {} does not exist", dir.display());
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
        Commands::Cleanup { dir: cli_dir, keep } => {
            let dir = cli_dir.or(snap_dir).ok_or_else(|| {
                anyhow::anyhow!("Snapshot directory must be specified via --dir or config file")
            })?;
            if !dir.exists() {
                bail!("Snapshot directory {} does not exist", dir.display());
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
