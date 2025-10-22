use anyhow::{Result, bail};
use btrfsutil::subvolume::Subvolume;
use log::{debug, info};
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(clap::Parser)]
pub struct List {
    /// Snapshot dir to scan
    #[arg(short = 'd', long, value_parser = super::parse_path)]
    pub snap_dir: Option<PathBuf>,
}

impl List {
    pub fn execute(self, snap_dir: Option<PathBuf>) -> Result<()> {
        let dir = self
            .snap_dir
            .or(snap_dir)
            .ok_or_else(|| anyhow::anyhow!("Snapshot directory not specified"))?;
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
