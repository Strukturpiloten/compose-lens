# Release process

Release preparation is automated by release-plz. The protected `Release` workflow remains the
only component allowed to publish the crate, create a tag, or create a GitHub release.

## One-time GitHub setup

1. Create one organization-owned GitHub App for the three Strukturpiloten repositories. Disable
   webhooks and grant repository **Contents: read and write** and **Pull requests: read and
   write**.
2. Install the App on `boxferry`, `compose-lens`, and `quadlet-lens`. Whenever repository
   permissions change, review and approve the updated installation permissions in the
   organization before rerunning the workflow.
3. Store the App client ID as the organization Actions variable
   `RELEASE_PLZ_APP_CLIENT_ID`. Store only the private key as the organization Actions secret
   `RELEASE_PLZ_APP_PRIVATE_KEY`. Limit both to these three repositories.
4. Keep the default workflow token read-only. The App token is used only to create or update the
   release pull request so that normal pull-request CI runs.
5. Keep the protected `release` environment, required reviewer, default-branch restriction,
   crates.io trusted publisher, tag ruleset, and immutable-release setting unchanged.
6. Require the stable `PR gate` status check in default-branch protection instead of enumerating
   its implementation jobs individually.

The release-plz configuration disables Cargo publication, Git tags, and GitHub releases. Its only
write operation is the `release-plz-*` preparation branch and pull request. The trusted publisher
continues to identify repository `compose-lens`, workflow `release.yml`, and environment
`release`; no crates.io token belongs in GitHub secrets.

## Routine release

1. Merge ordinary reviewed changes into the default branch. No release issue, local release
   branch, or manually created release pull request is needed.
2. Review the release-plz pull request. It updates the Cargo version, lockfile, and root
   `CHANGELOG.md`. Normal CI must pass before merge.
3. Merge the release-plz pull request. Only a merged pull request whose head starts with
   `release-plz-` dispatches the protected `Release` workflow.
4. Approve the `release` environment deployment. The workflow revalidates the repository,
   publishes through trusted publishing, attaches the attested crate and checksum, and publishes
   the immutable GitHub release.

Use concise pull-request titles such as `feat: ...`, `fix: ...`, or `feat!: ...`. Release-plz also
accepts other titles, but these prefixes make version selection and changelog grouping clearer.
For intentional pre-1.0 public breaks, use a breaking title and review the resulting minor version.

GitHub release notes are extracted from the matching version section in `CHANGELOG.md`, which is
the only release-history source in the repository. Keep changelog entries short and move technical
detail into the canonical topic documentation. On the first automated release pull request, move
the existing hand-authored `[Unreleased]` material into the generated version section if
release-plz did not already represent it, then leave `[Unreleased]` empty. Future entries come from
merged pull-request and commit titles.

## Recovery

`workflow_dispatch` remains available for release-plz preparation and protected publication
retries. Rerun `Release` from the same default-branch commit after a transient failure; the
workflow verifies an existing tag, replaces only its own draft release, and skips a crate version
already visible on crates.io. Never replace a published tag or release. If a corrected workflow
needs a new commit after an unpublished tag was created, remove only that unpublished tag before
retrying.
