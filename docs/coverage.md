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
| Document model and project view | `hostname`, `container_name`, `image`, `entrypoint`, `command`, `init`, `environment`, `env_file`, `labels`, `annotations`, `logging`, `extra_hosts`, `user`, `userns_mode`, `group_add`, `cap_add`, `cap_drop`, `devices`, `dns`, `dns_opt`, `dns_search`, `expose`, `security_opt`, `working_dir`, `read_only`, `pids_limit`, `shm_size`, `mem_limit`, `tmpfs`, `sysctls`, `ulimits`, `pull_policy`, `restart`, `stop_signal`, `stop_grace_period`, `healthcheck`, `depends_on`, `ports`, `volumes`, `networks`, `profiles`, `configs`, `secrets` |
| Document model only | `build`, `deploy` |
| Preserved, not typed | 49 exact current service keys; see [Exact service gaps](roadmap.md#exact-service-gaps). |

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

Service annotations cover authored mapping/list syntax, keyed effective merging, and safe generated
maps. Key-only ambiguity remains diagnosed, and provider/runtime behavior is not claimed.

Service `logging` covers authored, effective-project, and generated boundaries. Its driver remains
an uninterpreted string; ordered options retain exact string/number/null kind, value-only
interpolation, nested provenance, extensions, unknowns, malformed recovery, and explicit empty
maps. Generic recursive merge, reset, and override remain visible. Generation validates only safe
YAML construction and applies no logging defaults, option normalization, or provider semantics.

Explicit `container_name` values now travel through the document model, effective project view,
and generated-document boundary. Generation enforces the documented portable Compose name grammar;
authored parsing retains the scalar and leaves provider/runtime acceptance to compatibility policy.

Service `hostname` now travels independently through the same three boundaries. Authored YAML must
use a string scalar; exact values, spans, deferred dollar-bearing expressions, invalid literals,
interpolation sensitivity, and complete scalar-replacement provenance are retained. Resolved
literals use conservative ASCII RFC-1123 validation. Generated output accepts only a valid resolved
literal and never derives it from `container_name` or a service key. Provider-config rows remain
planned, and no runtime DNS, hosts-file, UTS, or name-resolution behavior is claimed.

Service `cap_add` and `cap_drop` now independently reach the same three boundaries. Each keeps
omission distinct from explicit empty state, retains exact strings and case variants, diagnoses
authored schema duplicates without deleting them, and applies exact-scalar uniqueness only during
ordinary multi-file merge. Generated output rejects exact duplicates and unsafe empty or multiline
items while applying no capability whitelist or cross-field reconciliation. Provider-config rows
remain planned; no runtime, privilege, namespace, seccomp, SELinux, or cross-format capability
behavior is claimed.

Service `devices` now reaches the authored, effective-project, and generated-document boundaries.
Omission differs from an explicit empty sequence; ordered mixed short and long forms, exact
duplicates, raw path/CDI/deferred/opaque strings, long `source`/`target`/`permissions`, extensions,
unknowns, spans, sensitivity, and nested provenance remain visible. Existing target-keyed
replacement, reset, and override behavior is retained and its Compose-prose/Compose-Go discrepancy
is documented. Generated output validates only safe resolved strings and parse-back fidelity. Six
provider-config rows remain planned; no host-device, CDI, permissions, GPU, privilege, runtime
access, or cross-format behavior is claimed.

Service `dns`, `dns_opt`, and `dns_search` cover authored, effective-project, and generated
boundaries while preserving their respective scalar/list or sequence merge rules. Raw values,
duplicates, provenance, and reset/override state remain observable without resolver interpretation.

Generated service-network attachments now close the existing native-model gap for optional
per-network `ipv4_address` and `ipv6_address`. Long-form output retains aliases, omission, raw
spelling, sensitivity, and named-network scope without validating IP grammar or IPAM pools.

Service `expose` covers ordered string/number scalars with kind-aware uniqueness and safe generated
documented forms. Unsupported or malformed forms remain raw diagnostics.

Service `security_opt` covers raw append merging and generated output. Exact AppArmor,
no-new-privileges, seccomp, SELinux-label, Mask, and Unmask shapes are exposed as independent
diagnostic candidates; near misses and conflicts are not selected. Provider, filesystem, security
enforcement, and cross-format behavior remain outside this coverage.

Service-level `restart` now travels through the same three boundaries. Authored input retains
retry-count spelling and interpolation; generated input uses a typed policy that cannot emit an
unknown value. Dependency-update `restart` and deploy restart policy remain separate concepts.

Service `pids_limit` now travels through the same three boundaries independently from deploy
resource limits. Authored and merged input retains exact unlimited, arbitrary-precision positive
decimal, ambiguous zero, deferred, and unsupported scalar states without fixed-width parsing.
Wrong YAML types diagnose without deleting their service. Generated output emits only `-1` or a
positive integral ASCII decimal and rejects zero, signs, fractions, exponents, expressions, and
arbitrary strings. Provider-config rows remain planned; no runtime or cgroup behavior is claimed.

Service `shm_size` now travels through the same three boundaries independently from
`build.shm_size`, IPC or pod grouping, resource limits, and runtime inspection. Authored values
retain exact scalar text, YAML number/string provenance, documented lowercase units plus
unconstrained amount spelling, ambiguous zero, deferred expressions, and distinct
provider-dependent number/string states. Generated output accepts only a quoted canonical positive
ASCII-integer amount with an explicit documented lowercase unit. Omission remains omitted; no
Podman 64 MiB default or provider normalization is injected. Provider-config rows remain planned,
and no runtime allocation or `/dev/shm` claim is made.

Service `mem_limit` now travels through all three boundaries independently from reservation, swap,
deploy resource memory, and `shm_size`. Exact scalar text/kind, documented lowercase units, zero,
deferred, schema-number, and provider-dependent string forms retain source and merge evidence.
Generated output requires a quoted positive arbitrary-precision ASCII decimal plus a documented
unit. Only `b` values are candidates for exact cross-format handling. Six provider rows remain
planned and make no host, cgroup, runtime, default, normalization, or enforcement claim.

Service-level `tmpfs` now travels through the same three boundaries independently from long-syntax
volume type `tmpfs`. Omission, scalar/list form, explicit empty lists, colon-delimited
`<path>[:<options>]`, ordering, exact duplicates, source spans, sensitivity, and complete merge
provenance remain visible. `mode`, `uid`, and `gid` assignments are classified as documented;
other well-shaped raw options remain provider-dependent. Ordinary list merging appends without
deduplication, and generated output preserves duplicates and raw options. Provider-config rows
remain planned; no runtime mount, default-flag, rootless, pod, or cross-format equivalence is claimed.

Service `sysctls` now travels through the same three boundaries with mapping/list form intact.
Omission, explicit empty collections, ordered mapping keys, exact scalar kinds and spelling,
ordered list items, duplicate evidence, spans, interpolation sensitivity, and complete merge
provenance remain visible. Generic merge combines mappings by exact key and appends lists without
silent deduplication. Generated mappings and lists emit resolved quoted strings only. Planned
provider rows ask config-retention questions only; no namespace, kernel, privilege, runtime
application, or cross-format equivalence claim is made.

Service `ulimits` now reaches the authored, effective-project, and generated-document boundaries.
The effective ordered mapping retains lowercase outer keys, single versus soft/hard range form,
authored and interpolated scalar spelling, YAML number/string kind, nested member provenance,
sensitivity, omission, explicit empty/reset mappings, recursive inner merge, scalar/range
replacement, and whole-field override. Malformed names, shapes, values, or missing range members
are diagnosed without losing valid siblings. Generated output accepts only unique lowercase names
and resolved `-1` or non-negative ASCII decimals, always quotes values, preserves order, and emits
an explicit empty map when requested. Planned provider rows ask config-retention questions only;
no default, runtime enforcement, host resource, Podman normalization, or cross-format equivalence
is inferred.

Service `pull_policy` now travels through the same three boundaries without normalizing authored
spelling. Documented literal forms, `if_not_present`, valid `every_<duration>` intervals, deferred
values, schema-only `refresh`, and other values remain distinct. Invalid/provider-specific values
produce diagnostics but remain inspectable. Generated output covers documented forms only and
retains custom interval spelling and sensitivity under `every_([0-9]+[wdhms])+`. Schema-valid
`every_0s` remains classified as `Every`, with its prose semantics documented as ambiguous.
`pull_refresh_after` remains preserved as an unmodeled field with effective merge provenance;
provider-config matrix rows remain planned.

Service `stop_signal` and `stop_grace_period` now travel independently through the document,
effective-project, and generated-document boundaries. Signal spellings remain raw because Compose
does not define a normative signal-token grammar; quoted empty strings are preserved distinctly
from null. Stop grace periods have a lifecycle-specific value/expression/other state. ComposeLens's
raw-preserving policy uses the documented `us`, `ms`, `s`, `m`, and `h` units and accepts composite,
zero-with-unit, and fractional forms. Native zero and authored spelling are preserved; ComposeLens
performs no target-runtime normalization.

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
boolean that keeps omission distinct, retains deferred interpolation and complete replacement
provenance, and generates only an explicitly selected boolean. No omission default is invented.
Remaining resource limits are the next high-value promotion group.

## Promotion checklist

A service field moves into the project view only with:

1. correct Compose merge behavior;
2. source provenance for the field and meaningful nested values;
3. malformed-form diagnostics and partial recovery;
4. multi-file tests; and
5. an additive public API that keeps authored forms distinct where semantics can differ.
