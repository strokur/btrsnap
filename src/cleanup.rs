use crate::utils;
use anyhow::{Context, Result, anyhow};
use btrfsutil::subvolume::{DeleteFlags, Subvolume};
use chrono::{DateTime, Duration, Local};
use humantime::Duration as HumanDuration;
use log::{debug, info};
use std::fs;
use std::path::PathBuf;
use walkdir::DirEntry;

#[derive(clap::Parser)]
pub struct Cleanup {
    /// Snapshot dir to scan
    #[arg(short = 'd', long, value_parser = utils::parse_path)]
    pub snap_dir: Option<PathBuf>,
    /// Retention duration (e.g., 7d, 30m)
    #[arg(short, long)]
    pub keep: Option<HumanDuration>,
}

impl Cleanup {
    pub fn execute(
        self,
        snap_dir: Option<PathBuf>,
        keep_duration: Option<HumanDuration>,
    ) -> Result<()> {
        let snap_dir = utils::resolve_snap_dir(self.snap_dir, snap_dir)?;
        let keep = self
            .keep
            .or(keep_duration)
            .ok_or_else(|| anyhow!("Retention duration not specified"))?;

        info!(
            "Cleaning snapshots in {} older than {}",
            snap_dir.display(),
            keep
        );
        let cutoff = Local::now() - Duration::from_std(keep.into())?;
        utils::scan_snapshots(&snap_dir, |entry| cleanup_snapshot(entry, cutoff))?;
        Ok(())
    }
}

fn cleanup_snapshot(entry: DirEntry, cutoff: DateTime<Local>) -> Result<()> {
    debug!("Checking path: {}", entry.path().display());

    // Get the modification time from file system metadata
    let metadata = fs::metadata(entry.path()).context(format!(
        "Failed to read metadata for {}",
        entry.path().display()
    ))?;
    let mtime = metadata.modified().context(format!(
        "Failed to get modification time for {}",
        entry.path().display()
    ))?;

    // Convert SystemTime to DateTime<Local>
    let mtime_local: DateTime<Local> = DateTime::from(mtime);

    // Check if snapshot is newer than or equal to cutoff
    if mtime_local >= cutoff {
        debug!(
            "Snapshot {} is newer than cutoff, keeping",
            entry.path().display()
        );
        return Ok(());
    }

    // Verify it's a BTRFS subvolume
    let subvol = match Subvolume::get(entry.path()) {
        Ok(subvol) => subvol,
        Err(_) => {
            debug!(
                "Path {} is not a BTRFS subvolume, skipping",
                entry.path().display()
            );
            return Ok(());
        }
    };

    // Delete the snapshot
    subvol.delete(DeleteFlags::empty())?;
    println!("Cleaned: {}", entry.path().display());
    Ok(())
}
