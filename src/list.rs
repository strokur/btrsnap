use crate::utils;
use anyhow::Result;
use btrfsutil::subvolume::Subvolume;
use log::{debug, info};
use std::path::PathBuf;

#[derive(clap::Parser)]
pub struct List {
    /// Snapshot dir to scan
    #[arg(short = 'd', long, value_parser = utils::parse_path)]
    pub snap_dir: Option<PathBuf>,
}

impl List {
    pub fn execute(self, snap_dir: Option<PathBuf>) -> Result<()> {
        let snap_dir = utils::resolve_snap_dir(self.snap_dir, snap_dir)?;
        info!("Listing snapshots in {}", snap_dir.display());
        utils::scan_snapshots(&snap_dir, list_snapshot)?;
        Ok(())
    }
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
