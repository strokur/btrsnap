use anyhow::{Result, bail};
use btrfsutil::subvolume::{DeleteFlags, Subvolume};
use chrono::{Duration, TimeZone, Utc};
use humantime::Duration as HumanDuration;
use log::{debug, info};
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(clap::Parser)]
pub struct Cleanup {
    /// Snapshot directory to scan
    #[arg(short = 'd', long, value_parser = super::parse_path)]
    pub snap_dir: Option<PathBuf>,
    /// Retention duration (e.g., 7d, 30m)
    #[arg(short, long)]
    pub keep: HumanDuration,
}

impl Cleanup {
    pub fn execute(self, snap_dir: Option<PathBuf>) -> Result<()> {
        let dir = self
            .snap_dir
            .or(snap_dir)
            .ok_or_else(|| anyhow::anyhow!("Snapshot directory not specified"))?;
        if !dir.exists() {
            bail!("Snapshot directory {} does not exist", dir.display());
        }
        info!(
            "Cleaning snapshots in {} older than {}",
            dir.display(),
            self.keep
        );
        let cutoff = Utc::now() - Duration::from_std(self.keep.into())?;
        for entry in WalkDir::new(&dir)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_dir() && e.path() != dir.as_path())
        {
            cleanup_snapshot(entry, cutoff)?;
        }
        Ok(())
    }
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

fn parse_timestamp_from_name(name: &str) -> Option<i64> {
    name.rsplit_once('-')
        .and_then(|(_name, ts_str)| ts_str.parse::<i64>().ok())
}
