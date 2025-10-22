use crate::utils;
use anyhow::{Result, anyhow};
use btrfsutil::subvolume::{DeleteFlags, Subvolume};
use chrono::{DateTime, Duration, TimeZone, Utc};
use humantime::Duration as HumanDuration;
use log::{debug, info};
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
        let cutoff = Utc::now() - Duration::from_std(keep.into())?;
        utils::scan_snapshots(&snap_dir, |entry| cleanup_snapshot(entry, cutoff))?;
        Ok(())
    }
}

fn cleanup_snapshot(entry: DirEntry, cutoff: DateTime<Utc>) -> Result<()> {
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

fn parse_timestamp_from_name(name: &str) -> Option<i64> {
    name.split_once('-')
        .and_then(|(_, ts_str)| ts_str.parse::<i64>().ok())
}
