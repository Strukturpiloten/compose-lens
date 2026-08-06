# Changelog

All notable changes to ComposeLens will be documented in this file. The project follows
[Semantic Versioning](https://semver.org/) with the pre-1.0 policy documented in
[`docs/api-stability.md`](docs/api-stability.md).

## [Unreleased]

## [0.1.14] - 2026-08-06

### Added

- Source-aware service `mem_limit` support across authored documents, effective projects, and
  generated output. Exact scalar text/kind, documented lowercase units, lexical zero, deferred,
  schema-number, provider-dependent string, provenance, and sensitivity remain available without
  fixed-width parsing. Generated output accepts only a quoted positive ASCII decimal plus an
  explicit documented unit. Six provider rows remain planned; only explicit byte-unit values are
  candidates for exact cross-format handling, and no runtime, host, cgroup, deploy, or provider
  behavior is inferred.

- Source-aware service `devices` support across authored documents, established Compose-Go-compatible
  target-keyed merge, effective projects, and deterministic generated output. Ordered mixed raw
  short forms and long `source`/`target`/`permissions` mappings retain omission, explicit empty,
  duplicates, CDI/deferred/opaque evidence, nested provenance, sensitivity, extensions, unknown
  fields, reset, and override. Generated output rejects unsafe or deferred strings and validates
  its bytes by parsing them back, without probing devices, parsing colon triples, validating CDI or
  permissions, or claiming runtime access. Six provider-config rows remain planned-only.

- Effective-project and generated-document support for ordered service `ulimits`. The project view
  retains lowercase names, scalar/range form, authored and interpolated spelling, YAML scalar kind,
  member and field provenance, sensitivity, omission, explicit empty/reset mappings, recursive
  soft/hard merging, replacement, and override. Malformed entries produce stable diagnostics
  without deleting valid siblings. Generated output accepts only unique lowercase names and
  resolved `-1` or non-negative ASCII decimals, quotes every value, preserves order and explicit
  empty maps, and parse-back validates. Six provider-config rows remain planned-only and make no
  runtime enforcement, default, accounting, or cross-format claim.

- Source-aware service `sysctls` support across authored documents, generic map/list multi-file
  merge, effective projects, and deterministic generated output. Omission, explicit empty map/list
  forms, ordered map entries, scalar kind and spelling, ordered list strings, spans, interpolation
  sensitivity, provenance, `!reset`, and `!override` remain visible. Invalid shapes and exact list
  duplicates produce stable recoverable diagnostics; generation emits only resolved quoted string
  assignments/items and rejects unsafe or duplicate input. Six provider-config rows remain planned
  without claiming runtime application, namespace validity, privileges, or cross-format equivalence.

- Source-aware service-level `tmpfs` support across authored documents, ordinary multi-file merge,
  effective projects, and generated output. Scalar/list form, omission, explicit empty lists,
  colon-delimited options, exact raw target options, ordering, duplicates, spans, merge provenance,
  sensitivity, `!reset`, and `!override` remain visible. Six provider-config rows remain planned
  without claiming runtime mount behavior or conflating service `tmpfs` with volume type `tmpfs`.

- Source-aware service `cap_add` support across authored documents, exact-scalar multi-file
  merging, effective projects, and deterministic generated output. Omission and explicit empty
  sequences remain distinct; exact duplicates are diagnosed without silent authored deletion,
  ordinary merge deduplicates with full provenance, and `!reset`/`!override` retain their defined
  behavior. Generation rejects empty, multiline, NUL-bearing, and exact-duplicate items without a
  capability whitelist or case normalization. `cap_add` remains independent from `cap_drop`, and
  six provider-config rows remain planned without making a runtime privilege claim.

- Source-aware service `cap_drop` support across authored documents, exact-scalar multi-file
  merging, effective projects, and deterministic generated output. Omission and explicit empty
  sequences remain distinct; exact duplicates are diagnosed without silent authored deletion,
  ordinary merge deduplicates with full provenance, and `!reset`/`!override` retain their defined
  behavior. Generation rejects empty, multiline, NUL-bearing, and exact-duplicate items without a
  capability whitelist or case normalization. Six provider-config rows remain planned and make no
  runtime capability claim.

- Source-aware service `hostname` support in authored documents and effective projects, retaining
  exact YAML string values, spans, deferred expressions, invalid literals, sensitivity, and
  complete scalar-replacement provenance without synthesizing a default.
- Non-exhaustive hostname classification and generated construction APIs with conservative ASCII
  RFC-1123 validation, deterministic quoted output, duplicate rejection, and typed parse-back
  validation. Six provider-config rows remain planned and make no runtime name-resolution claim.
- A distinct source-aware `Entrypoint` type at authored-document and merged-project layers,
  retaining null, scalar, list, empty forms, replacement provenance, and malformed-form diagnostics.
- `GeneratedEntrypoint` and `GeneratedService::set_entrypoint` for deterministic, redaction-aware,
  parse-back-validated string, list, and explicitly empty Compose output.
- Source-aware service `init` support in authored-document and merged-project layers, retaining
  omission, literal/deferred values, and complete replacement provenance without inventing a
  default.
- Deterministic `GeneratedService::set_init` boolean output with duplicate-singleton rejection and
  typed parse-back validation; an unset builder field remains omitted.
- Independent source-aware service `stop_signal` and lifecycle-specific `stop_grace_period` values
  in authored documents and effective merged projects, including raw spelling, deferred/invalid
  duration states, sensitivity, and complete replacement provenance.
- Deterministic generated lifecycle output with duplicate-singleton rejection, a raw-preserving
  duration policy based on documented Compose units, and typed parse-back validation.
- Raw-preserving service `pull_policy` values in authored documents and effective projects,
  separating documented policies, the `if_not_present` alias, custom intervals, deferred values,
  schema-only `refresh`, and invalid/provider-specific spellings with complete merge provenance.
- Non-exhaustive generated pull-policy construction with exact interval spelling, duplicate and
  `every_([0-9]+[wdhms])+` validation, sensitivity propagation, deterministic output, and typed
  parse-back checks. Schema-valid `every_0s` remains representable despite ambiguous prose
  semantics. `pull_refresh_after` remains preserved unmodeled evidence pending a separate native
  contract.
- Raw-preserving service `pids_limit` values in authored documents and effective projects,
  separating omission, unlimited `-1`, arbitrary-precision positive decimals, ambiguous zero,
  deferred interpolation, and unsupported scalar spellings with complete merge provenance.
- Non-exhaustive generated PID-limit construction that emits only unlimited or positive integral
  decimals, rejects zero/sign/fraction/exponent forms, and validates deterministic output by
  parsing it back. Six provider-config rows remain planned and make no runtime or cgroup claim.
- Raw-preserving service `shm_size` values across authored documents and effective projects,
  retaining exact scalar text and spans, YAML number/string provenance, documented lowercase
  units with unconstrained amount spelling, ambiguous zero, deferred expressions, distinct
  provider-dependent states, sensitivity, and complete replacement provenance without injecting a
  default or inspecting runtime shared memory.
- Non-exhaustive generated shared-memory construction that emits only quoted canonical positive
  ASCII-integer amounts with explicit documented lowercase units, rejects unsafe spellings, and
  validates exact amount/unit parse-back. Six provider-config rows remain planned and make no
  provider normalization, Podman-default, or runtime `/dev/shm` claim.

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
