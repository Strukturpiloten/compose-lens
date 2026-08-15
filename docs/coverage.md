# Native Compose coverage

This document distinguishes source preservation from typed consumer coverage. It was audited
against the commit-pinned official
[Compose service reference](https://docs.docker.com/reference/compose-file/services/) on
2026-08-14. The exact root/service-key inventory is committed in
[`schema/compose-key-inventory.json`](../schema/compose-key-inventory.json); it covers only the
current closed root and immediate service properties. Nested-schema drift remains intentionally
outside this bounded inventory and the promotion order lives in the [roadmap](roadmap.md).

## Coverage layers

| Layer          | Contract                                                                                                                 |
| -------------- | ------------------------------------------------------------------------------------------------------------------------ |
| Syntax         | Valid and recoverable YAML remains source-addressable; unknown fields and `x-` extensions are retained.                  |
| Document model | One Compose document exposes a source-aware native type without interpolation, merge, or normalization.                  |
| Project view   | An effective multi-file project exposes the typed value with merge provenance after optional explicit profile selection. |

A field is not ready for a converter merely because the document model types it. Multi-file
consumers need the project-view layer so they do not render and reparse a merged document or lose
provenance.

The closed-schema audit is now complete: resource metadata and `develop.watch.exec` children are
native at document and effective-project layers. They retain malformed evidence and sensitivity,
but deliberately have no generated, file, provider, driver, watcher, command, or runtime behavior.
Repository policy tests bind the claim to the official snapshot digest, exact 9 root/93 service key
sets, closed object shapes, and `x-*` extension allowance.

## Current service boundary

| Coverage                                                 | Service fields                                                                                                                                                                                                                                                                                                                                                                                   |
| -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Document model, project view, and generated construction | Existing typed service boundary plus `cpu_rt_runtime`, `cpu_shares`, `cpus`, `cpuset`, `device_cgroup_rules`, `ipc`, `mem_reservation`, `mem_swappiness`, `memswap_limit`, `network_mode`, `oom_kill_disable`, `oom_score_adj`, `pid`, `scale`, and `volumes_from`. Safe generated construction also covers selected raw identity and GPU `all` values with deterministic parse-back validation. |
| Document model only                                      | Explicitly bounded nested deploy-resource forms and malformed, extension, or future-unknown field evidence                                                                                                                                                                                                                                                                                       |
| Preserved, not typed                                     | No current closed-schema top-level or service keys; malformed or future nested include/model/GPU/develop members remain source-addressable through root or service `unmodeled_fields`, while bounded nested forms remain documented in the [roadmap](roadmap.md#exact-nested-semantic-gaps).                                                                                                     |

`blkio_config` retains raw integer-or-strict-string weights/rates and ordered device entries with
source and merge provenance. It applies no unit/range/default, path, device, cgroup/controller,
runtime, provider, I/O, conversion, or `extends` inheritance semantics.

`cgroup` retains a strict YAML string as `host`, `private`, a dollar-bearing deferred expression,
or diagnosed `Other` spelling, with source and scalar merge provenance. It applies no default,
controller, cgroup v1/v2, rootless, systemd, provider, version, runtime, or conversion semantics.

`cgroup_parent` retains a strict raw YAML string with source and scalar merge provenance. It applies
no grammar, path, controller, host, runtime, provider, version, default, `extends`, generation, or
conversion semantics.

`cpu_count` retains YAML integer/string category and exact spelling. Nonnegative YAML integers,
including unbounded, base-prefixed, separator, and negative-zero spellings, remain typed; negative
integers diagnose without loss. It applies no quota, host, scheduler, runtime, provider, OS,
version, default, `extends`, generation, or conversion semantics.

`cpu_percent` retains YAML integer/string category and exact spelling. YAML integers are classified
against the schema's inclusive `0..=100` range without fixed-width conversion; out-of-range values
diagnose without loss, while strings are never coerced or range-checked. It applies no percentage
calculation, CPU, quota, host, scheduler, runtime, provider, OS, version, default, `extends`,
generation, or conversion semantics.

`cpu_period` retains YAML number/string category and exact spelling without numeric conversion or
semantic validation. It applies no duration, microsecond, CFS, CPU, host, runtime, provider, OS,
version, default, `extends`, generation, or conversion semantics.

`cpu_quota` retains YAML number/string category and exact spelling without numeric conversion or
semantic validation. It applies no numeric quota, duration, microsecond, CFS, CPU, host, runtime,
provider, OS, version, default, `extends`, generation, or conversion semantics.

`cpu_rt_period` retains YAML number, duration, expression, and other-string categories with exact
spelling. Other strings diagnose without loss; no CPU calculation, microsecond conversion, realtime
scheduler, OS, host, default, provider, version, runtime, generation, or conversion semantics apply.

The preserved row follows the current Docker documentation grouping. Provider-specific additions
remain preserved even when they are not part of the compose-spec repository.

Long service-volume mounts additionally retain `consistency`, `bind.recursive`, and the `image`,
`tmpfs`, and `volume` option mappings through both authored and effective views. `image.subpath`,
`tmpfs.size`, `tmpfs.mode`, `volume.labels`, `volume.nocopy`, and `volume.subpath` retain source
spans, scalar categories where relevant, extensions, unknown/malformed evidence, interpolation,
and merge/reset/override provenance. ComposeLens neither resolves an image, volume, or path nor
interprets permissions, mount behavior, provider defaults, or runtime compatibility. Generation
does not yet write these new members.

`credential_spec` retains an explicit empty map and strict YAML-string config, file, and registry
members with nested merge provenance and recovery evidence. It does not resolve top-level configs,
paths, registries, accounts, Windows/gMSA, platform, provider, or runtime semantics.

`extends` retains a YAML-string short reference or a long mapping with optional strict YAML-string
`service` and `file` members, including explicit empties, extensions, unknown/malformed evidence,
and nested merge provenance. It preserves the schema-supported short form even though the
[Compose service reference](https://github.com/compose-spec/compose-spec/blob/main/05-services.md#extends)
focuses on mappings; the [schema](https://github.com/compose-spec/compose-spec/blob/main/schema/compose-spec.json)
and [merge rules](https://github.com/compose-spec/compose-spec/blob/main/13-merge.md) apply only
generic scalar replacement or recursive mapping merge here. These raw views do not expand or merge
a referenced service, look up files, normalize paths, traverse cycles, or import resources. The
separate `validate_references` stage may validate a same-file long-form `service` edge when `file`
is absent; it performs none of those operations or provider, platform, runtime, or conversion inference.

`provider` retains its required strict YAML-string `type` and optional ordered `options` mapping.
Option values retain YAML string/number/boolean scalar categories or ordered sequences of those
categories, with spans, malformed evidence, interpolation sensitivity, and generic merge/reset/
override provenance. It neither executes nor discovers providers, injects environment, resolves
credentials, validates provider grammar or compatibility, nor performs conversion.

`post_start` and `pre_stop` retain ordered hook mappings with a required null/scalar/list `command`, optional
local environment, privilege, user, and working-directory members, and per-item provenance and
recovery. Generic append/reset/override merge applies without hook execution, environment
inheritance calculation, lifecycle scheduling, privilege selection, provider compatibility, or conversion.

`pre_start` retains ordered hook mappings with optional null/scalar/list `command`, strict raw `image`,
local environment, boolean `privileged` and `per_replica`, user, and working-directory members.
It injects no defaults or lifecycle behavior, and generic append/reset/override merge retains provenance.

`runtime` retains strict YAML-string scalar spelling, including empties and deferred expressions,
with ordinary replacement/reset/override provenance and no runtime grammar, default, or compatibility claim.

`attach` retains only literal booleans or deferred dollar expressions through authored and effective
views; malformed forms remain source evidence. It has no default, generated API, logging, runtime,
provider, CLI, compatibility, or cross-format semantics.

`build.additional_contexts` retains raw ordered list items or scalar mapping entries, including duplicates, interpolation provenance, and generic map/list/reset/override behavior without parsing names, paths, URLs, images, or `service:` schemes. `build.context` preserves short/long form; `args`/`labels` retain map/list evidence; `cache_from`/`cache_to`, `platforms`, and `tags` retain raw ordered string sequences; `dockerfile` is non-empty and `dockerfile_inline` retains exact empty or multiline strings while `target`/`network`/`isolation` remain opaque. Boolean/string `no_cache`/`sbom`, boolean/expression `privileged`/`pull`, and raw-preserving `shm_size` retain sensitivity, provenance, reset/override, and partial recovery.
`build.extra_hosts` remains separate from service `extra_hosts`: raw string lists and mapping hostname keys with scalar or nested-list raw string addresses retain form, order, interpolation provenance, recursive-map/list merge evidence, and malformed recovery without address validation, DNS/host access, build generation, or conversion.
`build.entitlements` retains opaque ordered raw strings, including duplicates and empties, plus interpolation and append/reset/override provenance. Docker Compose v2.27.0 is a documented implementation badge; earlier and removal boundaries remain unknown.
`build.dockerfile_inline` retains source spans, interpolation sensitivity, scalar replacement/reset/override provenance, malformed recovery, and mutual-exclusion evidence with `dockerfile`; it does not parse Containerfile syntax, access paths or contexts, scan secrets, build, or infer Docker, BuildKit, or runtime behavior. Docker Compose v2.17.0 is a documented implementation badge; earlier and removal boundaries remain unknown.
`build.provenance` retains YAML boolean or opaque string category, interpolation sensitivity, scalar merge provenance, and malformed evidence without attestation parsing, generation, publication, validation, builder execution, or runtime inference. Docker Compose v2.39.0 is a documented implementation badge; earlier and removal boundaries remain unknown.
`build.privileged` accepts YAML literal booleans or deferred dollar expressions. Ordinary quoted
non-expression strings are rejected rather than coerced and remain source-addressable unmodeled
evidence with diagnostics. Docker Compose v2.15.0 is a documented implementation badge; earlier
and removal boundaries remain unknown. No privilege, platform, runtime, or build behavior is
inferred.
Cache descriptors, platforms, tags, isolation, and `shm_size` are typed source values, but their feasibility, provider support, service-platform coupling, and other runtime semantics are not modeled. `no_cache` and `sbom` strings receive no boolean coercion, `sbom` receives no generator parsing or generated-data exposure, and `pull` receives no environment resolution, default policy, or runtime inference. `build.shm_size` retains service-equivalent number/string spelling, lowercase-unit, zero, expression, and provider-dependent classification.
`build.ssh` retains sensitive ordered strings or scalar mapping entries under generic merge rules; it parses no SSH identifier, path, PEM, socket, agent, mount, or builder semantics and redacts all grants by default.
`build.ulimits` reuses the service Ulimits models: ordered names, scalar/range syntax, spelling,
per-file interpolation sensitivity, nested recursive merge, reset/override, malformed evidence,
and explicit empties remain visible without defaults, normalization, host-limit, builder, or runtime claims.
`dockerfile`/`dockerfile_inline` retain source-spanned conflict evidence. All closed immediate Build keys are typed, while malformed input, extensions, future unknown members, and deliberately bounded nested semantics remain source-addressable evidence. ComposeLens does not generate builds.

`deploy.endpoint_mode` and `deploy.mode` retain exact `vip`/`dnsrr` and `global`/`replicated`
values plus raw `Other` strings with a portability diagnostic. `deploy.replicas` retains exact
YAML number spelling or a distinct YAML string category, including empty and deferred strings.
Values interpolate before merge and retain scalar replacement, reset, override, sensitivity, and
provenance; malformed, extension, future-unknown, and explicitly bounded nested resource forms
stay source-addressable evidence. No integer grammar, positive/zero/default rule, mode coupling, scale, allocation,
scheduling, platform, discovery, VIP, DNS, deploy, runtime, or conversion behavior is inferred;
there is no version-boundary badge.

`deploy.labels` remains distinct from service container labels. It preserves mapping scalar/null
categories or ordered raw list entries; mappings merge by key while lists append duplicate
fallible-input evidence despite `uniqueItems`. Reset/override provenance, sensitivity, and malformed
evidence remain visible without container, service, runtime, platform, deployment, or conversion claims.

`deploy.restart_policy` retains deploy-specific `condition`, `delay`, `max_attempts`, and `window`
members. Conditions preserve documented, deferred, and unknown spellings; durations remain raw
strings and attempts retain YAML integer/string categories. Nested member provenance, reset, and
malformed evidence remain visible without a service-restart fallback, default, precedence, attempt
simulation, runtime, conversion, or version-boundary claim.

`deploy.placement` retains ordered YAML-string `constraints`, ordered preference mappings with an
optional YAML-string `spread`, and YAML-integer or YAML-string `max_replicas_per_node` categories.
The effective view retains collection, item, and nested-member provenance through append,
replacement, reset, and override; extensions, unknowns, and malformed values stay evidence. No
constraint/spread grammar, node selection, count/range/default, mode coupling, scheduling,
runtime, conversion, or version boundary is inferred.

`deploy.resources.limits.cpus` retains YAML-number or YAML-string scalar categories,
`deploy.resources.limits.memory` retains YAML-string-only raw text with documented lowercase-unit,
lexical-zero, deferred, and provider-dependent classification, and `limits.pids` retains YAML-integer
or YAML-string categories. `deploy.resources.reservations.cpus` separately retains YAML-number or
YAML-string scalar categories, while `reservations.memory` reuses the YAML-string-only raw memory
classification. Recursive mapping merge and reset/override provenance remain visible
with nested unknown, extension, malformed, and sensitive evidence. They infer no service CPU,
`mem_limit`, unlimited, positivity, range/default, host, cgroup, runtime, consistency, conversion,
or version-boundary behavior.

`deploy.resources.reservations.generic_resources` is schema-only list evidence. Ordinary
sequences append, and reset/override retain collection and item provenance. Each item preserves a
mapping/unmodeled form plus optional raw `discrete_resource_spec.kind` and number-or-string
`value`; malformed input remains observable. No prose, version, provider, matching, scheduling,
device, runtime, or conversion semantics are inferred; reservation-device semantics remain outside
this generic-resource boundary.

`deploy.resources.reservations.devices[]` is schema-only ordered evidence. Device mapping/unmodeled
forms, capabilities, strict YAML-string drivers, raw YAML-integer-or-string `count`, and ordered
strict-YAML-string/unmodeled `device_ids` preserve spans, sensitivity, and append/reset/override
provenance. Timestamp, regex, other scalar, and collection forms are retained as malformed evidence;
simultaneous count and IDs receive a conflict diagnostic without selecting either. No device
selection, capability/driver grammar, allocation matching, CDI, host, runtime, provider/version,
or conversion behavior is inferred. Options retain map/list syntax, scalar fidelity, malformed
evidence, exact duplicate list strings, and generic provenance without provider interpretation.

## Current top-level boundary

All current closed-schema top-level keys have document-model and project-view support. `version`
is retained with an obsolete-field diagnostic; `include` and `models` retain their bounded native
forms without loading files, environments, or providers. Malformed and future nested values remain
source-addressable evidence as documented in the [roadmap](roadmap.md).

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
`pull_refresh_after` now travels through authored and effective views as an independent strict raw
YAML string. Empty/deferred spelling, malformed evidence, scalar replacement/reset/override provenance,
and sensitivity remain visible. Its interval grammar, minimum, default, relationship to
`pull_policy: refresh`, refresh state, provider support, generation, and conversion remain unmodeled;
provider-config matrix rows remain planned.

Service `platform` now travels through authored and effective views as an independent strict raw YAML
string. Empty/deferred spelling, malformed evidence, scalar replacement/reset/override provenance, and
sensitivity remain visible. OCI component grammar, normalization, aliases/case, host/image-manifest or
build feasibility, provider support, and `build.platforms` coupling remain unmodeled; no generation or
conversion is supported.

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
all contributing spans. Service `init`, `stdin_open`, `tty`, and `privileged` also cross all three boundaries as
independent authored/project booleans that keep omission distinct, retain deferred interpolation
and complete replacement provenance, and generate only explicitly selected booleans. No omission
default, terminal, security, or runtime policy is invented.
Remaining resource limits are the next high-value promotion group.

## Promotion checklist

A service field moves into the project view only with:

1. correct Compose merge behavior;
2. source provenance for the field and meaningful nested values;
3. malformed-form diagnostics and partial recovery;
4. multi-file tests; and
5. an additive public API that keeps authored forms distinct where semantics can differ.
