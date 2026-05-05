pub mod arch;
pub mod cli;
pub mod config;
pub mod pkgbuild;
pub mod sources;
pub mod telemetry;
pub mod updater;

use clap::{CommandFactory, Parser};
use clap_complete::generate;
use std::io;

use cli::{Cli, Command};

pub use updater::{ChangedPackage, RunOptions, UpdateSummary, run};

pub fn cli_main() -> updater::Result<()> {
    match Cli::parse().command {
        Command::Completions { shell } => {
            generate(shell, &mut Cli::command(), "aur-updater", &mut io::stdout());
            Ok(())
        }
        Command::Update(args) => {
            telemetry::init_tracing(env!("CARGO_PKG_NAME"), "info");
            run(RunOptions {
                config_path: args.config,
                package_filter: args.package,
                dry_run: args.dry_run,
                changed_packages_out: args.changed_packages_out,
            })?;
            Ok(())
        }
    }
}
