use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::config::PackageConfig;

use super::{
    Error, Result, normalize_version,
    version_template::{self, VersionTemplateData},
};

pub fn latest_version(package: &PackageConfig) -> Result<String> {
    let git_url = package
        .git_url
        .as_deref()
        .ok_or_else(|| Error::MissingGitUrl {
            package: package.name.clone(),
        })?;
    let temp_dir = TempGitDir::create(package)?;

    clone_repo(package, git_url, package.branch.as_deref(), temp_dir.path())?;

    let revision_output = git_stdout(package, temp_dir.path(), ["rev-list", "--count", "HEAD"])?;
    let revision = revision_output
        .parse()
        .map_err(|_| Error::InvalidGitRevision {
            package: package.name.clone(),
            revision: revision_output,
        })?;
    let sha = git_stdout(package, temp_dir.path(), ["rev-parse", "HEAD"])?;
    let date = git_stdout(
        package,
        temp_dir.path(),
        ["log", "-1", "--format=%cd", "--date=format:%Y%m%d", "HEAD"],
    )?;
    if date.len() != 8 || !date.chars().all(|char| char.is_ascii_digit()) {
        return Err(Error::InvalidGitCommitDate {
            package: package.name.clone(),
            date,
        });
    }

    let template = package
        .version_template
        .as_deref()
        .unwrap_or("r{rev}.{sha7}");
    let version = version_template::render(
        template,
        &VersionTemplateData {
            date: &date,
            revision,
            sha: &sha,
        },
        package,
    )?;

    Ok(normalize_version(&version, &package.strip_prefixes))
}

fn clone_repo(
    package: &PackageConfig,
    git_url: &str,
    branch: Option<&str>,
    path: &Path,
) -> Result<()> {
    let path = path.display().to_string();
    let mut args = vec!["clone", "--filter=blob:none"];
    if let Some(branch) = branch {
        args.extend(["--single-branch", "--branch", branch]);
    }
    args.extend([git_url, &path]);

    git_output(package, Path::new("."), args).map(|_| ())
}

fn git_stdout<const N: usize>(
    package: &PackageConfig,
    current_dir: &Path,
    args: [&str; N],
) -> Result<String> {
    let output = git_output(package, current_dir, args)?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn git_output<I, S>(package: &PackageConfig, current_dir: &Path, args: I) -> Result<Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let args: Vec<String> = args
        .into_iter()
        .map(|arg| arg.as_ref().to_owned())
        .collect();
    let command = format!("git {}", args.join(" "));
    let output = Command::new("git")
        .args(&args)
        .current_dir(current_dir)
        .output()
        .map_err(|source| Error::GitCommand {
            package: package.name.clone(),
            command: command.clone(),
            source,
        })?;

    if output.status.success() {
        return Ok(output);
    }

    Err(Error::GitCommandFailed {
        package: package.name.clone(),
        command,
        status: output.status,
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
    })
}

struct TempGitDir {
    path: PathBuf,
}

impl TempGitDir {
    fn create(package: &PackageConfig) -> Result<Self> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let mut path = env::temp_dir();
        path.push(format!(
            "{}-{}-{timestamp}",
            env!("CARGO_PKG_NAME"),
            package.name.replace('/', "_")
        ));

        fs::create_dir(&path).map_err(|source| Error::CreateGitTempDir {
            package: package.name.clone(),
            path: path.clone(),
            source,
        })?;

        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempGitDir {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.path)
            && error.kind() != io::ErrorKind::NotFound
        {
            tracing::warn!(
                path = %self.path.display(),
                "failed to remove temporary git directory: {error}"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SourceKind;

    #[test]
    fn renders_default_git_version_template() {
        let package = PackageConfig {
            name: "example-git".to_owned(),
            path: "aur/example-git".to_owned(),
            source: SourceKind::Git,
            enabled: true,
            repo: None,
            npm_package: None,
            git_url: Some("https://example.com/repo.git".to_owned()),
            branch: Some("master".to_owned()),
            version_template: None,
            base_version: None,
            strip_prefixes: Vec::new(),
            exclude_tags: Vec::new(),
            reset_pkgrel: false,
        };
        let version = version_template::render(
            "r{rev}.{sha7}",
            &VersionTemplateData {
                date: "20260502",
                revision: 56,
                sha: "f976305123456789",
            },
            &package,
        )
        .unwrap();

        assert_eq!(version, "r56.f976305");
    }

    #[test]
    fn computes_version_from_local_git_repo() {
        let package = PackageConfig {
            name: "local-git".to_owned(),
            path: "aur/local-git".to_owned(),
            source: SourceKind::Git,
            enabled: true,
            repo: None,
            npm_package: None,
            git_url: None,
            branch: None,
            version_template: Some("r{rev}.{sha7}".to_owned()),
            base_version: None,
            strip_prefixes: Vec::new(),
            exclude_tags: Vec::new(),
            reset_pkgrel: false,
        };
        let source_dir = TempGitDir::create(&package).unwrap();

        git_output(&package, source_dir.path(), ["init"]).unwrap();
        fs::write(source_dir.path().join("README.md"), "test\n").unwrap();
        git_output(&package, source_dir.path(), ["add", "README.md"]).unwrap();
        git_output(
            &package,
            source_dir.path(),
            [
                "-c",
                "user.name=AUR Updater Test",
                "-c",
                "user.email=aur-updater@example.invalid",
                "commit",
                "-m",
                "initial commit",
            ],
        )
        .unwrap();

        let package = PackageConfig {
            git_url: Some(source_dir.path().display().to_string()),
            ..package
        };
        let version = latest_version(&package).unwrap();

        assert!(version.starts_with("r1."));
        assert_eq!(version.len(), "r1.".len() + 7);
    }
}
