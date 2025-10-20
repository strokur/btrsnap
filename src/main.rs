use anyhow::{Context, Result, bail};
use btrfsutil::subvolume::{DeleteFlags, SnapshotFlags, Subvolume};
use chrono::{Duration, TimeZone, Utc};
use clap::{Parser, Subcommand};
use color_print::cstr;
use humantime::Duration as HumanDuration;
use log::{debug, info};
use nix::unistd::Uid;
use std::env;
use std::fs;
use std::path::PathBuf;
use toml::Value;
use walkdir::WalkDir;

const AFTER_HELP: &'static str = cstr!(
    r#"
ENVIRONMENT VARIABLES:
    <bold>BTRSNAP_CONFIG</bold>
        Path to the TOML configuration file (e.g., /etc/btrsnap.toml).
        If set, allows running commands like `btrsnap create` without --config.
"#
);

#[derive(Parser)]
#[command(
    name = "btrsnap",
    about = "Manage BTRFS snapshots",
    version = "0.2.0",
    // color = ColorChoice::Always,
    after_help = AFTER_HELP
)]
struct Cli {
    /// Path to configuration file (TOML)
    #[arg(short = 'c', long)]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Commands>,
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
        /// Retention duration (e.g., 7d, 30m; overrides config if set)
        #[arg(short, long)]
        keep: Option<HumanDuration>,
    },
}

impl Commands {
    fn execute(self, snap_dir: Option<PathBuf>, toml_subvols: Vec<PathBuf>) -> Result<()> {
        match self {
            Commands::Create {
                subvol: cli_subvols,
                snap_dir: cli_snap_dir,
            } => {
                let snap_dir = cli_snap_dir.or(snap_dir).ok_or_else(|| {
                    anyhow::anyhow!(
                        "Snapshot directory must be specified via --snap-dir or config file"
                    )
                })?;
                if !snap_dir.exists() {
                    bail!("Snapshot directory {} does not exist", snap_dir.display());
                }

                let subvols_to_snap = if !cli_subvols.is_empty() {
                    cli_subvols
                } else if !toml_subvols.is_empty() {
                    toml_subvols
                } else {
                    bail!("No subvolumes specified. Provide --subvol or 'subvol-names' in config.");
                };

                info!("Creating snapshots in {}", snap_dir.display());
                let ts = Utc::now().timestamp();
                for sv in subvols_to_snap {
                    create_snapshot(&snap_dir, &sv, ts)?;
                }
            }
            Commands::Delete { snap } => {
                if snap.is_empty() {
                    bail!(
                        "No snapshots specified. Use --snap <path> to specify snapshots to delete, e.g., --snap /mnt/top-level/.snapshots/@nixos-<timestamp>"
                    );
                }
                for s in snap {
                    delete_snapshot(&s)?;
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
                    list_snapshot(entry)?;
                }
            }
            Commands::Cleanup { dir: cli_dir, keep } => {
                let dir = cli_dir.or(snap_dir).ok_or_else(|| {
                    anyhow::anyhow!("Snapshot directory must be specified via --dir or config file")
                })?;
                if !dir.exists() {
                    bail!("Snapshot directory {} does not exist", dir.display());
                }
                let keep_duration = keep.or(toml_cleanup_keep).ok_or_else(|| {
                    anyhow::anyhow!("Retention duration must be specified via --keep or 'cleanup.keep' in config file")
                })?;
                info!(
                    "Cleaning snapshots in {} older than {}",
                    dir.display(),
                    keep_duration
                );
                let cutoff = Utc::now() - Duration::from_std(keep_duration.into())?;
                for entry in WalkDir::new(&dir)
                    .max_depth(1)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_dir() && e.path() != dir.as_path())
                {
                    cleanup_snapshot(entry, cutoff)?;
                }
            }
        }
        Ok(())
    }
}

fn create_snapshot(snap_dir: &PathBuf, sv: &PathBuf, ts: i64) -> Result<()> {
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

    let ignore_path = snap_path.join(".ignore");
    fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(ignore_path.as_path())
        .context(format!(
            "Failed to touch .ignore in snapshot {}",
            snap_path.display()
        ))?;
    Ok(())
}

fn delete_snapshot(s: &PathBuf) -> Result<()> {
    debug!("Deleting snapshot: {}", s.display());
    let subvol =
        Subvolume::get(s.as_path()).context(format!("Failed to get subvolume {}", s.display()))?;
    subvol
        .delete(DeleteFlags::empty())
        .context(format!("Failed to delete snapshot {}", s.display()))?;
    println!("Deleted: {}", s.display());
    Ok(())
}

fn list_snapshot(entry: walkdir::DirEntry) -> Result<()> {
    debug!("Checking path: {}", entry.path().display());
    let subvol = match Subvolume::get(entry.path()) {
        Ok(subvol) => subvol,
        Err(_) => {
            debug!("Path {} is not a subvolume", entry.path().display());
            return Ok(());
        }
    };
    let subvol_info = subvol.info()?;
    println!(
        "{}: gen={}, otime={}",
        entry.path().display(),
        subvol_info.generation,
        subvol_info.otransid
    );
    Ok(())
}

fn cleanup_snapshot(entry: walkdir::DirEntry, cutoff: chrono::DateTime<Utc>) -> Result<()> {
    debug!("Checking path: {}", entry.path().display());
    let ts_str = match entry.path().file_name().and_then(|n| n.to_str()) {
        Some(ts_str) => ts_str,
        None => return Ok(()),
    };
    let ts = match parse_timestamp_from_name(ts_str) {
        Some(ts) => ts,
        None => return Ok(()),
    };
    if Utc.timestamp_opt(ts, 0).single() >= Some(cutoff) {
        return Ok(());
    }
    let subvol = match Subvolume::get(entry.path()) {
        Ok(subvol) => subvol,
        Err(_) => return Ok(()),
    };
    subvol.delete(DeleteFlags::empty())?;
    println!("Cleaned: {}", entry.path().display());
    Ok(())
}

fn load_config(
    config_path: Option<PathBuf>,
) -> Result<(Option<PathBuf>, Vec<PathBuf>, Option<HumanDuration>)> {
    let mut snap_dir: Option<PathBuf> = None;
    let mut toml_subvols: Vec<PathBuf> = vec![];
    let mut toml_cleanup_keep: Option<HumanDuration> = None;

    if let Some(path) = config_path {
        if !path.exists() {
            bail!("Config file not found: {}", path.display());
        }
        let content = fs::read_to_string(&path)
            .context(format!("Failed to read config file: {}", path.display()))?;
        let config_toml: Value = toml::from_str(&content).context("Invalid TOML in config file")?;

        let snap_str = config_toml
            .get("snap-dir")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'snap-dir' in config file"))?;
        snap_dir = Some(
            PathBuf::from(snap_str)
                .canonicalize()
                .context("Invalid 'snap-dir' path in config")?,
        );

        let names_arr = config_toml.get("subvol-names").and_then(|v| v.as_array());
        if let Some(names) = names_arr {
            let subvol_names: Vec<String> = names
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            if !subvol_names.is_empty() {
                let base_str = config_toml
                    .get("subvol-base")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'subvol-base' in config file (required when 'subvol-names' is provided)"))?;
                let subvol_base = PathBuf::from(base_str)
                    .canonicalize()
                    .context("Invalid 'subvol-base' path in config")?;
                toml_subvols = subvol_names
                    .iter()
                    .map(|name| subvol_base.join(name))
                    .collect();
            }
        }

        if let Some(cleanup_table) = config_toml.get("cleanup").and_then(|v| v.as_table()) {
            if let Some(keep_str) = cleanup_table.get("keep").and_then(|v| v.as_str()) {
                toml_cleanup_keep = Some(keep_str.parse::<HumanDuration>().context(format!(
                    "Invalid 'cleanup.keep' duration in config: {}",
                    keep_str
                ))?);
            }
        }
    }
    Ok((snap_dir, toml_subvols, toml_cleanup_keep))
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

fn main() -> Result<()> {
    env_logger::init();
    info!("Starting btrsnap");

    let cli = Cli::parse();

    if !Uid::effective().is_root() {
        bail!("Error: Must run with sudo or as root for BTRFS operations");
    }

    let config_path = cli.config.or_else(|| {
        env::var("BTRSNAP_CONFIG")
            .ok()
            .and_then(|s| PathBuf::from(s).canonicalize().ok())
    });

    let (snap_dir, toml_subvols, toml_cleanup_keep) = load_config(config_path)?;

    let command = cli.command.unwrap_or(Commands::Create {
        subvol: vec![],
        snap_dir: None,
    });

    command.execute(snap_dir, toml_subvols, toml_cleanup_keep)
}
