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

## Phase 4: rendering and editing — open

- [ ] Implement deterministic canonical rendering.
- [ ] Implement preservation-oriented edits where the syntax layer permits them.
- [ ] Define formatting options without coupling semantics to presentation.

## Phase 5: ecosystem hardening — open

- [ ] Expand Docker Compose and Podman Compose version matrices.
- [ ] Grow the licensed real-world corpus.
- [ ] Stabilize the public crate API.
- [ ] Publish releases and compatibility documentation.
