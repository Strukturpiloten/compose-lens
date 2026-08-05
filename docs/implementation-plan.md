# Cross-repository implementation plan

This plan gives BoxFerry, ComposeLens, and QuadletLens one stable task numbering scheme. Repository roadmaps describe internal phases; this document describes delivery order across repositories.

Last synchronized: 2026-08-05.

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
| T3 | QuadletLens | completed | Ordered Quadlet syntax and rendering kernel |
| T4 | BoxFerry | completed | Independent neutral model and conversion engine |
| T5 | All repositories | in progress | Minimum native typed subsets for the first conversion |
| T6 | BoxFerry, integrating both Lens libraries | in progress | First Compose-to-Quadlet vertical slice |
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

Status: completed. QuadletLens owns this task.

QuadletLens now provides ordered loss-aware syntax, structured recovery, exact preservation,
conservative canonical rendering, lexical path/specifier classification, a strict versioned
capability schema, and Podman 5.4.0 as its support floor. Its digest-pinned generator harness
verifies the first-conversion subset on all 20 patch releases through current 6.0.2, using official
images through 5.8.2 and exact source builds thereafter. Untested capabilities remain explicit
fail-closed evidence gaps.

## T4: BoxFerry independent conversion core

Status: completed. BoxFerry owns this task.

Implement neutral application, service, volume, network, port, environment, and tolerant image-reference models; provenance and redacted diagnostics; exact, approximate, unsupported, and invalid outcomes; target version ranges; adapter contracts; and an in-memory adapter. This task does not depend on unfinished Lens APIs.

BoxFerry has implemented the public library facade, neutral application graph, provenance,
protected values, redacted structured diagnostics, version-bounded target profiles, validated
fidelity outcomes, explicit loss authorization, adapter contracts, and in-memory adapter. Its
stable and Rust 1.85.0 tests exercise the same orchestration available to external Rust projects.
The component crates remain unpublished until their release contract is finalized.

## T5: Minimum native typed subsets

Status: in progress. Each repository owns its native types; BoxFerry owns mappings.

- ComposeLens (completed): services, images, commands, environment, service labels, extra hosts, ports, volumes,
  networks, profiles, configs, and secrets.
- QuadletLens (completed): `.container`, `.pod`, `.volume`, `.network`, required generic systemd
  sections, repeatable container/pod host mappings, repeatable container labels, and exact
  document-set relationships.
- BoxFerry (in progress): Compose-to-neutral and first neutral-to-Quadlet mappings, path policy,
  explicit pod grouping, and end-to-end host mappings are implemented; broader value encoders
  remain.

Delivered ComposeLens slice: source-aware services and typed top-level resource definitions; tolerant image references; distinct command, environment, port, volume, service-network, config-grant, secret-grant, and label forms; deferred scalar expressions; field provenance; and retained extensions and unknown fields. [ADR 0003](decisions/0003-preserve-compose-syntax-forms.md) prohibits implicit form normalization. The [Phase 2 typed-model document](typed-model.md) defines the completed boundary and evidence.

Delivered QuadletLens slice: ordered source-aware `.container`, `.pod`, `.network`, and `.volume`
documents; preserved generic systemd and unknown entries; native key enums;
conservative path and unit-reference forms; separate syntax/model diagnostics; and exact
document-set dependency resolution. BoxFerry consumes ComposeLens 0.1.6 and QuadletLens 0.1.6 from
crates.io through independent adapter crates. It maps images, commands, environment, extra hosts,
single ports, named volumes, bind mounts, networks, explicit profiles, provenance, and short/long
SELinux relabel intent into the neutral model, then generates validated separate-container or
explicitly grouped-pod Quadlet output. Source omissions are structured outcomes governed by
`LossPolicy`, not warning-only side effects.

ComposeLens now provides `build_project_view`, a native profile-selected consumer boundary over the
merged project. Its `ProjectValue<T>` and collection items retain every contributing source span,
and BoxFerry now consumes that released boundary without reparsing canonical output. Its multi-file
adapter regression retains all contributing origins and the unquoted comma-containing volume form.

ComposeLens 0.1.6 and QuadletLens 0.1.6 are published on crates.io with documented pre-1.0
compatibility contracts. BoxFerry consumes both through compatible crates.io requirements and
commits its application lockfile. Commit-pinned Git dependencies remain an emergency-only fallback.

The BoxFerry adapter fixture exposed a ComposeLens 0.1 YAML-backend defect: an unquoted short
volume scalar with comma-separated options could truncate the document without a syntax diagnostic.
The published 0.1.1 release accepts the complete valid scalar through a byte-preserving private
parser adapter, restores authored scalar values from the original source, and keeps
`compose.yaml.unparsed-input` as a general omission fail-safe.

Explicit host mappings are complete across ComposeLens's merged `extra_hosts` view, BoxFerry's
neutral model, and QuadletLens's capability-evidenced `AddHost` keys. Separate containers retain
service scope. Single-pod grouping requires identical ordered mappings and moves them to pod scope;
conflicts reject the explicit grouping request.

### Coverage guardrail and completed health/dependency slices

The three repositories now document syntax preservation, native typing, effective project views,
neutral representation, target capabilities, and end-to-end conversion as separate coverage
stages. The authoritative cross-format matrix lives in the
[BoxFerry repository](https://github.com/Strukturpiloten/boxferry), with native details in the
ComposeLens and QuadletLens coverage documents. A field is not complete
merely because one Lens recognizes it.

ComposeLens 0.1.3 and QuadletLens 0.1.3 are published and consumed by BoxFerry. The neutral health
model and adapters preserve regular health-check intent and report Compose `start_interval` as an
unsupported non-equivalent target behavior.

ComposeLens 0.1.4 and QuadletLens 0.1.4 are published and consumed by BoxFerry. Ordered neutral
dependency edges retain condition, requirement, restart, and merge provenance. Required and
optional startup edges map to `Requires`/`Wants` plus `After`; healthy edges select
`Notify=healthy` only for explicit encodable target health commands. Unsupported restart and
completion semantics remain policy-controlled losses, while missing required services and cycles
are invalid. Golden tests cover separate containers and explicitly grouped pods.

ComposeLens and QuadletLens 0.1.6 are published and consumed by BoxFerry. They cover execution
identity and container context plus effective config/secret grants, repeatable container `Secret`,
and pod `UserNS`, with Quadlet evidence across Podman 5.4.0 through 6.0.2.

## T6: First end-to-end milestone

Status: in progress. BoxFerry coordinates this task. Compose import, the first Quadlet export,
their combined compatibility report, path policies, pod grouping, explicit host mappings, health
checks, and dependencies are
implemented. Broader value encoders and the TYPO3 showcase remain.

Deliver tested Compose-to-Quadlet conversion for images, commands, health checks, dependencies, environment, extra hosts,
ports, named volumes, bind mounts, networks, and explicit Compose profile selection. Every
conversion emits compatibility and manual-action reports. After synthetic scenarios are stable,
use `Strukturpiloten/typo3-container` as the first public real-world showcase and regression corpus.

ComposeLens has established the licensed, sanitized `Strukturpiloten/typo3-container` regression
fixture. The BoxFerry conversion showcase remains part of T6 and must consume that fixture only
after the adapter boundary is ready.

## T7: Expanded testing tiers

Status: in progress. ComposeLens's Phase 5 repository work is completed. QuadletLens has an exact
Podman 5.4-to-current generator matrix, and BoxFerry has its first provenance-reviewed Compose
adapter fixture; broader BoxFerry tiers remain.

- Per pull request: unit, integration, golden, round-trip, and property tests.
- Scheduled: Docker Compose, Podman Compose, and real Quadlet generator conformance.
- Release validation: supported Podman matrices, rootless/rootful contexts, real-world projects, and eventually disposable Kubernetes clusters.

Each harness becomes required only after its command, isolation model, version source, fixture provenance, and failure policy are documented.

ComposeLens delivery evidence includes 48 reviewed exact provider-config records, a 36-entry
fail-closed runtime-effect matrix, two independently licensed real-world fixtures, narrowly scoped
compatibility rules, a consumer-facing 0.1.x API contract, and the published ComposeLens 0.1.6
crate and GitHub release. The runtime entries remain planned until suitable isolated SELinux hosts
execute them. Future crates.io releases use trusted publishing with short-lived OIDC credentials.

ComposeLens 0.1.7 is published for runtime-to-Compose output. It adds pure,
deterministic, parse-back-validated document construction for the current BoxFerry observation
subset. Field-specific syntax selection retains short SELinux bind behavior, emits SCTP through
the platform-protocol-capable short port form, and keeps optional exact platform names distinct
from network/volume lifecycle ownership.

ComposeLens 0.1.8 and 0.1.9 are published with service-label support and the real-world YAML
backend corrections. ComposeLens 0.1.10 is the additive source-side candidate for explicit
`container_name` values across the document, merged-project, and generated-document boundaries.
The coordinated QuadletLens 0.1.9 candidate types and generator-verifies `ContainerName=`. BoxFerry
will consume both published releases before promoting the field into its neutral model; sibling
path dependencies are not used.
