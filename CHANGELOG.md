# Changelog

All notable changes to ComposeLens will be documented in this file. The project follows
[Semantic Versioning](https://semver.org/) with the pre-1.0 policy documented in
[`docs/api-stability.md`](docs/api-stability.md).

## [Unreleased]

## [0.1.1] - 2026-08-02

### Added

- A source-aware native `project` view over merged and optionally profile-selected projects, with
  complete field, collection, item, and key provenance for the first BoxFerry conversion boundary.
- Recoverable project-view diagnostics, unmodeled-field references, and sensitive-value `Debug`
  redaction.

### Fixed

- Accept complete valid unquoted block plain scalars containing commas, including short volume
  options such as `./data:/data:Z,ro`, without changing the authored source or byte spans.
- Detect and diagnose any remaining incomplete source retention by the private YAML backend instead
  of allowing typed processing to continue over a silently truncated document.

## [0.1.0] - 2026-08-02

### Added

- Release-candidate implementation for ComposeLens 0.1.0.
- Loss-aware YAML syntax with source spans and structured diagnostics.
- Source-aware native Compose types for the first BoxFerry conversion boundary.
- Explicit interpolation, loading, merging, profiles, path/default/reference resolution, and
  exact-version compatibility profiles.
- Deterministic canonical rendering and atomic preservation-oriented scalar editing.
- Reviewed provider-config evidence for four Docker Compose and two `podman-compose` versions.
- Exact planned rootless/rootful Podman and Docker runtime-effect matrices.
- Licensed TYPO3 and Docker Awesome Compose real-world regression fixtures.
- Raw-preserving `extra_hosts`, user/group, `userns_mode`, unlimited `ulimits`, health checks, and
  dependency-condition types with document and post-merge validation.
- Host-independent container-path classification and independently identifiable build/deploy
  subfields.
- Evidence-backed compatibility findings for `host-gateway` and Podman user-namespace values,
  anchored to official Podman 5.4 documentation without claiming untested provider pass-through.
