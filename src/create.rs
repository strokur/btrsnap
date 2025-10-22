use crate::utils;
use anyhow::{Context, Result, bail};
use btrfsutil::subvolume::{SnapshotFlags, Subvolume};
use chrono::Utc;
use log::{debug, info};
use std::fs;
use std::path::PathBuf;

#[derive(clap::Parser)]
pub struct Create {
    /// Path to subvolume (repeatable)
    #[arg(short = 'v', long, value_parser = utils::parse_path)]
    pub subvol: Vec<PathBuf>,
    /// Snapshot directory
    #[arg(short = 'd', long, value_parser = utils::parse_path)]
    pub snap_dir: Option<PathBuf>,
}

impl Create {
    pub fn execute(self, snap_dir: Option<PathBuf>, subvols: Vec<PathBuf>) -> Result<()> {
        let snap_dir = utils::resolve_snap_dir(self.snap_dir, snap_dir)?;
        let subvols_to_snap = if !self.subvol.is_empty() {
            self.subvol
        } else if !subvols.is_empty() {
            subvols
        } else {
            bail!("Subvolumes not specified");
        };

        info!("Creating snapshots in {}", snap_dir.display());
        let ts = Utc::now().timestamp();
        for sv in subvols_to_snap {
            create_snapshot(&snap_dir, &sv, ts)?;
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
