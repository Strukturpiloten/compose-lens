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
`scripts/shared-ci.py` verifies the exact archive before dispatch. Local, CI and image file tools, including Actionlint, use `scripts/install-host-tools.sh` and the
selected shared architecture-specific integrity inventory.
Local SemVer preflight and development API checks share the verified compiler and isolated storage.

The CI qualification recipe derives Rust from `rust-toolchain.toml` and exports the same canonical
downloaded tools and shared Python executables from the selected automation source. Its Feature
graph and lock remain unchanged. `scripts/install-file-tools.sh` only forwards compatibility calls
to the canonical launcher; it contains no versions, checksums or downloader. The obsolete
version-only installer owner is removed. Shared metadata owns each exact release/checksum pair
for both architectures and groups atomic updates; callers retain only the selected source SHA.
A late `enabled: false` rule still holds image and Feature proposals without a manual approval
route. Package/compiler inputs retain their ordinary owners.

VS Code Dev Containers use the configured existing Podman storage, preferably rootless. Ordinary
validation requires no private Podman store, root bridge or local image controls. Fresh standard
GitHub CI image qualification and subsequent updater activation are separate requirements under
issue #563. The qualification caller does not itself establish native image or App evidence.

`yaml-edit` remains a private exact dependency under [ADR 0002](decisions/0002-loss-aware-yaml-syntax.md).
