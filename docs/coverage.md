# Native Compose coverage

This document distinguishes source preservation from typed consumer coverage. It was audited
against the current official
[Compose service reference](https://docs.docker.com/reference/compose-file/services/) on
2026-08-06. The exact current untyped-key inventory and promotion order live in the
[roadmap](roadmap.md).

## Coverage layers

| Layer | Contract |
| --- | --- |
| Syntax | Valid and recoverable YAML remains source-addressable; unknown fields and `x-` extensions are retained. |
| Document model | One Compose document exposes a source-aware native type without interpolation, merge, or normalization. |
| Project view | An effective multi-file project exposes the typed value with merge provenance after optional explicit profile selection. |

A field is not ready for a converter merely because the document model types it. Multi-file
consumers need the project-view layer so they do not render and reparse a merged document or lose
provenance.

## Current service boundary

| Coverage | Service fields |
| --- | --- |
| Document model and project view | `container_name`, `image`, `entrypoint`, `command`, `init`, `environment`, `env_file`, `labels`, `extra_hosts`, `user`, `userns_mode`, `group_add`, `working_dir`, `read_only`, `restart`, `healthcheck`, `depends_on`, `ports`, `volumes`, `networks`, `profiles`, `configs`, `secrets` |
| Document model only | `build`, `ulimits`, `deploy` |
| Preserved, not typed | 67 exact current service keys; see [Exact service gaps](roadmap.md#exact-service-gaps). |

The preserved row follows the current Docker documentation grouping. Provider-specific additions
remain preserved even when they are not part of the compose-spec repository.

## Current top-level boundary

`name`, `services`, `networks`, `volumes`, `configs`, and `secrets` have both document-model and
project-view support. `version`, `include`, and `models` remain syntax-preserved only. Their exact
nested gaps and implemented definition fields are listed in the [roadmap](roadmap.md) and
[Typed Compose model](typed-model.md). Other nested values remain source-addressable and appear as
typed field references where the current boundary supports them.

## Next promotion

Execution identity now exposes effective `user`, `userns_mode`, ordered `group_add`, `working_dir`,
and `read_only` values with complete merge provenance. Values remain raw and source-aware:
ComposeLens classifies user components and known namespace modes but never resolves operating-system
accounts, supplementary groups, paths, or runtime namespace state.

Service config and secret grants now expose effective short and long forms. Long-form `source`,
`target`, `uid`, `gid`, and `mode` values retain their own provenance so unique-by-target
multi-file merging remains visible to consumers.

Service labels now expose both source-aware authored forms and a normalized-by-key effective view.
Each effective entry keeps mapping/list syntax and complete merge provenance. Key-only list labels
remain distinguishable while exposing their documented empty-string value. The generated-document
API emits ordered quoted mappings and rejects duplicate names.

Explicit `container_name` values now travel through the document model, effective project view,
and generated-document boundary. Generation enforces the documented portable Compose name grammar;
authored parsing retains the scalar and leaves provider/runtime acceptance to compatibility policy.

Service-level `restart` now travels through the same three boundaries. Authored input retains
retry-count spelling and interpolation; generated input uses a typed policy that cannot emit an
unknown value. Dependency-update `restart` and deploy restart policy remain separate concepts.

Service `env_file` now travels through the document model and effective project view. Scalar and
ordered-list short syntax remain distinct from long entries; long `path`, `required`, `format:
raw`, extensions, unknown fields, nested provenance, and deferred interpolation are retained.
ComposeLens performs no file discovery, existence check, or environment-file parsing. Generated
Compose output retains ordered short and long entries, explicit `required`, and `format: raw`,
then validates its own bytes through the native parser.

Entrypoint now has distinct source-aware document/project types and generated string, list, and
explicitly empty forms. `null` continues to mean “use the image entrypoint,” while empty scalar or
list forms explicitly clear it. Multi-file processing uses Compose's replacement rule and retains
all contributing spans. Service `init` also crosses all three boundaries as an authored/project
boolean that can retain deferred interpolation and a generated explicit boolean. Remaining
lifecycle controls and resource limits are the next high-value promotion groups.

## Promotion checklist

A service field moves into the project view only with:

1. correct Compose merge behavior;
2. source provenance for the field and meaningful nested values;
3. malformed-form diagnostics and partial recovery;
4. multi-file tests; and
5. an additive public API that keeps authored forms distinct where semantics can differ.
