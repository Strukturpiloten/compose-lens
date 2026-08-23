# Release process

release-plz prepares version pull requests. The protected `Release` workflow is the only component
allowed to publish the crate, create a tag, or create a GitHub release.

## One-time GitHub configuration

- Install the organization-owned release GitHub App on `boxferry`, `compose-lens`, and
  `quadlet-lens` with repository Contents and Pull requests read/write permissions; disable webhooks.
- Store its client ID as organization variable `RELEASE_PLZ_APP_CLIENT_ID` and its private key as
  organization secret `RELEASE_PLZ_APP_PRIVATE_KEY`, limited to those repositories.
- Keep the default workflow token read-only. The App token is used only for preparation branches and
  pull requests so normal pull-request CI runs.
- Protect the `release` environment with a required reviewer and default-branch restriction.
- Keep crates.io trusted publishing bound to repository `compose-lens`, workflow `release.yml`, and
  environment `release`. Do not store a crates.io token in GitHub.
- Require the stable `PR gate` status in branch protection.

The preparation configuration disables publication, tags, and GitHub releases. Those actions remain
inside the protected workflow.

## Routine release

1. Merge a release-worthy code change into `main`.
2. Review the release-plz pull request, including Cargo version, lockfile, and `CHANGELOG.md`.
3. Merge it only after normal CI passes.
4. Approve the protected `release` environment deployment.
5. Verify trusted publication, the attested crate and checksum, tag, and immutable GitHub release.

Only a merged pull request whose head starts with `release-plz-` dispatches publication. No local
release branch, manually created tag, or crates.io API token is needed.

## Release classification

GitHub uses the pull-request title as the squash commit title. release-plz prepares a release only
when an unreleased commit uses `feat`, `fix`, `perf`, `refactor`, or `revert`, with an optional scope
and breaking `!`. Intentional pre-1.0 API breaks use a breaking title and normally produce a minor
version.

Use `docs`, `test`, `ci`, `build`, `style`, or `chore` for non-release work. A shipped behavior change
with a maintenance title will not be published automatically; choose its title deliberately.

## Changelog

`CHANGELOG.md` is the release-note source. Each version needs a usable release section before
publication. Keep entries concise and link to canonical topic documentation rather than copying
models, fixtures, or test inventories.

release-plz owns generated layout, and `.prettierignore` excludes only the changelog from Prettier.
Markdownlint and release-structure validation still apply.

## Recovery

Use `workflow_dispatch` to retry preparation or protected publication. Rerun `Release` from the same
default-branch commit after a transient failure; it verifies an existing tag, replaces only its own
draft release, and skips a crate already visible on crates.io.

Never replace a published tag or release. If a workflow correction needs a new commit after an
unpublished tag was created, remove only that unpublished tag before retrying.
