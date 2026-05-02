pub mod arch;
pub mod cli;
pub mod config;
pub mod pkgbuild;
pub mod sources;
pub mod telemetry;
pub mod updater;

use clap::Parser;

use cli::Cli;

pub use updater::{RunOptions, run};

pub fn cli_main() -> updater::Result<()> {
    telemetry::init_tracing(env!("CARGO_PKG_NAME"), "info");

    let cli = Cli::parse();
    run(RunOptions {
        config_path: cli.config,
        package_filter: cli.package,
        dry_run: cli.dry_run,
    })
}
