use anyhow::{Context, Result, anyhow};
use humantime;
use std::fs;
use std::path::PathBuf;
use toml::Value;

pub fn load(
    config_path: Option<PathBuf>,
) -> Result<(Option<PathBuf>, Vec<PathBuf>, Option<humantime::Duration>)> {
    let mut snap_dir: Option<PathBuf> = None;
    let mut toml_subvols: Vec<PathBuf> = vec![];
    let mut toml_keep: Option<humantime::Duration> = None;

    if let Some(path) = config_path {
        let config_toml = read_toml(&path)?;
        snap_dir = Some(parse_snap_dir(&config_toml, &path)?);
        toml_subvols = parse_subvols(&config_toml, &path)?;
        toml_keep = parse_keep_duration(&config_toml)?;
    }
    Ok((snap_dir, toml_subvols, toml_keep))
}

fn read_toml(path: &PathBuf) -> Result<Value> {
    let content = fs::read_to_string(path)
        .context(format!("Failed to read config file: {}", path.display()))?;
    toml::from_str(&content).context("Invalid TOML in config file")
}

fn parse_snap_dir(config: &Value, path: &PathBuf) -> Result<PathBuf> {
    let snap_str = config
        .get("snap-dir")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Missing 'snap-dir' in config file: {}", path.display()))?;
    PathBuf::from(snap_str).canonicalize().context(format!(
        "Invalid 'snap-dir' path in config file: {}",
        path.display()
    ))
}

fn parse_subvols(config: &Value, path: &PathBuf) -> Result<Vec<PathBuf>> {
    let names_arr = config.get("subvol-names").and_then(|v| v.as_array());
    if let Some(names) = names_arr {
        let subvol_names: Vec<String> = names
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        if !subvol_names.is_empty() {
            let base_str = config
                .get("subvol-base")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    anyhow!("Missing 'subvol-base' in config file: {}", path.display())
                })?;
            let subvol_base = PathBuf::from(base_str).canonicalize().context(format!(
                "Invalid 'subvol-base' path in config file: {}",
                path.display()
            ))?;
            return Ok(subvol_names
                .iter()
                .map(|name| subvol_base.join(name))
                .collect());
        }
    }
    Ok(vec![])
}

fn parse_keep_duration(config: &Value) -> Result<Option<humantime::Duration>> {
    if let Some(cleanup_table) = config.get("cleanup").and_then(|v| v.as_table()) {
        if let Some(keep_str) = cleanup_table.get("keep").and_then(|v| v.as_str()) {
            return Ok(Some(keep_str.parse::<humantime::Duration>().context(
                format!("Invalid 'keep' duration in config: {}", keep_str),
            )?));
        }
    }
    Ok(None)
}
