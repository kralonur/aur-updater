use std::{fs, io, path::Path, path::PathBuf};

use serde::Deserialize;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to read config {path}")]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to parse config {path}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("config does not contain any [[package]] entries")]
    EmptyConfig,

    #[error("encountered package with empty name")]
    EmptyPackageName,

    #[error("[{package}] path is required")]
    MissingPackagePath { package: String },

    #[error("[{package}] repo is required for GitHub sources")]
    MissingGithubRepo { package: String },

    #[error("[{package}] git_url is required for git source")]
    MissingGitUrl { package: String },

    #[error("[{package}] base_version is required because version_template uses {{base}}")]
    MissingBaseVersion { package: String },

    #[error("[{package}] npm_package is required for npm source")]
    MissingNpmPackage { package: String },
}

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(rename = "package")]
    pub packages: Vec<PackageConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PackageConfig {
    pub name: String,
    pub path: String,
    pub source: SourceKind,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub repo: Option<String>,
    #[serde(default)]
    pub npm_package: Option<String>,
    #[serde(default)]
    pub git_url: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub version_template: Option<String>,
    #[serde(default)]
    pub base_version: Option<String>,
    #[serde(default)]
    pub strip_prefixes: Vec<String>,
    #[serde(default)]
    pub exclude_tags: Vec<String>,
    #[serde(default)]
    pub reset_pkgrel: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    GithubRelease,
    Git,
    Npm,
}

impl Config {
    pub fn from_path(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path).map_err(|source| Error::Read {
            path: path.to_owned(),
            source,
        })?;
        let config: Self = toml::from_str(&contents).map_err(|source| Error::Parse {
            path: path.to_owned(),
            source,
        })?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        if self.packages.is_empty() {
            return Err(Error::EmptyConfig);
        }

        for package in &self.packages {
            if package.name.trim().is_empty() {
                return Err(Error::EmptyPackageName);
            }
            if package.path.trim().is_empty() {
                return Err(Error::MissingPackagePath {
                    package: package.name.clone(),
                });
            }

            if package
                .version_template
                .as_deref()
                .is_some_and(|template| template.contains("{base}"))
                && package
                    .base_version
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .is_empty()
            {
                return Err(Error::MissingBaseVersion {
                    package: package.name.clone(),
                });
            }

            match package.source {
                SourceKind::GithubRelease => {
                    if package
                        .repo
                        .as_deref()
                        .unwrap_or_default()
                        .trim()
                        .is_empty()
                    {
                        return Err(Error::MissingGithubRepo {
                            package: package.name.clone(),
                        });
                    }
                }
                SourceKind::Git => {
                    if package
                        .git_url
                        .as_deref()
                        .unwrap_or_default()
                        .trim()
                        .is_empty()
                    {
                        return Err(Error::MissingGitUrl {
                            package: package.name.clone(),
                        });
                    }
                }
                SourceKind::Npm => {
                    if package
                        .npm_package
                        .as_deref()
                        .unwrap_or_default()
                        .trim()
                        .is_empty()
                    {
                        return Err(Error::MissingNpmPackage {
                            package: package.name.clone(),
                        });
                    }
                }
            }
        }

        Ok(())
    }
}

fn default_enabled() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_enabled_default_and_sources() {
        let config: Config = toml::from_str(
            r#"
            [[package]]
            name = "openai-codex-bin"
            path = "aur/openai-codex-bin"
            source = "github_release"
            repo = "openai/codex"
            strip_prefixes = ["rust-v", "v"]
            exclude_tags = ["nightly"]

            [[package]]
            name = "claude-agent-acp"
            path = "aur/claude-agent-acp"
            source = "npm"
            npm_package = "@agentclientprotocol/claude-agent-acp"
            enabled = false

            [[package]]
            name = "my-generic-git-package"
            path = "aur/my-generic-git-package"
            source = "git"
            git_url = "https://gitlab.com/example/project.git"
            branch = "master"
            version_template = "r{rev}.{sha7}"
            "#,
        )
        .unwrap();

        assert!(config.packages[0].enabled);
        assert_eq!(config.packages[0].source, SourceKind::GithubRelease);
        assert!(!config.packages[1].enabled);
        assert_eq!(config.packages[1].source, SourceKind::Npm);
        assert_eq!(config.packages[2].source, SourceKind::Git);
    }

    #[test]
    fn validates_required_source_fields() {
        let config: Config = toml::from_str(
            r#"
            [[package]]
            name = "broken"
            path = "aur/broken"
            source = "github_release"
            "#,
        )
        .unwrap();

        assert!(config.validate().is_err());
    }
}
