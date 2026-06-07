# Publishing

## GitHub Pages

The public documentation is built from the Markdown files in `docs/` using
MkDocs. The generated site is written to `site/` locally and deployed by GitHub
Actions.

The site is served at:

```text
https://macvmtop.hongjunwu.com/
```

The custom domain is included in `docs/CNAME`, so MkDocs copies it into the
published artifact.

Repository setup:

1. Open the repository settings on GitHub.
2. Go to Pages.
3. Set the build and deployment source to GitHub Actions.
4. Confirm the custom domain is `macvmtop.hongjunwu.com`.

The workflow is defined in:

```text
.github/workflows/pages.yml
```

It runs when documentation files, `mkdocs.yml`, or the Pages workflow change on
`main`. It can also be started manually from the Actions tab.

## Local Documentation Build

```sh
python3 -m venv .venv
. .venv/bin/activate
python -m pip install --upgrade pip
python -m pip install -r docs/requirements.txt
mkdocs build --strict
```

Local output is generated under `site/` and ignored by git.

## Install Script

The install script is served from:

```text
https://macvmtop.hongjunwu.com/install.sh
```

It lives at `docs/install.sh` and is copied to the site root by MkDocs. Keep it
POSIX `sh` compatible.

## Crate Publish Readiness

Before publishing a release:

```sh
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo package --list
cargo package
```

Check the package list before publishing. It should include source,
documentation, and metadata files, and it should not include local reference
checkouts or generated build output.

## GitHub Releases

Version tags create GitHub Releases automatically:

```sh
git tag vX.Y.Z
git push origin vX.Y.Z
```

The release workflow is defined in:

```text
.github/workflows/release.yml
```

It runs on tags matching `v*.*.*`, verifies that the tag matches the version in
`Cargo.toml`, runs the Rust checks, builds macOS release archives, writes
checksums, and creates a GitHub Release.

Release assets are named for the install script:

```text
macvmtop-aarch64-apple-darwin.tar.gz
macvmtop-x86_64-apple-darwin.tar.gz
```

The release workflow can also be started manually from the Actions tab with an
existing tag.

Current release binaries are ad-hoc signed by the linker and are not Developer
ID signed or notarized. `spctl --assess --type execute` rejects the current
release binaries. Shipping Developer ID signed binaries requires Apple
Developer Program credentials and a notarization step in the release workflow.

When signing is added, the release workflow should:

- import a Developer ID Application certificate from GitHub Actions secrets
- sign each target binary before packaging
- submit the signed archives or binaries to Apple notarization
- staple notarization tickets where applicable
- keep publishing `SHA256SUMS` for installer verification

## Reference Checkouts

The local reference repositories under `references/` are research inputs. They
must stay outside git history and crate packages.

Current guardrails:

- `.gitignore` ignores `/references/`
- `Cargo.toml` excludes `/references` from crate packaging
- `Cargo.toml` excludes `/target` and `/site` from crate packaging
