# aur-updater

Rust CLI for updating AUR package directories in a maintenance repository.

It reads a TOML config, checks upstream versions, updates `pkgver` in each
`PKGBUILD`, optionally resets `pkgrel=1`, then regenerates checksums and
`.SRCINFO`.

## Usage

```bash
# Update every enabled package in the config.
aur-updater --config packages.toml

# Update only one package.
aur-updater --config packages.toml --package openai-codex-bin

# Check versions and tools without editing files.
aur-updater --config packages.toml --dry-run
```

Run it from an environment with Arch packaging tools available:

```bash
pacman -Syu --noconfirm --needed base-devel git pacman-contrib npm
```

`--dry-run` checks versions and required tools, but does not edit files.

`GITHUB_TOKEN` env variable is optional. When set, it is used only for `github_release`
requests to raise GitHub API rate limits.

## Config

Supported sources:

- `github_release`: latest non-draft, non-prerelease GitHub release
- `npm`: npm registry `/latest`
- `git`: clone a branch and compute a VCS-style version

Example:

```toml
[[package]]
name = "openai-codex-bin"
path = "aur/openai-codex-bin"
source = "github_release"
repo = "openai/codex"
strip_prefixes = ["rust-", "v"]
exclude_tags = ["nightly", "preview"]
reset_pkgrel = true

[[package]]
name = "claude-agent-acp"
path = "aur/claude-agent-acp"
source = "npm"
npm_package = "@agentclientprotocol/claude-agent-acp"
reset_pkgrel = true

[[package]]
name = "btrfs-desktop-notification-git"
path = "aur/btrfs-desktop-notification-git"
source = "git"
git_url = "https://gitlab.com/Zesko/btrfs-desktop-notification.git"
branch = "master"
version_template = "r{rev}.{sha7}"
reset_pkgrel = true
```

Version normalization strips configured prefixes and replaces `-` with `_` for
Arch `pkgver` compatibility.

## Git Templates

For `source = "git"`, the default template is:

```toml
version_template = "r{rev}.{sha7}"
```

Available tokens:

- `{rev}`: `git rev-list --count HEAD`
- `{sha}`: full commit SHA
- `{shaN}`: first `N` SHA characters, for example `{sha7}` or `{sha10}`
- `{date}`: latest commit date as `YYYYMMDD`
- `{base}`: configured `base_version`

Examples:

```toml
version_template = "r{rev}.{sha7}"
version_template = "{date}.r{rev}.g{sha7}"
version_template = "{base}.r{rev}.g{sha10}"
base_version = "0.13.0"
```

## GitHub Actions

The intended setup is a separate AUR maintenance repo that downloads a released
`aur-updater` binary and runs it inside an Arch container:

```yaml
- name: Run updater
  env:
    # Optional: used only to raise GitHub API rate limits for github_release.
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  run: ./aur-updater --config packages.toml
```

After the updater runs, let the workflow commit changed `PKGBUILD` and
`.SRCINFO` files.

This repository includes a tag-based release workflow that builds
`aur-updater-x86_64-unknown-linux-musl` and uploads it as a GitHub Release
asset.
