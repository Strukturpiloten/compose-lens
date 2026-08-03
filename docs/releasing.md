# Release process

ComposeLens 0.1.0 is published on crates.io. Later releases are started manually through the
protected GitHub Actions [release workflow](../.github/workflows/release.yml); the workflow has no
version input and authenticates to crates.io only through trusted publishing.

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

The workflow scopes its short-lived registry credential to the publication step. The GitHub
environment stores no crates.io API token.

## Trusted publishing

The crates.io trusted publisher must match the release job exactly:

- GitHub owner: `Strukturpiloten`
- repository: `compose-lens`
- workflow: `release.yml`
- environment: `release`

Do not add a crates.io token as a GitHub secret or variable. The authentication action exchanges
the job's GitHub OIDC identity for a short-lived crates.io token and revokes it when the job ends.
`CARGO_REGISTRY_TOKEN` exists only in the `cargo publish` step and is populated from that temporary
token. After a successful OIDC-authenticated release, crates.io may be configured to require
trusted publishing for the crate.

The 0.1.0 bootstrap token was a one-time ownership-establishment credential. It has no supported
path in the current workflow and must remain revoked.

## Routine release

For later versions, update only the workspace package version, changelog, and matching
`docs/releases/<version>.md` release notes in a reviewed pull request. After CI succeeds, run
the release workflow from the default branch and approve the `release` environment deployment.

Do not create the tag or GitHub release manually. If a run fails, inspect its draft release and
rerun from the same commit; the workflow verifies an existing tag, replaces the workflow-owned
draft and its generated assets, and skips a crate version that is already present on crates.io.

The workflow re-runs quality checks, including a patch-level public-API comparison with the latest
normal crates.io release, builds the locked crate archive, creates a SHA-256 checksum and
provenance attestation, creates an annotated tag and workflow-owned draft GitHub release,
publishes the crate, and then publishes the GitHub release. The semver action is pinned by full
commit and exact release tag; Renovate maintains both. A failure before the final step leaves the
GitHub release as a draft for inspection and retry.

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
