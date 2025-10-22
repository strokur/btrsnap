use anyhow::{Context, Result, anyhow, bail};
use std::path::PathBuf;
use walkdir::{DirEntry, WalkDir};

pub fn resolve_snap_dir(
    cli_snap_dir: Option<PathBuf>,
    config_snap_dir: Option<PathBuf>,
) -> Result<PathBuf, anyhow::Error> {
    let snap_dir = cli_snap_dir
        .or(config_snap_dir)
        .ok_or_else(|| anyhow!("Snapshot directory not specified"))?;
    if !snap_dir.exists() {
        bail!("Snapshot directory {} does not exist", snap_dir.display());
    }
    snap_dir
        .canonicalize()
        .context("Failed to canonicalize snapshot directory")
}

pub fn parse_path(s: &str) -> Result<PathBuf, anyhow::Error> {
    PathBuf::from(s).canonicalize().context("Invalid path")
}

pub fn scan_snapshots<F>(snap_dir: &PathBuf, mut callback: F) -> Result<(), anyhow::Error>
where
    F: FnMut(DirEntry) -> Result<(), anyhow::Error>,
{
    for entry in WalkDir::new(snap_dir)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir() && e.path() != snap_dir.as_path())
    {
        callback(entry)?;
    }
    Ok(())
}
