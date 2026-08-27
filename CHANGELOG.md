# Changelog

All notable changes to ComposeLens will be documented in this file. The project follows
[Semantic Versioning](https://semver.org/) with the pre-1.0 policy documented in
[`docs/api-stability.md`](docs/api-stability.md).

## [Unreleased]

## [0.3.1](https://github.com/Strukturpiloten/compose-lens/compare/v0.3.0...v0.3.1) - 2026-08-27

### Added

- Add caller-authorized environment-file and secret-value resolution with source-aware empty/unset
  states and redacted protected payloads ([#70](https://github.com/Strukturpiloten/compose-lens/issues/70)).

### Changed

- Promote curated `Unreleased` notes under a blank-line-safe release-plz version heading instead
  of generating duplicate changelog groups ([#79](https://github.com/Strukturpiloten/compose-lens/issues/79)).

- Sort generated service environment entries by key while retaining duplicate relative order;
  authored and canonical ordering remains unchanged
  ([#70](https://github.com/Strukturpiloten/compose-lens/issues/70)).
- Replace repeated field, API, test, and milestone ledgers with bounded task-oriented guides that
  link to canonical schema and test evidence, and consolidate rendering, fixture, and conformance
  maintenance guidance
  ([#72](https://github.com/Strukturpiloten/compose-lens/pull/72)).

## [0.3.0](https://github.com/Strukturpiloten/compose-lens/compare/v0.2.0...v0.3.0) - 2026-08-19

### Added

- [**breaking**] emit marker-first minimally quoted YAML ([#66](https://github.com/Strukturpiloten/compose-lens/pull/66))

### Changed

- Automates version and changelog preparation with release-plz, makes this changelog the sole
  release-history source, and retains the protected trusted-publishing workflow as publisher.

## [0.2.0] - 2026-08-17

### Added

- Native authored and effective-project coverage for service CPU, memory, namespace, OOM, scale,
  device-cgroup, and `volumes_from` keys, retaining merge provenance, interpolation sensitivity,
  malformed evidence, local service-reference validation, and deterministic parse-back-generated
  raw syntax without runtime inference.
- Completes structured authored/effective coverage for the current closed-schema Compose keys,
  including includes, models, GPU selectors, development watches, and local model references.
- Adds authored and effective long-volume mount options for consistency, recursive binds, image,
  tmpfs, and named-volume subsettings; generation and runtime interpretation remain excluded.

### Changed

- Starts the 0.2.x API line with one complete resource `external()` getter; removes the
  compatibility-only `external_syntax()` path and renames the deprecated Compose name-mapping
  model.
- SemVer validation derives the release type from Cargo package versions instead of forcing every
  candidate to be a patch.

## [0.1.17] - 2026-08-13

### Added

- Paired positive and negative tests for exact implementation-version parsing, component access,
  inclusive ranges, overflow, malformed input, and inverted bounds.
- A pinned CI and release coverage ratchet for the locked all-feature, all-target test suite.
- A shared VS Code and Dev Container workflow with one-command Rust, file-quality, policy,
  coverage, MSRV, offline-link, package, and API checks.

### Fixed

- Local API checks always use an isolated writable Cargo cache instead of the container image's
  potentially read-only global package lock.

## [0.1.16] - 2026-08-10

### Added

- Expands loss-aware authored and effective Build support, including contexts, inputs, caches,
  Dockerfiles, resource settings, and redacted SSH forms.
- Expands the Deploy model with endpoint and replica settings, labels, lifecycle configuration,
  placement, limits and reservations, and schema-only device and resource evidence.
- Adds source-aware service configuration for `credential_spec`, `extends`, `provider`, lifecycle
  hooks, runtime/pull/platform strings, cgroup values, and CPU scalar categories.
- Adds `attach` and `blkio_config` preservation, terminal/security and logging support, and
  generated service-network attachments.
- Expands authored and merged IPAM plus generated application-owned network and volume drivers,
  options, labels, and conflict diagnostics.
- Preserves 0.1.x source compatibility and Rust 1.85.0+ support; provider and runtime behavior
  remain outside the evidence boundary.

## [0.1.15] - 2026-08-07

### Added

- Loss-aware authored, merged, and generated support for service `dns`, `dns_opt`,
  `dns_search`, `expose`, and `annotations`.
- Raw `security_opt` support with diagnostic candidates for AppArmor, seccomp,
  no-new-privileges, SELinux labels, Mask, and Unmask. Ambiguous values are never selected
  silently.
- Field-specific merge behavior, provenance, sensitivity, reset/override state, and deterministic
  parse-back-validated generation.

## [0.1.14] - 2026-08-06

### Added

- Loss-aware lifecycle and identity support for `entrypoint`, `init`, `stop_signal`,
  `stop_grace_period`, `pull_policy`, and `hostname`.
- Resource and container-setting support for `pids_limit`, `shm_size`, `mem_limit`, `tmpfs`,
  `sysctls`, `ulimits`, `cap_add`, `cap_drop`, and `devices`.
- Authored, effective-project, and safe generated boundaries retain source evidence without
  claiming host or runtime enforcement.

## [0.1.13] - 2026-08-06

### Added

- Deterministic generated service `env_file` output with ordered short and long syntax, explicit
  `required`/`format: raw` options, sensitive-path redaction, and typed parse-back validation.

## [0.1.12] - 2026-08-05

### Added

- Source-aware service `env_file` scalar, ordered list, and long syntax in the authored document
  model and effective multi-file project view.
- Raw-preserving `path`, `required`, and `format` options, complete merge provenance, sensitive
  interpolation redaction, and recoverable malformed-entry diagnostics without file I/O.

## [0.1.11] - 2026-08-05

### Added

- Raw-preserving service-level `restart` policies in the source-aware document and effective
  multi-file project view, including deferred interpolation, replacement provenance, retry-count
  spelling, and recoverable invalid-value diagnostics.
- Deterministic `GeneratedRestartPolicy` output for `no`, `always`, `on-failure[:max-retries]`, and
  `unless-stopped`, with duplicate-singleton rejection and typed parse-back validation.

## [0.1.10] - 2026-08-05

### Added

- Source-aware `container_name` support in the single-document model and effective multi-file
  project view, including replacement provenance and malformed-value recovery.
- Deterministic `GeneratedService::set_container_name` output with Compose name-grammar and
  duplicate-singleton validation.

## [0.1.9] - 2026-08-05

### Fixed

- Accept valid hyphenated YAML anchor and alias names, including scalar, sequence, and mapping
  values, while retaining original source bytes and guarding normalized-name collisions.
- Keep unquoted `--option` block-sequence items scalar, including values containing commas.
- Accept an indented mapping value after one or more blank lines without losing its parent key.
- Resolve direct aliases only after structural parser recovery so an anchored mapping cannot leak
  fields into an unrelated parent mapping.

## [0.1.8] - 2026-08-05

### Added

- Native service labels in the source-aware document and merged project view, retaining mapping,
  `KEY=VALUE`, and key-only list syntax with full merge provenance.
- Deterministic generated service-label mappings with explicit empty values, duplicate-name
  rejection, parse-back validation, and sensitive-value redaction.

## [0.1.7] - 2026-08-04

### Added

- Typed deterministic construction for new Compose documents covering the first runtime-migration
  service, network, and volume subset.
- Parse-back validation through the loss-aware syntax and native typed-document layers.
- Deliberate short syntax for `SELinux` relabel binds, duplicate/ambiguity rejection, and sensitive
  generated-output redaction.

## [0.1.6] - 2026-08-03

### Added

- Effective service `configs` and `secrets` in the native merged-project view.
- Short/long grant fidelity, field-level provenance, retained unknown options, and recoverable
  malformed-form diagnostics for downstream conversion adapters.
- Multi-file regressions for Compose's unique-by-target grant merging, including nested fields
  retained from earlier files.

## [0.1.5] - 2026-08-03

### Added

- Native document and merged-project support for service `group_add`, `working_dir`, and
  `read_only` values.
- Effective `user` and `userns_mode` values in the merged-project view, completing the first
  execution-identity consumer boundary.
- Multi-file field/item provenance, malformed-form recovery, and public-consumer coverage for the
  additive identity API.

## [0.1.4] - 2026-08-03

### Added

- A source-aware merged-project `depends_on` view that retains effective short and long syntax,
  ordered service edges, nested condition/restart/required values, unknown fields, and complete
  multi-file provenance.
- Sensitive interpolated semantic keys report their sensitivity and redact their values from
  `Debug` output.
- Recoverable malformed-form coverage and a public-consumer dependency contract for downstream
  conversion engines.

## [0.1.3] - 2026-08-03

### Added

- A source-aware merged-project `healthcheck` view with field-level provenance for command,
  interval, timeout, retries, start period, start interval, and explicit disabling.
- Multi-file, disabled-check, malformed-form, and public-consumer coverage for the additive API.

## [0.1.2] - 2026-08-03

### Added

- Source-aware `extra_hosts` entries in the native merged-project view for both sequence and
  mapping syntax, including per-entry provenance and raw-preserving IPv4, IPv6, deferred-value,
  and `host-gateway` classification.

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
