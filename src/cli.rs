use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Path to packages.toml
    #[arg(short, long, value_name = "PATH")]
    pub config: PathBuf,

    /// Update only one package
    #[arg(short, long, value_name = "NAME")]
    pub package: Option<String>,

    /// Check versions and tools without changing files
    #[arg(long)]
    pub dry_run: bool,
}
