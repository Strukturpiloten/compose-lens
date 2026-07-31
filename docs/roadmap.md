# Roadmap

The roadmap is ordered by dependencies rather than dates.

## Phase 0: foundation

- Accept architecture and origin ADR.
- Select the YAML parsing strategy through an ADR and prototypes.
- Scaffold the crate, CI, lints, MSRV policy, and fixture metadata format.
- Define source spans, diagnostics, and public API stability policy.

## Phase 1: syntax and source model

- Parse YAML into a source-aware document.
- Preserve unknown mappings and scalar spelling.
- Implement structured syntax diagnostics.
- Establish round-trip and malformed-input test suites.

## Phase 2: typed Compose document

- Implement top-level project, services, networks, volumes, configs, and secrets.
- Add short/long syntax value types.
- Preserve extensions and unknown fields.
- Support tolerant image references and source provenance.

## Phase 3: project processing

- Implement explicit environment providers and interpolation.
- Implement multi-file loading and merging.
- Implement profile selection, path origins, references, and configurable defaults.
- Add implementation-specific compatibility profiles.

## Phase 4: rendering and editing

- Implement deterministic canonical rendering.
- Implement preservation-oriented edits where the syntax layer permits them.
- Define formatting options without coupling semantics to presentation.

## Phase 5: ecosystem hardening

- Expand Docker Compose and Podman Compose version matrices.
- Grow the licensed real-world corpus.
- Stabilize the public crate API.
- Publish releases and compatibility documentation.
