# Architecture decision records

## Status values

- `proposed` — under discussion
- `accepted` — current direction
- `superseded` — replaced by another ADR
- `rejected` — considered but not adopted

## Index

| ADR                                                        | Status   | Decision                                                                                  |
| ---------------------------------------------------------- | -------- | ----------------------------------------------------------------------------------------- |
| [0001](0001-project-boundaries-and-origin.md)              | accepted | Independent Compose library and from-scratch implementation                               |
| [0002](0002-loss-aware-yaml-syntax.md)                     | accepted | Private loss-aware YAML concrete syntax tree                                              |
| [0003](0003-preserve-compose-syntax-forms.md)              | accepted | Preserve field-specific short and long syntax variants                                    |
| [0004](0004-explicit-processing-overlays.md)               | accepted | Explicit providers and non-destructive processing overlays                                |
| [0005](0005-explicit-ordered-project-loading.md)           | accepted | Caller-supplied ordered loading with explicit origins                                     |
| [0006](0006-provenance-preserving-compose-merge.md)        | accepted | Parser-independent field-aware merge with provenance                                      |
| [0007](0007-explicit-post-merge-views.md)                  | accepted | Explicit profile, path, reference, and default views                                      |
| [0008](0008-versioned-provider-runtime-compatibility.md)   | accepted | Exact provider/runtime profiles with scoped evidence                                      |
| [0009](0009-deterministic-canonical-rendering.md)          | accepted | Explicit merged-project canonical YAML with a fixed v1 presentation                       |
| [0010](0010-atomic-span-based-preservation-edits.md)       | accepted | Atomic typed scalar replacements over exact source spans                                  |
| [0011](0011-presentation-only-render-formatting.md)        | accepted | Deterministic presentation options separated from semantic processing                     |
| [0012](0012-repository-conformance-harness.md)             | accepted | Exact-version repository conformance with reviewed retained evidence                      |
| [0013](0013-versioned-public-api-and-release-contract.md)  | accepted | Versioned pre-1.0 public API and auditable release contract                               |
| [0014](0014-issue-derived-native-model-expansion.md)       | accepted | Loss-aware issue-derived fields, container paths, and field-level build/deploy identities |
| [0015](0015-byte-preserving-yaml-backend-compatibility.md) | accepted | Same-length private parser compatibility with original-source scalar recovery             |
| [0016](0016-native-merged-project-view.md)                 | accepted | Native profile-selected project values with complete merge provenance                     |
| [0017](0017-parse-back-validated-compose-generation.md)    | accepted | Typed new-document generation with deterministic bytes and parse-back validation          |

Use the next four-digit number for new decisions. Include context, decision, consequences, and alternatives. Supersede accepted decisions with a new ADR rather than rewriting history.
