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

## Phase 2: typed Compose document — open

- [ ] Implement top-level project, services, networks, volumes, configs, and secrets.
- [ ] Add short/long syntax value types.
- [ ] Preserve extensions and unknown fields.
- [ ] Support tolerant image references and source provenance.

## Phase 3: project processing — open

- [ ] Implement explicit environment providers and interpolation.
- [ ] Implement multi-file loading and merging.
- [ ] Implement profile selection, path origins, references, and configurable defaults.
- [ ] Add implementation-specific compatibility profiles.

## Phase 4: rendering and editing — open

- [ ] Implement deterministic canonical rendering.
- [ ] Implement preservation-oriented edits where the syntax layer permits them.
- [ ] Define formatting options without coupling semantics to presentation.

## Phase 5: ecosystem hardening — open

- [ ] Expand Docker Compose and Podman Compose version matrices.
- [ ] Grow the licensed real-world corpus.
- [ ] Stabilize the public crate API.
- [ ] Publish releases and compatibility documentation.
