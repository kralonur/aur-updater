mod github;
mod npm;

use reqwest::blocking::Client;
use reqwest::header::InvalidHeaderValue;
use thiserror::Error;

use crate::config::{PackageConfig, SourceKind};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("[{package}] repo is required for github_release source")]
    MissingGithubRepo { package: String },

    #[error("[{package}] npm_package is required for npm source")]
    MissingNpmPackage { package: String },

    #[error("failed to build HTTP client")]
    BuildHttpClient {
        #[source]
        source: reqwest::Error,
    },

    #[error("[{package}] failed to query GitHub releases")]
    GithubRequest {
        package: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("[{package}] failed to parse GitHub releases response")]
    GithubResponse {
        package: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("[{package}] could not find a non-draft, non-prerelease GitHub release")]
    NoSuitableGithubRelease { package: String },

    #[error("GITHUB_TOKEN contains characters invalid for an HTTP header")]
    InvalidGithubToken {
        #[source]
        source: InvalidHeaderValue,
    },

    #[error("[{package}] failed to query npm registry")]
    NpmRequest {
        package: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("[{package}] failed to parse npm latest response")]
    NpmResponse {
        package: String,
        #[source]
        source: reqwest::Error,
    },
}

pub struct SourceClient {
    http: Client,
}

impl SourceClient {
    pub fn new() -> Result<Self> {
        let http = Client::builder()
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                "/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|source| Error::BuildHttpClient { source })?;

        Ok(Self { http })
    }

    pub fn latest_version(&self, package: &PackageConfig) -> Result<String> {
        match package.source {
            SourceKind::GithubRelease => github::latest_release_version(&self.http, package),
            SourceKind::Npm => npm::latest_version(&self.http, package),
        }
    }
}

pub fn normalize_version(raw: &str, strip_prefixes: &[String]) -> String {
    let mut version = raw.trim().to_owned();

    for prefix in strip_prefixes {
        if let Some(stripped) = version.strip_prefix(prefix) {
            version = stripped.to_owned();
        }
    }

    version.replace('-', "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_matching_prefixes_in_order_and_arch_normalizes() {
        let prefixes = vec!["rust-".to_owned(), "v".to_owned()];
        assert_eq!(
            normalize_version("rust-v1.2.3-beta.1", &prefixes),
            "1.2.3_beta.1"
        );
    }

    #[test]
    fn leaves_unmatched_prefixes_alone() {
        let prefixes = vec!["v".to_owned()];
        assert_eq!(
            normalize_version("release-1.2.3", &prefixes),
            "release_1.2.3"
        );
    }
}
