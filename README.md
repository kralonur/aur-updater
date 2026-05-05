# aur-updater

Rust CLI for updating AUR package directories.

It reads a TOML config, checks upstream versions, updates `pkgver` in each
`PKGBUILD`, optionally resets `pkgrel=1`, then regenerates checksums and
`.SRCINFO`.

## Usage

```bash
# Update every enabled package in the config.
aur-updater update --config packages.toml

# Update only one package.
aur-updater update --config packages.toml --package your-package

# Check versions and tools without editing files.
aur-updater update --config packages.toml --dry-run
```

Run it from an environment with Arch packaging tools available:

```bash
pacman -Syu --noconfirm --needed base-devel git pacman-contrib npm
```

`--dry-run` checks versions and required tools, but does not edit files.

`GITHUB_TOKEN` env variable is optional. When set, it is used only for `github_release`
requests to raise GitHub API rate limits.

## Local Users

Use this flow when an AUR package is stale and you want to build a newer version
locally without maintaining or publishing anything.

```bash
mkdir -p ~/tmp/aur-update
cd ~/tmp/aur-update

git clone https://aur.archlinux.org/your-package.git
```

Create `packages.toml` next to the cloned package:

```toml
[[package]]
name = "your-package"
path = "your-package"
source = "github_release"
repo = "owner/project"
strip_prefixes = ["v"]
exclude_tags = ["nightly", "preview"]
reset_pkgrel = true
```

Check what would change:

```bash
aur-updater update --config packages.toml --dry-run
```

Apply the update:

```bash
aur-updater update --config packages.toml
```

Then build and install the package locally:

```bash
cd your-package
makepkg -si
```

`aur-updater` already runs `updpkgsums` and regenerates `.SRCINFO`; you do not
need to run `makepkg --printsrcinfo` yourself unless you make additional manual
PKGBUILD edits.

## Local Maintainers

If you maintain the AUR package, use the same local flow, then review, commit,
and push the package repository:

```bash
git -C your-package diff
git -C your-package add PKGBUILD .SRCINFO
git -C your-package commit -m "Update to 1.2.3"
git -C your-package push
```

## GitHub Actions

For GitHub-based automation, check `.github/workflows/package-update.yml`.

It is a real workflow for this repository's `examples/packages.toml`, and it is
also the copyable workflow for other maintainers. Copy it into your maintenance
repository, then adjust `CONFIG_PATH` and the `push.paths` entries to match your
package layout.

The PKGBUILDs in `examples/` are intentionally simple and are not meant to be
complete packaging recipes. Their purpose is to show package version updates,
checksum refreshes, and `.SRCINFO` generation for each supported source type.

Minimal workflow step:

```yaml
- name: Run updater
  env:
    # Optional: used only to raise GitHub API rate limits for github_release.
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  run: ./aur-updater update --config packages.toml
```

## Config

Supported sources:

- `github_release`: latest non-draft, non-prerelease GitHub release
- `npm`: npm registry `/latest`
- `git`: clone a branch and compute a VCS-style version

Example:

```toml
[[package]]
name = "your-package"
path = "aur/your-package"
source = "github_release"
repo = "owner/project"
strip_prefixes = ["v"]
exclude_tags = ["nightly", "preview"]
reset_pkgrel = true

[[package]]
name = "your-npm-package"
path = "aur/your-npm-package"
source = "npm"
npm_package = "@scope/package"
reset_pkgrel = true

[[package]]
name = "your-package-git"
path = "aur/your-package-git"
source = "git"
git_url = "https://example.com/owner/project.git"
branch = "main"
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

This repository includes a tag-based release workflow that builds
`aur-updater-x86_64-unknown-linux-musl` and uploads it as a GitHub Release
asset.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for test requirements, commit message
rules, and pull request guidelines.
