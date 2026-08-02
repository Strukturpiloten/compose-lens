# Roadmap

The roadmap is ordered by dependencies rather than dates.

Cross-repository delivery uses the stable task numbers in the [implementation plan](implementation-plan.md). This roadmap remains the detailed internal phase order for ComposeLens.

The parser-selection, source/diagnostic kernel, and initial preservation milestones are delivered by T2. Typed Compose behavior begins with T5.

## Status key

- [x] Completed and validated
- [ ] Open

## Phase 0: foundation — completed

- [x] Accept architecture and origin ADR.
- [x] Select the YAML parsing strategy through an ADR and prototypes.
- [x] Scaffold the crate, CI, lints, MSRV policy, and fixture metadata format.
- [x] Define source spans, diagnostics, and public API stability policy.

## Phase 1: syntax and source model — completed

- [x] Parse YAML into a source-aware document.
- [x] Preserve unknown mappings and scalar spelling.
- [x] Implement structured syntax diagnostics.
- [x] Establish round-trip and malformed-input test suites.

## Phase 2: typed Compose document — completed

Phase 2 is complete for the first BoxFerry conversion boundary. Fields outside that documented
boundary remain loss-aware unknown-field references; typing the entire Compose Specification is
not a Phase 2 exit criterion. See the [Phase 2 typed model](typed-model.md).

- [x] Establish source-aware project, service, network, volume, config, and secret collections.
- [x] Complete the typed service and top-level resource fields required by the first conversion boundary.
- [x] Define the representation-fidelity policy for non-interchangeable syntax forms.
- [x] Implement distinct short and long service-volume types, including bind and SELinux options.
- [x] Add field-specific syntax variants for commands, environment, ports, networks, configs, and secrets.
- [x] Preserve extensions and unknown fields for the implemented typed subset.
- [x] Provide reusable typed-value source provenance.
- [x] Support tolerant image references, including combined tags and digests.
- [x] Preserve deferred boolean expressions, null values, empty values, scalar kinds, extensions, and unknown fields.
- [x] Validate valid and recoverable-invalid Phase 2 forms with authored, source-spanned fixtures.

## Phase 3: project processing — completed

- [x] Implement explicit environment providers and the source-aware interpolation kernel.
- [x] Apply interpolation to eligible YAML values as a non-destructive per-file overlay.
- [x] Load ordered caller-supplied documents with unique source IDs, explicit origins, recoverable diagnostics, and first-file project-directory semantics.
- [x] Merge loaded documents with provenance and Compose field-specific rules.
- [x] Select active services from an explicit profile request without mutating the merged project.
- [x] Classify and lexically resolve supported host paths from retained project origins and explicit home context.
- [x] Validate selected-service references to services, networks, volumes, configs, and secrets.
- [x] Request documented semantic defaults from a caller-owned policy without modifying source values.
- [x] Add exact-version specification, Docker Compose, `podman-compose`, and tolerant compatibility profiles.
- [x] Separate the Compose provider from its optional Docker or Podman backend runtime.
- [x] Add published versioned evidence for syntax-form asymmetries, beginning with long bind SELinux behavior.

## Phase 4: rendering and editing — completed

- [x] Implement deterministic canonical rendering.
- [x] Implement preservation-oriented edits where the syntax layer permits them.
- [x] Define formatting options without coupling semantics to presentation.

## Phase 5: ecosystem hardening — completed with 0.1.0

- [x] Expand Docker Compose and Podman Compose version matrices.
  - [x] Define an exact-version provider-config matrix with explicit planned and observed states.
  - [x] Add an authored implementation-sensitive fixture and a pure matrix policy test.
  - [x] Add an ignored, environment-cleared result-capture runner and evidence-review contract.
  - [x] Execute and review the six initial provider targets across all eight probes from trusted artifacts.
  - [x] Add isolated Docker Engine and Podman runtime-effect matrices.
  - [x] Promote reviewed feature outcomes into narrowly scoped compatibility rules.
- [x] Grow the licensed real-world corpus.
  - [x] Define admission, licensing, generation, sanitization, and update policy.
  - [x] Add the generated `Strukturpiloten/typo3-container` PostgreSQL fixture with source and processing regressions.
  - [x] Add independently licensed projects that contribute distinct Compose behavior.
- [x] Stabilize and regression-test the supported pre-1.0 public crate API.
- [x] Prepare and validate the 0.1.0 release package, release notes, and compatibility documentation.

## Maintainer-controlled release operations — completed

These credentialed external actions are deliberately not represented as implementation work:

- [x] Publish ComposeLens 0.1.0 to crates.io from an approved clean release commit.
- [x] Create and push the `v0.1.0` tag and corresponding GitHub release.

The exact operational checklist is in the [release process](releasing.md).

## Post-0.1 issue-derived backlog

The [Podlet and `compose_spec_rs` regression review](research/podlet-compose-spec-rs-regressions-2026-08-01.md)
confirms existing coverage and records additional native Compose candidates without reopening the
completed first-conversion boundary.

- [x] Type `extra_hosts`, preserving short/long forms, IPv6 spelling, and implementation tokens.
- [x] Type raw-preserving user/group values and unlimited `ulimits`.
- [x] Complete native dependency-condition and health-check types and validation.
- [x] Separate host-platform paths from container-platform anonymous-volume targets.
- [x] Add typed Podman/Compose extensions only with a demonstrated consumer and compatibility evidence.
- [x] Evaluate build and deploy subfields individually instead of treating either section as one support flag.

Completed by [ADR 0014](decisions/0014-issue-derived-native-model-expansion.md), the authored
`post-01-issue-backlog` and `post-01-invalid` fixtures, the licensed TYPO3 consumer regression,
and typed, post-merge, compatibility, and platform-path tests. Provider pass-through for Podman's
`userns_mode` values and `host-gateway` deliberately remains `unknown` until an exact conformance
record exists; that evidence gap does not return these values to the untyped model.
