# Architecture decision records

## Status values

- `proposed` — under discussion
- `accepted` — current direction
- `superseded` — replaced by another ADR
- `rejected` — considered but not adopted

## Index

| ADR | Status | Decision |
| --- | --- | --- |
| [0001](0001-project-boundaries-and-origin.md) | accepted | Independent Compose library and from-scratch implementation |
| [0002](0002-loss-aware-yaml-syntax.md) | accepted | Private loss-aware YAML concrete syntax tree |
| [0003](0003-preserve-compose-syntax-forms.md) | accepted | Preserve field-specific short and long syntax variants |
| [0004](0004-explicit-processing-overlays.md) | accepted | Explicit providers and non-destructive processing overlays |
| [0005](0005-explicit-ordered-project-loading.md) | accepted | Caller-supplied ordered loading with explicit origins |
| [0006](0006-provenance-preserving-compose-merge.md) | accepted | Parser-independent field-aware merge with provenance |
| [0007](0007-explicit-post-merge-views.md) | accepted | Explicit profile, path, reference, and default views |
| [0008](0008-versioned-provider-runtime-compatibility.md) | accepted | Exact provider/runtime profiles with scoped evidence |
| [0009](0009-deterministic-canonical-rendering.md) | accepted | Explicit merged-project canonical YAML with a fixed v1 presentation |
| [0010](0010-atomic-span-based-preservation-edits.md) | accepted | Atomic typed scalar replacements over exact source spans |
| [0011](0011-presentation-only-render-formatting.md) | accepted | Deterministic presentation options separated from semantic processing |

Use the next four-digit number for new decisions. Include context, decision, consequences, and alternatives. Supersede accepted decisions with a new ADR rather than rewriting history.
