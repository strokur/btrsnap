use crate::utils;
use anyhow::{Context, Result, bail};
use btrfsutil::subvolume::{DeleteFlags, Subvolume};
use log::debug;
use std::path::PathBuf;

#[derive(clap::Parser)]
pub struct Delete {
    /// Path to snapshot (repeatable)
    #[arg(short, long, value_parser = utils::parse_path)]
    pub snapshot: Vec<PathBuf>,
}

impl Delete {
    pub fn execute(self) -> Result<()> {
        if self.snapshot.is_empty() {
            bail!("Snapshots not specified");
        }
        for s in self.snapshot {
            delete_snapshot(&s)?;
        }
        Ok(())
    }
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
