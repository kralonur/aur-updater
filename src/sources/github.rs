use std::env;

use reqwest::{
    blocking::Client,
    header::{AUTHORIZATION, HeaderMap, HeaderValue},
};
use serde::Deserialize;

use crate::config::PackageConfig;

use super::{Error, Result, normalize_version};

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    draft: bool,
}

pub fn latest_release_version(http: &Client, package: &PackageConfig) -> Result<String> {
    let repo = package
        .repo
        .as_deref()
        .ok_or_else(|| Error::MissingGithubRepo {
            package: package.name.clone(),
        })?;
    let url = format!("https://api.github.com/repos/{repo}/releases");

    let releases: Vec<Release> = http
        .get(url)
        .headers(auth_headers()?)
        .send()
        .map_err(|source| Error::GithubRequest {
            package: package.name.clone(),
            source,
        })?
        .error_for_status()
        .map_err(|source| Error::GithubRequest {
            package: package.name.clone(),
            source,
        })?
        .json()
        .map_err(|source| Error::GithubResponse {
            package: package.name.clone(),
            source,
        })?;

    let release = select_release(&releases, &package.exclude_tags).ok_or_else(|| {
        Error::NoSuitableGithubRelease {
            package: package.name.clone(),
        }
    })?;

    Ok(normalize_version(
        &release.tag_name,
        &package.strip_prefixes,
    ))
}

fn auth_headers() -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    if let Ok(token) = env::var("GITHUB_TOKEN")
        && !token.trim().is_empty()
    {
        let value = HeaderValue::from_str(&format!("Bearer {token}"))
            .map_err(|source| Error::InvalidGithubToken { source })?;
        headers.insert(AUTHORIZATION, value);
    }
    Ok(headers)
}

fn select_release<'a>(releases: &'a [Release], exclude_tags: &[String]) -> Option<&'a Release> {
    releases.iter().find(|release| {
        !release.draft
            && !release.prerelease
            && !exclude_tags
                .iter()
                .any(|exclude| release.tag_name.contains(exclude))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_drafts_prereleases_and_excluded_tags() {
        let releases: Vec<Release> = serde_json::from_str(
            r#"
            [
              {"tag_name": "rust-v1.3.0-nightly", "draft": false, "prerelease": false},
              {"tag_name": "rust-v1.2.0", "draft": false, "prerelease": true},
              {"tag_name": "rust-v1.1.0", "draft": true, "prerelease": false},
              {"tag_name": "rust-v1.0.0", "draft": false, "prerelease": false}
            ]
            "#,
        )
        .unwrap();

        let exclude_tags = vec!["nightly".to_owned()];
        let selected = select_release(&releases, &exclude_tags).unwrap();

        assert_eq!(selected.tag_name, "rust-v1.0.0");
    }
}
