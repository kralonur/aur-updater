use std::{
    fs, io,
    path::Path,
    path::PathBuf,
    process::{Command, ExitStatus, Stdio},
};

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to run makepkg in {dir}")]
    RunMakepkg {
        dir: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("makepkg --printsrcinfo failed in {dir} with status {status}")]
    MakepkgFailed { dir: PathBuf, status: ExitStatus },

    #[error("failed to write {path}")]
    WriteSrcinfo {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to check whether {command} is available")]
    CheckCommand {
        command: String,
        #[source]
        source: io::Error,
    },

    #[error("{command} is required but was not found in PATH")]
    MissingCommand { command: String },

    #[error("failed to run {command} in {dir}")]
    RunCommand {
        command: String,
        dir: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("{command} failed in {dir} with status {status}")]
    CommandFailed {
        command: String,
        dir: PathBuf,
        status: ExitStatus,
    },
}

pub fn verify_tools() -> Result<()> {
    require_command("updpkgsums")?;
    require_command("makepkg")?;
    Ok(())
}

pub fn regenerate(package_dir: &Path) -> Result<()> {
    run_in_dir("updpkgsums", &[], package_dir)?;

    let output = Command::new("makepkg")
        .arg("--printsrcinfo")
        .current_dir(package_dir)
        .stderr(Stdio::inherit())
        .output()
        .map_err(|source| Error::RunMakepkg {
            dir: package_dir.to_owned(),
            source,
        })?;

    if !output.status.success() {
        return Err(Error::MakepkgFailed {
            dir: package_dir.to_owned(),
            status: output.status,
        });
    }

    let srcinfo_path = package_dir.join(".SRCINFO");
    fs::write(&srcinfo_path, output.stdout).map_err(|source| Error::WriteSrcinfo {
        path: srcinfo_path,
        source,
    })
}

fn require_command(name: &str) -> Result<()> {
    let status = Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {name} >/dev/null 2>&1"))
        .status()
        .map_err(|source| Error::CheckCommand {
            command: name.to_owned(),
            source,
        })?;

    if !status.success() {
        return Err(Error::MissingCommand {
            command: name.to_owned(),
        });
    }

    Ok(())
}

fn run_in_dir(command: &str, args: &[&str], dir: &Path) -> Result<()> {
    let status = Command::new(command)
        .args(args)
        .current_dir(dir)
        .status()
        .map_err(|source| Error::RunCommand {
            command: command.to_owned(),
            dir: dir.to_owned(),
            source,
        })?;

    if !status.success() {
        return Err(Error::CommandFailed {
            command: command.to_owned(),
            dir: dir.to_owned(),
            status,
        });
    }

    Ok(())
}
