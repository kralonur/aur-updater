use std::{
    fs, io,
    path::{Path, PathBuf},
};

use thiserror::Error;
use tracing::{info, warn};

use crate::{
    arch,
    config::{Config, PackageConfig},
    pkgbuild::Pkgbuild,
    sources::SourceClient,
};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Config(#[from] crate::config::Error),

    #[error(transparent)]
    Sources(#[from] crate::sources::Error),

    #[error(transparent)]
    Pkgbuild(#[from] crate::pkgbuild::Error),

    #[error(transparent)]
    Arch(#[from] crate::arch::Error),

    #[error("no enabled package named '{package}' found in config")]
    PackageNotFound { package: String },

    #[error("no enabled packages found in config")]
    NoEnabledPackages,

    #[error("package directory does not exist: {path}")]
    MissingPackageDir { path: PathBuf },

    #[error("PKGBUILD does not exist: {path}")]
    MissingPkgbuildFile { path: PathBuf },

    #[error("failed to write changed packages to {path}")]
    WriteChangedPackages {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("[{package}] update failed")]
    PackageUpdate {
        package: String,
        #[source]
        source: Box<Error>,
    },
}

impl Error {
    fn package_update(package: impl Into<String>, source: Error) -> Self {
        Self::PackageUpdate {
            package: package.into(),
            source: Box::new(source),
        }
    }
}

#[derive(Debug)]
pub struct RunOptions {
    pub config_path: PathBuf,
    pub package_filter: Option<String>,
    pub dry_run: bool,
    pub changed_packages_out: Option<PathBuf>,
}

#[derive(Debug)]
struct PackageOutcome {
    name: String,
    old_version: String,
    new_version: String,
    changed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateSummary {
    pub changed_packages: Vec<ChangedPackage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedPackage {
    pub name: String,
    pub old_version: String,
    pub new_version: String,
}

pub fn run(options: RunOptions) -> Result<UpdateSummary> {
    let config = Config::from_path(&options.config_path)?;

    if options.dry_run {
        arch::verify_tools()?;
    }

    let packages = selected_packages(&config, options.package_filter.as_deref())?;
    let source_client = SourceClient::new()?;
    let mut outcomes = Vec::new();

    for package in packages {
        let outcome = update_package(package, &source_client, options.dry_run)
            .map_err(|source| Error::package_update(package.name.clone(), source))?;
        outcomes.push(outcome);
    }

    print_summary(&outcomes, options.dry_run);
    let summary = UpdateSummary::from_outcomes(&outcomes);

    if let Some(path) = &options.changed_packages_out {
        summary.write_changed_packages(path)?;
    }

    Ok(summary)
}

fn selected_packages<'a>(
    config: &'a Config,
    package_filter: Option<&str>,
) -> Result<Vec<&'a PackageConfig>> {
    let packages: Vec<_> = config
        .packages
        .iter()
        .filter(|package| package.enabled)
        .filter(|package| {
            package_filter
                .map(|wanted| package.name == wanted)
                .unwrap_or(true)
        })
        .collect();

    if packages.is_empty() {
        if let Some(wanted) = package_filter {
            return Err(Error::PackageNotFound {
                package: wanted.to_owned(),
            });
        }
        return Err(Error::NoEnabledPackages);
    }

    Ok(packages)
}

fn update_package(
    package: &PackageConfig,
    source_client: &SourceClient,
    dry_run: bool,
) -> Result<PackageOutcome> {
    let package_dir = Path::new(&package.path);
    if !package_dir.is_dir() {
        return Err(Error::MissingPackageDir {
            path: package_dir.to_owned(),
        });
    }

    let pkgbuild_path = package_dir.join("PKGBUILD");
    if !pkgbuild_path.is_file() {
        return Err(Error::MissingPkgbuildFile {
            path: pkgbuild_path,
        });
    }

    let pkgbuild = Pkgbuild::read(&pkgbuild_path)?;
    let old_version = pkgbuild.pkgver().to_owned();
    let new_version = source_client.latest_version(package)?;

    if old_version == new_version {
        info!("[{}] already up to date ({old_version})", package.name);
        return Ok(PackageOutcome {
            name: package.name.clone(),
            old_version,
            new_version,
            changed: false,
        });
    }

    if dry_run {
        info!(
            "[{}] would update PKGBUILD from {} to {}",
            package.name, old_version, new_version
        );
        if package.reset_pkgrel {
            info!("[{}] would reset pkgrel to 1", package.name);
        }
    } else {
        info!(
            "[{}] updating PKGBUILD from {} to {}",
            package.name, old_version, new_version
        );
        Pkgbuild::write_updated(&pkgbuild_path, &new_version, package.reset_pkgrel)?;
        arch::regenerate(package_dir)?;
    }

    Ok(PackageOutcome {
        name: package.name.clone(),
        old_version,
        new_version,
        changed: true,
    })
}

fn print_summary(outcomes: &[PackageOutcome], dry_run: bool) {
    let changed: Vec<_> = outcomes.iter().filter(|outcome| outcome.changed).collect();

    if changed.is_empty() {
        info!("No updates were necessary.");
        return;
    }

    if dry_run {
        warn!("Dry run: no files were changed.");
        info!("Planned package updates:");
    } else {
        info!("Updated packages:");
    }

    for outcome in changed {
        info!(
            "  - {}: {} -> {}",
            outcome.name, outcome.old_version, outcome.new_version
        );
    }
}

impl UpdateSummary {
    fn from_outcomes(outcomes: &[PackageOutcome]) -> Self {
        let changed_packages = outcomes
            .iter()
            .filter(|outcome| outcome.changed)
            .map(|outcome| ChangedPackage {
                name: outcome.name.clone(),
                old_version: outcome.old_version.clone(),
                new_version: outcome.new_version.clone(),
            })
            .collect();

        Self { changed_packages }
    }

    fn write_changed_packages(&self, path: &Path) -> Result<()> {
        let mut contents = self
            .changed_packages
            .iter()
            .map(|package| format!("{}\t{}", package.name, package.new_version))
            .collect::<Vec<_>>()
            .join("\n");
        if !contents.is_empty() {
            contents.push('\n');
        }

        fs::write(path, contents).map_err(|source| Error::WriteChangedPackages {
            path: path.to_owned(),
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use super::*;

    #[test]
    fn writes_changed_packages_with_new_versions() {
        let path = env::temp_dir().join(format!(
            "aur-updater-changed-packages-{}",
            std::process::id()
        ));
        let summary = UpdateSummary {
            changed_packages: vec![
                ChangedPackage {
                    name: "first".to_owned(),
                    old_version: "1.0.0".to_owned(),
                    new_version: "1.1.0".to_owned(),
                },
                ChangedPackage {
                    name: "second".to_owned(),
                    old_version: "2.0.0".to_owned(),
                    new_version: "2.1.0".to_owned(),
                },
            ],
        };

        summary.write_changed_packages(&path).unwrap();

        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "first\t1.1.0\nsecond\t2.1.0\n"
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn writes_empty_changed_package_file_when_nothing_changed() {
        let path = env::temp_dir().join(format!(
            "aur-updater-no-changed-packages-{}",
            std::process::id()
        ));
        let summary = UpdateSummary {
            changed_packages: Vec::new(),
        };

        summary.write_changed_packages(&path).unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "");
        fs::remove_file(path).unwrap();
    }
}
