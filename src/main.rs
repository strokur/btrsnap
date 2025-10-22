use anyhow::{Result, bail};
use clap::{CommandFactory, Parser, Subcommand};
use color_print::cstr;
use log::info;
use nix::unistd::Uid;
use std::env;
use std::path::PathBuf;

mod cleanup;
pub mod config;
mod create;
mod delete;
mod list;
pub mod utils;

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
    fn execute(
        self,
        snap_dir: Option<PathBuf>,
        subvols: Vec<PathBuf>,
        keep_duration: Option<humantime::Duration>,
    ) -> Result<()> {
        match self {
            Commands::Create(cmd) => cmd.execute(snap_dir, subvols),
            Commands::Delete(cmd) => cmd.execute(),
            Commands::List(cmd) => cmd.execute(snap_dir),
            Commands::Cleanup(cmd) => cmd.execute(snap_dir, keep_duration),
        }
    }
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

    let (snap_dir, toml_subvols, toml_keep) = config::load(config_path)?;
    cli.command
        .unwrap()
        .execute(snap_dir, toml_subvols, toml_keep)
}
