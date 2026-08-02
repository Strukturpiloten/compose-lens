# Cross-repository implementation plan

This plan gives BoxFerry, ComposeLens, and QuadletLens one stable task numbering scheme. Repository roadmaps describe internal phases; this document describes delivery order across repositories.

Last synchronized: 2026-07-31.

## Status convention

- `planned` — scoped but not started
- `in progress` — implementation is currently active
- `completed` — exit criteria are met and validation is documented
- `blocked` — progress requires a named external decision or capability

The repository that owns a task is authoritative for its detailed status. Update the summary copies in the other two repositories whenever a task changes state.

## Program status

| Task | Owner | Status | Deliverable |
| --- | --- | --- | --- |
| T1 | All repositories | completed | Executable testing and fixture foundations |
| T2 | ComposeLens | completed | Loss-aware YAML syntax and diagnostic kernel |
| T3 | QuadletLens | planned | Ordered Quadlet syntax and rendering kernel |
| T4 | BoxFerry | planned | Independent neutral model and conversion engine |
| T5 | All repositories | in progress | Minimum native typed subsets for the first conversion |
| T6 | BoxFerry, integrating both Lens libraries | planned | First Compose-to-Quadlet vertical slice |
| T7 | All repositories | in progress | Expanded conformance, runtime, and release testing tiers |

## T1: Testing foundations

Status: completed.

The repositories have Cargo-discovered policy tests, versioned fixture manifests, provenance and secret-review rules, immutable GitHub Action checks, stable/MSRV CI execution, and documented suite ownership. Product suites are created only with meaningful behavior.

## T2: ComposeLens YAML syntax kernel

Status: completed. ComposeLens owns this task.

Work:

1. Evaluate candidate YAML representations against comments, anchors, aliases, duplicate keys, scalar spelling, unknown fields, source spans, malformed input, recovery, MSRV, and licensing.
2. Record the parser and concrete-syntax-tree decision in an ADR.
3. Implement source identifiers, byte spans, line/column lookup, diagnostic codes, severities, labels, and parse results.
4. Implement the initial loss-aware YAML syntax document without interpolation or normalization.
5. Add authored malformed-input and exact preservation fixtures.
6. Prove parse/render/parse stability for the supported syntax corpus.

Exit criteria:

- The selected representation and rejected alternatives are documented with exact evaluated versions.
- Valid source text, including comments, duplicate keys, anchors, aliases, and scalar spelling, renders without byte changes.
- Malformed YAML returns structured, source-spanned diagnostics without panicking and retains a renderable syntax document.
- Public source and diagnostic primitives are documented and compile on Rust 1.85.0.

Delivery evidence: [ADR 0002](decisions/0002-loss-aware-yaml-syntax.md), the [parser evaluation](research/yaml-representation.md), `src/source/`, `src/diagnostic/`, `src/syntax/`, and the authored `syntax` and `roundtrip` fixture suites.

## T3: QuadletLens ordered syntax kernel

Status: planned. QuadletLens owns this task.

Implement ordered sections and entries, repeated keys, comments, continuations, unknown Quadlet keys, generic systemd sections, systemd specifiers such as `%h`, source locations, structured diagnostics, and deterministic rendering. Add malformed-input and parse/render fixtures. Then define the capability schema and Podman 5.4 evidence baseline with minimum/maximum versions, fallbacks, and known-bug ranges.

## T4: BoxFerry independent conversion core

Status: planned. BoxFerry owns this task.

Implement neutral application, service, volume, network, port, environment, and tolerant image-reference models; provenance and redacted diagnostics; exact, approximate, unsupported, and invalid outcomes; target version ranges; adapter contracts; and an in-memory adapter. This task does not depend on unfinished Lens APIs.

## T5: Minimum native typed subsets

Status: in progress. Each repository owns its native types; BoxFerry owns mappings.

- ComposeLens (completed): services, images, commands, environment, ports, volumes, networks, profiles, configs, and secrets.
- QuadletLens: `.container`, `.volume`, `.network`, and required generic systemd sections.
- BoxFerry: mappings, path-policy differences, and Podman 5.4 fallback decisions.

Delivered ComposeLens slice: source-aware services and typed top-level resource definitions; tolerant image references; distinct command, environment, port, volume, service-network, config-grant, secret-grant, and label forms; deferred scalar expressions; field provenance; and retained extensions and unknown fields. [ADR 0003](decisions/0003-preserve-compose-syntax-forms.md) prohibits implicit form normalization. The [Phase 2 typed-model document](typed-model.md) defines the completed boundary and evidence.

Before integration, document dependency and release mechanics. Prefer early pre-1.0 Lens releases; use commit-pinned Git dependencies only as a temporary fallback.

## T6: First end-to-end milestone

Status: planned. BoxFerry coordinates this task.

Deliver tested Compose-to-Quadlet conversion for images, commands, environment, ports, named volumes, bind mounts, networks, and explicit Compose profile selection. Every conversion emits compatibility and manual-action reports. After synthetic scenarios are stable, use `Strukturpiloten/typo3-container` as the first public real-world showcase and regression corpus.

ComposeLens has established the licensed, sanitized `Strukturpiloten/typo3-container` regression
fixture. The BoxFerry conversion showcase remains part of T6 and must consume that fixture only
after the adapter boundary is ready.

## T7: Expanded testing tiers

Status: in progress. ComposeLens's Phase 5 repository work is completed; QuadletLens and BoxFerry
retain their own T7 work.

- Per pull request: unit, integration, golden, round-trip, and property tests.
- Scheduled: Docker Compose, Podman Compose, and real Quadlet generator conformance.
- Release validation: supported Podman matrices, rootless/rootful contexts, real-world projects, and eventually disposable Kubernetes clusters.

Each harness becomes required only after its command, isolation model, version source, fixture provenance, and failure policy are documented.

ComposeLens delivery evidence includes 48 reviewed exact provider-config records, a 36-entry
fail-closed runtime-effect matrix, two independently licensed real-world fixtures, narrowly scoped
compatibility rules, a consumer-facing 0.1.x API contract, and a CI-verified release package. The
runtime entries remain planned until suitable isolated SELinux hosts execute them. Credentialed
crates.io and GitHub publication remains a maintainer-controlled release operation.
