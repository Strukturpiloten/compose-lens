# Release process

ComposeLens is the only repository in the initial three-project set that is currently configured
for crates.io publication. Releases are started manually through the protected GitHub Actions
[release workflow](../.github/workflows/release.yml); the workflow has no version input.

## Version sources

- The package version is declared once in the workspace `Cargo.toml`.
- The MSRV is declared once as `rust-version` in the workspace `Cargo.toml`.
- The current development toolchain is declared in `rust-toolchain.toml`.
- The release workflow derives the package version with `cargo metadata`.

Do not add a second version file or type a version into a workflow form. Normal Rust development
uses Cargo's manifest as the package-version source of truth.

## One-time GitHub setup

In the ComposeLens repository settings:

1. Create an environment named `release`.
2. Add Martin “Becks” Beckert as a required reviewer for that environment.
3. Restrict deployment branches to the default branch.
4. Set the default workflow token permission to read-only. The release job requests only its
   explicit write permissions.
5. Protect the default branch and the `v*` tag namespace with rulesets. Permit the release
   workflow to create release tags.
6. Enable immutable releases under the repository's release settings.

The workflow scopes registry credentials to publication steps; third-party actions never receive
the bootstrap crates.io token.

## First crates.io publication

Crates.io cannot register a trusted publisher until the crate exists. For version 0.1.0 only:

1. Create a crates.io API token that is allowed to publish the new `compose-lens` crate.
2. Add it as the `CRATES_IO_BOOTSTRAP_TOKEN` secret on the protected `release` environment,
   not as a repository-wide variable.
3. Prepare the release change:
   - keep a fresh `[Unreleased]` section in `CHANGELOG.md`;
   - move the release entries under `## [0.1.0] - YYYY-MM-DD`;
   - review `docs/releases/0.1.0.md` as the public-facing GitHub release notes;
   - update the workspace package version if it is not already correct.
4. Merge the release change to the default branch and wait for CI.
5. Open **Actions → Release → Run workflow** and select the default branch.

The workflow re-runs quality checks, builds the locked crate archive, creates a SHA-256 checksum
and provenance attestation, creates an annotated tag and workflow-owned draft GitHub release,
publishes the crate, and then publishes the GitHub release. A failure before the final step leaves
the GitHub release as a draft for inspection and retry.

## Switch to trusted publishing

Immediately after the first crate is visible on crates.io:

1. Configure a trusted publisher for owner `Strukturpiloten`, repository `compose-lens`,
   workflow `release.yml`, and environment `release`.
2. Remove `CRATES_IO_BOOTSTRAP_TOKEN` from GitHub.
3. Revoke the bootstrap API token on crates.io.
4. After one successful OIDC-authenticated release, optionally require trusted publishing for the
   crate on crates.io.

Future versions intentionally fail if the bootstrap token is still present. The workflow obtains
a short-lived crates.io token through OIDC and revokes it automatically after the job.

## Routine release

For later versions, update only the workspace package version, changelog, and matching
`docs/releases/<version>.md` release notes in a reviewed pull request. After CI succeeds, run
the release workflow from the default branch and approve the `release` environment deployment.

Do not create the tag or GitHub release manually. If a run fails, inspect its draft release and
rerun from the same commit; the workflow verifies an existing tag, replaces the workflow-owned
draft and its generated assets, and skips a crate version that is already present on crates.io.

## Recovering a failed workflow

The crates.io publication-state probe runs before the workflow creates a tag, attestation, or
draft release. A failure at that probe is therefore safe to rerun after the external service
recovers.

If a later step fails after the tag and draft release exist, rerun from the same commit. The
workflow verifies the tag, deletes only the matching draft release by its numeric release ID, and
creates a fresh draft with the reviewed notes, crate, and checksum. Do not add manual assets or
notes to this workflow-owned draft because a retry intentionally replaces it.

The workflow manages draft lifecycle through GitHub's low-level Releases API. It takes the numeric
release ID and asset upload URL directly from the create response and uses them for every later
operation. Do not replace this with an immediate lookup through `gh release view`, `gh release
list`, or the published-release-by-tag endpoint: draft URLs use an `untagged-*` slug, CLI JSON
fields vary by installed version, and a newly created draft must not need to become list-visible
before the workflow can continue. See GitHub's [create-release API][github-create-release] and
[release-asset API][github-upload-release-asset].

If fixing the workflow itself requires a new commit, delete the unpublished remote tag first so
the corrected workflow can bind the version to the new commit. An existing draft may remain; the
workflow will replace it after verifying that it is still a draft. Never delete or replace a tag
or release that has already been published.

[github-create-release]: https://docs.github.com/en/rest/releases/releases#create-a-release
[github-upload-release-asset]: https://docs.github.com/en/rest/releases/assets#upload-a-release-asset
