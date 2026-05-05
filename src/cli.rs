use clap::{Parser, Subcommand};
use clap_complete::Shell;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Update configured AUR packages
    Update(UpdateArgs),

    /// Generate shell completion script to stdout
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
}

#[derive(Debug, Parser)]
pub struct UpdateArgs {
    /// Path to packages.toml
    #[arg(short, long, value_name = "PATH")]
    pub config: PathBuf,

    /// Update only one package
    #[arg(short, long, value_name = "NAME")]
    pub package: Option<String>,

    /// Check versions and tools without changing files
    #[arg(long)]
    pub dry_run: bool,

    /// Write changed packages, one per line, to this file
    ///
    /// Each line is tab-separated as: package name, new version.
    /// In dry-run mode, writes packages that would change.
    #[arg(long, value_name = "PATH")]
    pub changed_packages_out: Option<PathBuf>,
}
