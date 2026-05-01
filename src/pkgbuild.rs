use std::{fs, io, path::Path, path::PathBuf};

use regex::Regex;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to read PKGBUILD {path}")]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to write PKGBUILD {path}")]
    Write {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("PKGBUILD does not contain pkgver=")]
    MissingPkgver,

    #[error("PKGBUILD does not contain pkgrel=")]
    MissingPkgrel,

    #[error("failed to compile internal regex")]
    Regex {
        #[source]
        source: regex::Error,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pkgbuild {
    contents: String,
    pkgver: String,
}

impl Pkgbuild {
    pub fn read(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path).map_err(|source| Error::Read {
            path: path.to_owned(),
            source,
        })?;
        Self::parse(contents)
    }

    pub fn parse(contents: String) -> Result<Self> {
        let pkgver = current_pkgver(&contents)?;
        Ok(Self { contents, pkgver })
    }

    pub fn pkgver(&self) -> &str {
        &self.pkgver
    }

    pub fn with_version(&self, version: &str, reset_pkgrel: bool) -> Result<String> {
        let pkgver_re = Regex::new(r"(?m)^pkgver=.*$").map_err(|source| Error::Regex { source })?;
        if !pkgver_re.is_match(&self.contents) {
            return Err(Error::MissingPkgver);
        }

        let pkgver_replacement = format!("pkgver={version}");
        let mut updated = pkgver_re
            .replace(&self.contents, pkgver_replacement.as_str())
            .to_string();

        if reset_pkgrel {
            let pkgrel_re =
                Regex::new(r"(?m)^pkgrel=.*$").map_err(|source| Error::Regex { source })?;
            if !pkgrel_re.is_match(&updated) {
                return Err(Error::MissingPkgrel);
            }
            updated = pkgrel_re.replace(&updated, "pkgrel=1").to_string();
        }

        Ok(updated)
    }

    pub fn write_updated(path: &Path, version: &str, reset_pkgrel: bool) -> Result<()> {
        let pkgbuild = Self::read(path)?;
        let updated = pkgbuild.with_version(version, reset_pkgrel)?;
        fs::write(path, updated).map_err(|source| Error::Write {
            path: path.to_owned(),
            source,
        })
    }
}

fn current_pkgver(contents: &str) -> Result<String> {
    let re = Regex::new(r"(?m)^pkgver=(?P<value>.+)$").map_err(|source| Error::Regex { source })?;
    let captures = re.captures(contents).ok_or(Error::MissingPkgver)?;
    Ok(captures["value"].trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_updates_pkgver_and_pkgrel() {
        let pkgbuild = Pkgbuild::parse(
            r#"
pkgname=demo
pkgver=1.0.0
pkgrel=7
"#
            .trim_start()
            .to_owned(),
        )
        .unwrap();

        assert_eq!(pkgbuild.pkgver(), "1.0.0");
        assert_eq!(
            pkgbuild.with_version("1.2.3", true).unwrap(),
            "pkgname=demo\npkgver=1.2.3\npkgrel=1\n"
        );
    }

    #[test]
    fn leaves_pkgrel_when_not_resetting() {
        let pkgbuild = Pkgbuild::parse("pkgver=1\npkgrel=4\n".to_owned()).unwrap();
        assert_eq!(
            pkgbuild.with_version("2", false).unwrap(),
            "pkgver=2\npkgrel=4\n"
        );
    }
}
