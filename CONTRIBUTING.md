# Contributing

Thanks for taking time to improve `aur-updater`. Keep changes focused and easy
to review.

## Before Opening a Pull Request

Run these checks locally:

```bash
cargo fmt --check
cargo test
```

If you add a new source type, include a matching example package and verify that
example locally. From the new example package directory, check that Arch can
parse the package metadata:

```bash
makepkg --printsrcinfo
```

Run that command from the changed package directory.

For new source types, small focused tests are strongly encouraged. They help
future contributors understand the source behavior, version parsing, version
normalization, and source-specific edge cases.

## Commit Messages

Use Conventional Commits. Commit messages should pass commitlint-style rules:

```text
type(scope): summary
```

The scope is optional:

```text
feat: add npm source support
fix(config): reject empty package paths
docs: update local usage instructions
test: cover git version templates
chore: update examples
```

Use one of these common types:

- `feat`: user-facing feature
- `fix`: bug fix
- `docs`: documentation-only change
- `test`: tests only
- `refactor`: code change without behavior change
- `chore`: maintenance, examples, tooling, or release work

Keep the summary short, lowercase after the type, and written in the imperative
style.

## Pull Request Guidelines

- Explain what changed and why.
- Mention any behavior or CLI output changes.
- Include tests for behavior changes when practical.
- Keep unrelated cleanup out of the pull request.
- Do not commit generated build artifacts, downloaded source archives, or local
  package build directories.
