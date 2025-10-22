use anyhow::{Context, Result, bail};
use clap::{CommandFactory, Parser, Subcommand};
use color_print::cstr;
use log::info;
use nix::unistd::Uid;
use std::env;
use std::fs;
use std::path::PathBuf;
use toml::Value;

mod cleanup;
mod create;
mod delete;
mod list;

const AFTER_HELP: &str = cstr!(
    r#"
<bold><underline>ENVIRONMENT VARIABLES:</underline></bold>
  <bold>BTRSNAP_CONFIG</bold>
      Path to the TOML configuration file (e.g., /etc/btrsnap.toml).
      If set, allows running commands like `btrsnap create` without --config.
"#
);

#[derive(Parser)]
#[command(
    about,
    version,
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
    Create(create::Create),
    /// Delete specific snapshot(s)
    Delete(delete::Delete),
    /// List snapshots with info
    List(list::List),
    /// Cleanup snapshots older than duration (e.g., 7d)
    Cleanup(cleanup::Cleanup),
}

impl Commands {
    fn execute(self, snap_dir: Option<PathBuf>, toml_subvols: Vec<PathBuf>) -> Result<()> {
        match self {
            Commands::Create(cmd) => cmd.execute(snap_dir, toml_subvols),
            Commands::Delete(cmd) => cmd.execute(),
            Commands::List(cmd) => cmd.execute(snap_dir),
            Commands::Cleanup(cmd) => cmd.execute(snap_dir),
        }
    }
}

fn load_config(config_path: Option<PathBuf>) -> Result<(Option<PathBuf>, Vec<PathBuf>)> {
    let mut snap_dir: Option<PathBuf> = None;
    let mut toml_subvols: Vec<PathBuf> = vec![];

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
    }
    Ok((snap_dir, toml_subvols))
}

fn parse_path(s: &str) -> Result<PathBuf> {
    PathBuf::from(s).canonicalize().context("Invalid path")
}

fn main() -> Result<()> {
    env_logger::init();
    info!("Starting btrsnap");

    // Parse CLI arguments, handling errors explicitly
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            // Print any parsing errors and exit
            e.print()?;
            std::process::exit(1);
        }
    };

    // If no subcommand is provided, explicitly print help and exit
    if cli.command.is_none() {
        Cli::command().print_help()?;
        return Ok(());
    }

    // Check for root privileges only if a subcommand is provided
    if !Uid::effective().is_root() {
        bail!("Error: Must run with sudo or as root for BTRFS operations");
    }

    let config_path = cli.config.or_else(|| {
        env::var("BTRSNAP_CONFIG")
            .ok()
            .and_then(|s| PathBuf::from(s).canonicalize().ok())
    });

    let (snap_dir, toml_subvols) = load_config(config_path)?;
    cli.command.unwrap().execute(snap_dir, toml_subvols)
}
