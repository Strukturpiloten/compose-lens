# Processing model

A Compose file is not a fully resolved project. ComposeLens keeps every processing stage explicit so
callers can inspect authored intent, choose authorized inputs, and retain evidence for diagnostics or
conversion.

## Representations

| Representation     | What it answers                                                             |
| ------------------ | --------------------------------------------------------------------------- |
| `SyntaxDocument`   | What bytes and YAML structure were authored, and where?                     |
| `ComposeDocument`  | Which native Compose values and syntax forms are present in one document?   |
| `LoadedProject`    | Which ordered inputs and origins form this project?                         |
| `MergedProject`    | What is effective after Compose merge rules, and which sources contributed? |
| `ProfileSelection` | Which services are active for an explicit request?                          |
| `ProjectView`      | Which effective native Compose values can a consumer inspect?               |

Malformed or future input remains source-addressable wherever recovery is safe. A later view does not
replace the earlier representations; callers retain the level needed to explain a result.

## Operations

| Stage                  | Explicit input                                 | Result and boundary                                      |
| ---------------------- | ---------------------------------------------- | -------------------------------------------------------- |
| Parse syntax           | Source ID and text                             | Loss-aware YAML plus syntax diagnostics; no I/O          |
| Extract model          | Syntax document                                | Native values, unknown fields, and model diagnostics     |
| Load                   | Ordered `DocumentInput` values                 | Origins and first-document project base; no discovery    |
| Interpolate            | Environment provider                           | Per-file overlays and substitution provenance            |
| Merge                  | Loaded project and optional matching overlay   | Field-aware effective tree with merge provenance         |
| Select profiles        | Merged project and `ProfileRequest`            | Active/inactive service view without deletion            |
| Build project view     | Merged project and optional matching selection | Native consumer values with source provenance            |
| Resolve or default     | Project view and caller policy                 | Lexical paths, reference findings, or explicit decisions |
| Validate compatibility | Explicit implementation profile and versions   | Evidence-backed findings; no provider invocation         |
| Render or edit         | A specific representation and explicit options | Deterministic text plus diagnostics                      |

Operations reject a selection, overlay, or plan created for another project instead of assuming that
matching source IDs imply matching content.

## Loading and includes

`LoadedProject::load` accepts text and origin metadata already supplied by the caller. It preserves
input order, uses the first document's directory as the multi-file base, and aggregates recoverable
diagnostics. Applications decide how paths, standard input, editor buffers, archives, or remote
content become `DocumentInput` values.

Includes remain a sequence of opt-in operations:

1. `IncludeResolution::load` traverses through an `IncludeLoader`, the only authorization and I/O
   boundary.
2. `IncludeResolution::compose` imports non-conflicting child resources and reports local-wins
   conflicts without rereading anything.
3. `plan_project_directories` asks a caller-owned resolver to interpret explicit project-directory
   declarations.
4. `resolve_included_resource_paths` lexically resolves the supported selected resource paths using
   the matching plan.

Traversal retains partial graphs, cycles, identities, source spans, and diagnostics. It does not
invent environment precedence, project names, provider behavior, or filesystem semantics. ADRs
[0020](decisions/0020-caller-authorized-include-traversal.md) through
[0023](decisions/0023-include-config-secret-path-resolution.md) define these boundaries.

## Interpolation and merge

Interpolation receives an explicit environment provider. Parsing never consults it. Eligible values
are processed once per file before merge; keys stay unchanged. Results retain original expressions,
resolved values, substitution provenance, sensitivity, and diagnostics. A caller may deliberately use
an empty, map-backed, or application-specific provider.

Compose merge is not a generic YAML overlay. Mappings, sequences, shell-command fields, keyed
resources, syntax alternatives, `!reset`, and `!override` require field-aware rules. The merged tree
records whether values were authored, appended, replaced, recursively merged, reset, or overridden,
including every contributing span. Unknown data remains visible rather than being discarded because
the typed model does not yet interpret it.

The merge result preserves syntax that could affect behavior. It does not normalize provider values,
apply target policy, or resolve conflicts on behalf of BoxFerry. ADR
[0006](decisions/0006-provenance-preserving-compose-merge.md) owns the merge contract.

## Post-merge views

Profile selection is non-destructive. Services without profile restrictions remain active; restricted
services require an explicit matching request. Inactive services stay in the merged project for
diagnostics and reference analysis.

`build_project_view` exposes effective native Compose values directly from the merge result and
optional profile selection. Each value and collection item retains complete merge provenance and
sensitivity. Fields not promoted into a typed project value remain available as source-aware field
references.

Resolution also stays explicit:

- path resolution is lexical and uses caller-provided origins and home context;
- reference validation distinguishes missing and profile-inactive targets;
- default resolution asks a `DefaultProvider` and records each decision; and
- none of these operations opens a path or queries image or runtime metadata.

## Compatibility

Syntax acceptance and runtime support are separate claims. `validate_compatibility` uses an explicit
provider, optional runtime, and exact version range. Findings distinguish supported,
implementation-specific, deprecated, unsupported, and unknown evidence. Missing evidence remains
unknown, including under tolerant validation.

Compatibility validation is pure. Versioned provider and runtime observations are captured outside
the library and reviewed under [the conformance policy](../conformance/README.md).

## Rendering and diagnostics

Rendering never activates another processing stage. Canonical output renders a merged project,
generated output renders caller-constructed native values, and preservation editing patches exact
source spans. Their contracts are described in [Rendering](rendering.md).

Every stage returns structured diagnostics instead of printing. Stable codes are suitable for
automation; messages, labels, and notes explain the problem to people. Recovery-oriented results may
contain partial data, so callers should inspect diagnostics and use `is_valid()` when a stage must be
error-free before continuing.
