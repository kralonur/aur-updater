use reqwest::blocking::Client;
use serde::Deserialize;

use crate::config::PackageConfig;

use super::{Error, Result, normalize_version};

#[derive(Debug, Deserialize)]
struct NpmLatest {
    version: String,
}

pub fn latest_version(http: &Client, package: &PackageConfig) -> Result<String> {
    let npm_package = package
        .npm_package
        .as_deref()
        .ok_or_else(|| Error::MissingNpmPackage {
            package: package.name.clone(),
        })?;
    let url = format!("https://registry.npmjs.org/{npm_package}/latest");

    let latest: NpmLatest = http
        .get(url)
        .send()
        .map_err(|source| Error::NpmRequest {
            package: package.name.clone(),
            source,
        })?
        .error_for_status()
        .map_err(|source| Error::NpmRequest {
            package: package.name.clone(),
            source,
        })?
        .json()
        .map_err(|source| Error::NpmResponse {
            package: package.name.clone(),
            source,
        })?;

    Ok(normalize_version(&latest.version, &package.strip_prefixes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_npm_latest_fixture() {
        let latest: NpmLatest = serde_json::from_str(r#"{"version": "1.2.3-beta.1"}"#).unwrap();
        assert_eq!(normalize_version(&latest.version, &[]), "1.2.3_beta.1");
    }
}
