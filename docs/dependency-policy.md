# Dependency policy

ComposeLens keeps its own Cargo graph and never depends on BoxFerry product crates. BoxFerry
owns reusable development automation through the immutable revision in `.github/automation.json`.

## Canonical policy and ownership

`.github/renovate.json` extends the common and Rust presets at the manifest-selected revision.
They own grouping, automatic reviewed pull requests, immutable source pins, lockfile maintenance
and approval policy. Consumer managers retain only repository-specific declarations. Every pin
has one extraction owner; update consumers and Renovate evidence together.

Ordinary third-party releases retain the 72-hour hold. A verified isolated first-party release
may waive age for its whole necessary Cargo resolution through the canonical trusted closure
check. Mixed updates, other registries and metadata failures do not qualify. Integrity, audit,
compatibility and exact-candidate checks still apply. Ordinary Rust dependencies remain active.

Trusted dependency admission reads the exact base manifest. Candidate code cannot authorize its
own admission. Required CI verifies every selected ordinary job and independently planned skip;
main, manual, final local and release validation remain complete for that ordinary scope.

Native provider targets, bootstrap artifacts and historical observations stay fixed. Preserve
native/custom discovery and integrity without automatic proposals or manual update routes.

## Integrity and tooling

Rust/package/compiler declarations remain in `rust-toolchain.toml`, `Cargo.toml` and `Cargo.lock`.
The selected shared Python lock owns uv, Ruff and ty; no second Python lock or mypy is added.
`scripts/shared-ci.py` verifies the exact archive before dispatch. Ordinary host and CI file tools
use `scripts/install-host-tools.sh` and the selected shared architecture-specific integrity inventory.
Local SemVer preflight and development API checks share the verified compiler and isolated storage.

The current-main Dev Container Dockerfile, lifecycle verification, Feature graph and lock remain
unchanged. Its legacy `scripts/install-file-tools.sh` is image-only, with one retained extractor.
A late `enabled: false` rule holds image, Feature and image-tool proposals without a manual
approval route. Host tooling never falls back to this installer. Existing image ARG versions do
not become ordinary tool authorities; package/compiler inputs retain their ordinary owners.

VS Code Dev Containers use the configured existing Podman storage, preferably rootless. Ordinary
validation requires no private Podman store, root bridge or local image controls. Fresh standard
GitHub CI image qualification and subsequent updater activation are separate requirements under
issue #563; the unchanged image closure is not newly qualified by ordinary adoption.

`yaml-edit` remains a private exact dependency under [ADR 0002](decisions/0002-loss-aware-yaml-syntax.md).
