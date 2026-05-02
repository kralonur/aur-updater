use regex::Regex;
use std::sync::LazyLock;

use crate::config::PackageConfig;

use super::{Error, Result};

static SHA_LENGTH_TOKEN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{sha([1-9][0-9]*)\}").expect("valid sha token regex"));

pub struct VersionTemplateData<'a> {
    pub date: &'a str,
    pub revision: u64,
    pub sha: &'a str,
}

pub fn render(
    template: &str,
    data: &VersionTemplateData<'_>,
    package: &PackageConfig,
) -> Result<String> {
    let mut version = template
        .replace("{date}", data.date)
        .replace("{rev}", &data.revision.to_string())
        .replace("{sha}", data.sha);

    version = SHA_LENGTH_TOKEN
        .replace_all(&version, |captures: &regex::Captures<'_>| {
            let length = captures[1].parse().unwrap_or(data.sha.len());
            data.sha.chars().take(length).collect::<String>()
        })
        .into_owned();

    if version.contains("{base}") {
        let base = package
            .base_version
            .as_deref()
            .ok_or_else(|| Error::MissingBaseVersion {
                package: package.name.clone(),
            })?;
        version = version.replace("{base}", base);
    }

    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SourceKind;

    fn package() -> PackageConfig {
        PackageConfig {
            name: "example".to_owned(),
            path: "aur/example".to_owned(),
            source: SourceKind::Git,
            enabled: true,
            repo: None,
            npm_package: None,
            git_url: Some("https://example.com/repo.git".to_owned()),
            branch: Some("main".to_owned()),
            version_template: None,
            base_version: Some("0.13.0".to_owned()),
            strip_prefixes: Vec::new(),
            exclude_tags: Vec::new(),
            reset_pkgrel: false,
        }
    }

    #[test]
    fn renders_common_aur_vcs_templates() {
        let data = VersionTemplateData {
            date: "20260423",
            revision: 202,
            sha: "3c06723f0123456789abcdef0123456789abcdef",
        };

        assert_eq!(
            render("{base}.r{rev}.g{sha10}", &data, &package()).unwrap(),
            "0.13.0.r202.g3c06723f01"
        );
        assert_eq!(
            render("{date}.r{rev}.g{sha7}", &data, &package()).unwrap(),
            "20260423.r202.g3c06723"
        );
        assert_eq!(
            render("r{rev}.{sha7}", &data, &package()).unwrap(),
            "r202.3c06723"
        );
        assert_eq!(
            render("r{rev}.{sha40}", &data, &package()).unwrap(),
            "r202.3c06723f0123456789abcdef0123456789abcdef"
        );
    }

    #[test]
    fn renders_seen_aur_git_pkgver_shapes() {
        let cases = [
            (
                "r{rev}.{sha7}",
                VersionTemplateData {
                    date: "20260423",
                    revision: 150,
                    sha: "3a626c5123456789abcdef0123456789abcdef01",
                },
                "r150.3a626c5",
            ),
            (
                "{date}.r{rev}.g{sha7}",
                VersionTemplateData {
                    date: "20260423",
                    revision: 0,
                    sha: "5e7cef3123456789abcdef0123456789abcdef01",
                },
                "20260423.r0.g5e7cef3",
            ),
            (
                "r{rev}.{sha9}",
                VersionTemplateData {
                    date: "20260423",
                    revision: 11002,
                    sha: "cdabad3d0123456789abcdef0123456789abcdef",
                },
                "r11002.cdabad3d0",
            ),
            (
                "{base}.r{rev}.g{sha10}",
                VersionTemplateData {
                    date: "20260423",
                    revision: 202,
                    sha: "c3c06723f0123456789abcdef0123456789abcdef",
                },
                "0.13.0.r202.gc3c06723f0",
            ),
            (
                "r{rev}.{sha7}",
                VersionTemplateData {
                    date: "20260423",
                    revision: 56,
                    sha: "f976305123456789abcdef0123456789abcdef01",
                },
                "r56.f976305",
            ),
        ];

        for (template, data, expected) in cases {
            assert_eq!(render(template, &data, &package()).unwrap(), expected);
        }
    }
}
