# API stability policy

ComposeLens is pre-1.0, but its API is not an unbounded experiment. Version 0.1 establishes one
documented public release line for early BoxFerry integration and independent consumers. This
policy is recorded by [ADR 0013](decisions/0013-versioned-public-api-and-release-contract.md).

## The 0.1.x contract

Within the 0.1.x line:

- patch releases preserve source compatibility for supported public entry points;
- public APIs use ComposeLens-owned types, while `yaml-edit` remains private;
- the module paths used by `tests/public_api.rs` remain available;
- diagnostic code strings remain machine-readable contracts;
- canonical-v1 default rendering remains deterministic for the same semantic input;
- parsing, processing, validation, and rendering keep their documented side-effect boundaries; and
- all supported public APIs compile on Rust 1.85.0 or newer.

Bug fixes may change a result that contradicted a documented contract or retained conformance
evidence. Such a change needs a regression test and changelog entry. A patch release must not
silently normalize a preserved syntax form, expose a parser-dependency type, perform new I/O, or
rename a diagnostic code.

## Supported entry points

The 0.1 consumer contract covers these explicit stages:

| Stage | Public modules |
| --- | --- |
| Source and diagnostics | `source`, `diagnostic`, `syntax` |
| Native Compose types | `model` |
| Caller-owned environment and project inputs | `interpolation`, `loader` |
| Merge and post-merge views | `merge`, `profiles`, `project`, `resolution` |
| Versioned compatibility reports | `validation` |
| Canonical/generated output and scalar preservation edits | `render` |

The compile-and-behavior contract in `tests/public_api.rs` exercises these paths as an external crate
consumer would. The additive `project` module provides native values without hiding loading,
interpolation, merging, or profile selection. The modules remain separate deliberately; 0.1 does
not add a convenience function that hides file access, interpolation, merging, profile selection,
validation, or rendering.

The additive generated-document path accepts only explicit Compose-owned values and performs no
processing or I/O. Successful output is parse-back validated through the syntax and native model.
Additive `init`, `stdin_open`, `tty`, and `privileged` getters retain omitted, literal, and deferred states at the
authored-document and effective-project layers; their generated setters emit only explicitly
supplied booleans.
Additive `Service::attach` and `ProjectService::attach` retain omitted, literal, and deferred
states, plus malformed source evidence, at the authored-document and effective-project layers.
They introduce no default, generated API, logging, runtime, provider, CLI, compatibility, or
cross-format semantics.
`Service::blkio_config` and `ProjectService::blkio_config` add authored/effective source-aware
integer-or-string scalar and ordered-item access only. They have no generated API, defaults,
controller/runtime/provider/I/O/conversion behavior, or `extends` path-keyed inheritance claim.
Additive `BuildDefinition::{additional_contexts,context,args,cache_from,cache_to,dockerfile,dockerfile_inline,entitlements,extra_hosts,target,network,isolation,platforms,no_cache,privileged,sbom,pull,shm_size,tags,labels,secrets,ssh,ulimits}` and `ProjectBuild` retain form, scalar spelling, sensitivity, provenance, explicit empties, duplicates, reset/override, and partial recovery. `additional_contexts` retains raw ordered list spelling or scalar mapping entries without parsing names, paths, URLs, images, `service:` schemes, or builder behavior. Build-specific `extra_hosts` remains distinct from service `ExtraHosts`, retaining raw string list entries and mapping hostname keys with scalar or nested-list string addresses. `entitlements` retains opaque ordered strings without an allowlist, privilege, BuildKit/platform, execution, or runtime claim; Docker Compose v2.27.0 is an implementation badge only, with earlier and removal boundaries unknown. `dockerfile_inline` retains exact empty or multiline string content, interpolation sensitivity, scalar merge provenance, and conflicts without Containerfile parsing, path/context access, secret scanning, build execution, or Docker/BuildKit/runtime claims; Docker Compose v2.17.0 is an implementation badge only, with earlier and removal boundaries unknown.
`cache_from`, `cache_to`, and `platforms` are ordered raw strings; `no_cache` and `sbom` retain YAML boolean/string distinction without string coercion. ComposeLens neither parses cache descriptor/reference/path/credential or OCI grammar nor validates availability, service platform, defaults, build execution, or SBOM generators/data.
`build.privileged` retains literal booleans or deferred dollar expressions in `BooleanValue` at both
authored and effective layers. Ordinary quoted non-expression strings are rejected rather than
coerced and remain source-addressable unmodeled evidence with diagnostics. Docker Compose v2.15.0
is an implementation badge only, with earlier and removal boundaries unknown; privilege,
platform, runtime, and build behavior remain outside this API contract.
`isolation` accepts only opaque YAML string scalars and remains independent from service `isolation`; it does not validate modes, platforms, privileges, defaults, or `BUILDAH_ISOLATION` behavior.
`build.shm_size` exposes the existing `ShmSize` model at authored and effective layers, preserving YAML number/string spelling and the documented lowercase-unit, lexical-zero, deferred-expression, and provider-dependent classifications. It infers no builder default, host setting, allocation, or runtime behavior.
`build.ulimits` exposes the existing `Ulimits` and `ProjectUlimits` models with ordered keys, single/range form, retained scalar spelling, independent soft/hard provenance, and malformed evidence. It applies neither defaults nor `-1`/name normalization and validates no host, builder, or runtime behavior.
`BuildSsh` and `ProjectBuildSsh` retain list-string or mapping string/number/boolean/null forms. SSH values and mapping entries are always sensitive, redact from `Debug`, and expose raw data only through explicit accessors; no grant identifier, path, PEM, socket, agent, mount, or build behavior is parsed or accessed.
`BuildProvenance` is distinct from `BuildSbom` and retains only YAML boolean or opaque string form, source span, sensitivity, scalar merge provenance, and malformed evidence; it makes no attestation or builder/runtime claim. Docker Compose v2.39.0 is an implementation badge only, with earlier and removal boundaries unknown.
Other build fields remain source-addressable unmodeled evidence, and generated build output is outside this boundary.
Additive `CredentialSpec` and `ProjectCredentialSpec` retain mapping span/member provenance,
strict YAML-string config/file/registry spelling, explicit empties, malformed/extension/unknown
evidence, and generic merge sensitivity. They resolve no top-level config, path, file, registry,
account, URI, Windows/gMSA, platform, provider, runtime, or conversion semantics.
Additive `Extends`, `ExtendsReference`, `ProjectExtends`, and `ProjectExtendsReference` retain the
schema-supported YAML-string short form and long mapping `service`/`file` members, explicit
empties, mapping/member provenance, sensitivity, and malformed/extension/unknown evidence. A
missing long-form `service` has a stable diagnostic. Generic scalar replacement and recursive
mapping merge apply without service expansion/merge, file lookup, path normalization, cycle
traversal, or resource import. The separate `validate_references` stage may validate a same-file
long-form `service` edge without `file`, while claiming no provider, platform, runtime, or
conversion semantics.
Additive `Provider`, `ProviderOptions`, `ProjectProvider`, and `ProjectProviderOptions` retain a
required strict YAML-string `type`, ordered nonempty-key options, string/number/boolean scalar or
sequence categories, source spans, nested provenance, sensitivity, and malformed evidence. Generic
merge/reset/override behavior applies without provider execution/discovery, setup/teardown,
environment injection, credential resolution, provider grammar/compatibility validation, or
conversion semantics. No generated provider API is supported.
Additive `PostStartHooks`, `PostStartHook`, `PreStopHooks`, `PreStopHook`, `ServiceHook`,
`ProjectPostStartHook`, `ProjectPreStopHook`, `ProjectService::post_start`, and
`ProjectService::pre_stop` retain ordered hooks with required null/scalar/list commands, local
map/list environments, optional privilege/user/working-directory members, malformed evidence, and
generic append/reset/override provenance. They do not execute or schedule hooks, calculate
environment inheritance, apply defaults, decide privilege, validate provider compatibility, or
support generated construction.
Additive `PreStartHooks`, `PreStartHook`, `PreStartServiceHook`, `ProjectPreStartHook`,
`ProjectPreStartServiceHook`, and `ProjectService::pre_start` retain optional commands, strict raw
images, local environments, optional privilege/per-replica/user/working-directory members, malformed
evidence, and generic append/reset/override provenance. They infer no defaults, lifecycle/runtime
behavior, provider compatibility, conversion, or generated construction.
Additive `Service::runtime` and `ProjectService::runtime` retain strict YAML-string values with
empty/deferred spelling, provenance, sensitivity, and malformed evidence. They infer no runtime
grammar, default, compatibility, execution, generated construction, or conversion semantics.
Additive `CgroupNamespace`, `CgroupNamespaceKind`, `Service::cgroup`, and `ProjectService::cgroup`
retain strict YAML-string spelling as `host`, `private`, deferred expression, or diagnosed `Other`.
They supply no default, controller, cgroup version, rootless, systemd, provider, compatibility,
runtime, `extends`, generated, or conversion semantics.
Additive `Service::cgroup_parent` and `ProjectService::cgroup_parent` retain strict raw YAML-string
spelling, provenance, sensitivity, and malformed evidence independently from `cgroup`. They supply
no grammar, path, controller, host, runtime, provider, version, default, `extends`, generated, or
conversion semantics.
Additive `CpuCount`, `Service::cpu_count`, and `ProjectService::cpu_count` retain YAML
integer/string category and exact spelling, including unbounded/base/separator/negative-zero
integers and diagnosed negative integers. They supply no numeric conversion, quota, host,
scheduler, runtime, provider, OS, version, default, `extends`, generated, or conversion semantics.
Additive `CpuPercent`, `Service::cpu_percent`, and `ProjectService::cpu_percent` retain exact
YAML integer/string categories. YAML integers are classified against the schema's inclusive
`0..=100` range without fixed-width conversion; quoted, block, empty, and deferred strings are
not coerced or range-checked. They supply no percentage calculation, CPU, quota, host, scheduler,
runtime, provider, OS, version, default, `extends`, generated, or conversion semantics.
Additive `CpuPeriod`, `Service::cpu_period`, and `ProjectService::cpu_period` retain exact YAML
number/string categories without numeric conversion or semantic validation. Quoted, block, empty,
and deferred strings remain strings. They supply no duration, microsecond, CFS, CPU, host, runtime,
provider, OS, version, default, `extends`, generated, or conversion semantics.
Additive `CpuQuota`, `Service::cpu_quota`, and `ProjectService::cpu_quota` retain exact YAML
number/string categories without numeric conversion or semantic validation. Quoted, block, empty,
and deferred strings remain strings. They supply no numeric quota, duration, microsecond, CFS, CPU,
host, runtime, provider, OS, version, default, `extends`, generated, or conversion semantics.
Additive `CpuRtPeriod`, `Service::cpu_rt_period`, and `ProjectService::cpu_rt_period` retain exact
YAML-number, Compose-duration, expression, and other-string categories. Duration strings use only
the existing raw `us`/`ms`/`s`/`m`/`h` policy; other strings diagnose without loss. They supply no
CPU calculation, microsecond conversion, realtime scheduler, OS, host, default, provider, version,
runtime, generated, or conversion semantics.
Additive `DeployDefinition::{endpoint_mode,mode,replicas}`, `ProjectService::deploy`, and `ProjectDeploy` expose
effective deploy endpoint mode/mode/replicas values and retain malformed, extension, and future-unknown deploy
evidence. `DeployEndpointMode`
retains `vip`, `dnsrr`, or raw `Other(String)` values; other strings receive a portability
diagnostic without rejection, while non-string forms remain source-addressable evidence. The prose
`vip` default and schema lack of an effective default are intentionally unresolved, so this API
injects no default and claims no platform, discovery, VIP, DNS, replica, deployment, runtime, or
conversion behavior.
`DeployMode` likewise retains `global`, `replicated`, or raw `Other(String)` values; empty,
deferred, and provider-specific strings diagnose portability without coercion, and omission does
not become `replicated`. No replica, scale, placement, job, deployment, runtime, or conversion
behavior is within this API contract.
`DeployReplicas` retains raw YAML number spelling or a distinct YAML string form, including empty
and deferred strings. It validates no integer grammar and applies no positive/zero/default rule,
mode coupling, scale, allocation, scheduling, runtime, or conversion behavior; no version boundary
is claimed.
`DeployDefinition::labels` and `ProjectDeploy::labels` expose deployment labels separately from
service container labels. They retain mapping scalar/null categories or ordered raw list spelling;
`ProjectLabelsForm` preserves an explicit empty map/list distinction, mapping keys merge while
lists append duplicate fallible-input evidence. No container, service, runtime, platform,
deployment, or conversion behavior or version boundary is claimed.
`DeployDefinition::restart_policy` and `ProjectDeploy::restart_policy` are deploy-specific. They
retain member-level provenance and malformed evidence without service-restart fallback, defaults,
precedence, attempt simulation, runtime, or conversion claims.
Update_config retains explicit map form, member provenance, strict string values, raw scalar
categories, and retained malformed or provider-specific order evidence. No rollout, scheduling,
duration, failure-rate, default, runtime, or conversion behavior is inferred.
Rollback_config has a distinct public authored/effective type with the same explicit map form,
member provenance, strict string values, raw scalar categories, and retained malformed or
provider-specific order evidence. No rollout, execution, order, monitor, failure-rate, default,
scheduler, provider, runtime, version, or conversion behavior is inferred.
`DeployDefinition::placement` and `ProjectDeploy::placement` expose ordered YAML-string
constraints, preference mappings with optional string `spread`, and max-replicas-per-node as a
YAML-integer or YAML-string category. The effective values preserve collection, item, and nested
member provenance across append, replacement, reset, and override; extensions, unknown members,
and malformed values remain source-addressable evidence. The API supplies no constraint/spread
grammar, node-selection, count/range/default, mode coupling, scheduling, runtime, or conversion
interpretation.
`DeployDefinition::resources` and `ProjectDeploy::resources` expose `limits.cpus` as a YAML-number
or YAML-string category, `limits.memory` as a YAML-string-only deploy-specific type, and
`limits.pids` as a YAML-integer or YAML-string category; `reservations.cpus` is a context-specific
YAML-number or YAML-string category, and `reservations.memory` reuses the YAML-string-only
deploy-specific memory type. CPU and PID retain exact scalar spelling; memory retains raw
text and conservatively classifies documented lowercase byte units, lexical zero, deferred
expressions, and provider-dependent strings. Nested mapping merge, replacement, reset, override,
sensitivity, and malformed/unknown/extension evidence remain observable at every level. These
contracts infer no service CPU, `mem_limit`, unlimited, positivity, range/default, host, cgroup,
runtime, consistency, or conversion behavior.
`DeployResourceReservations::generic_resources` and its project-view equivalent expose only
schema-only list evidence: ordinary sequences append, while reset and override retain their
operation/provenance. Each retained item distinguishes mapping from unmodeled form and may carry
an optional raw `discrete_resource_spec.kind` string and number-or-string `value`; malformed
items and members remain evidence. This API makes no prose, version, provider, matching,
scheduling, device, runtime, or conversion claim; reservation `devices` remains outside it.
`DeployResourceReservations::devices` and its project-view equivalent add schema-only ordered
device evidence. Mapping/unmodeled device forms retain extensions and unknown members; the required
capabilities list retains exact string/unmodeled items, duplicates, sensitivity, and merge
provenance. The optional `driver` is an exact YAML string scalar; timestamp and regex styles,
other scalar kinds, and collections remain recoverable unmodeled evidence. No driver grammar,
default, loading, selection, CDI, host, runtime, provider/version, or conversion behavior is
claimed. `count` retains only raw YAML-integer or strict YAML-string spelling, while `device_ids`
retains ordered strict YAML-string/unmodeled items. Their simultaneous presence is diagnosed without
choosing or discarding either. There is no count range, sign, default, or `all` semantics. `options`
retains its map or list form: maps preserve ordered, strict nonempty YAML-string keys and shared
scalar values, while lists preserve ordered strict YAML strings, including empty and duplicate
items. Invalid entries remain source-addressable, ordinary device-list merge rules apply, and no
option key, value, driver, provider, default, device-selection, runtime, or conversion semantics
are inferred.
Additive lifecycle getters expose `stop_signal` independently from the lifecycle-specific
`StopGracePeriod` state at both authored-document and effective-project layers. Generated setters
retain caller spelling and sensitivity without applying target-runtime normalization.
Additive `PullPolicy` getters retain exact authored spelling and separate documented, aliased,
deferred, schema-only, and other classifications. `GeneratedPullPolicy` is non-exhaustive and
emits documented forms only. Additive `pull_refresh_after` getters retain strict raw YAML-string
spelling, source spans, sensitivity, and scalar merge provenance without an interval grammar,
default, `pull_policy: refresh` coupling, provider support, generation, or conversion claim.
Additive `platform` getters retain strict raw YAML-string spelling, source spans, sensitivity, and
scalar merge provenance without OCI component parsing/normalization, aliases/case, host or image-manifest
inspection, build feasibility, provider support, `build.platforms` coupling, defaults, generation, or conversion.
Additive `PidsLimit` getters retain exact spelling and separate unlimited, arbitrary-precision
finite, ambiguous zero, deferred, and other states. Non-exhaustive `GeneratedPidsLimit` emits only
unlimited or validated positive ASCII-decimal values; omission remains omission.
Additive `ShmSize` getters retain exact scalar text, source span, YAML number/string provenance,
documented lowercase units and amount spelling, ambiguous zero, deferred expressions, and distinct
provider-dependent number/string states. Public shared-memory enums are non-exhaustive.
`GeneratedShmSize` emits only a quoted canonical positive ASCII-integer amount with an explicit
documented lowercase unit; omission remains omission and no provider default is injected.
Additive `MemLimit` getters retain exact scalar text, span, YAML number/string provenance,
documented lowercase units and amount spelling, lexical zero, deferred expressions, schema-only
numbers, and provider-dependent strings. `GeneratedMemLimit` emits only a quoted canonical positive
ASCII-decimal amount with a distinct explicit unit; omission remains omission and no default,
deploy reconciliation, runtime enforcement, host/cgroup inspection, or non-byte exact
cross-format claim is introduced.
Additive `Tmpfs`/`TmpfsItem` getters retain omission, scalar/list form, explicit empty lists,
`<path>[:<options>]` spelling, documented versus expression/provider-dependent classification,
exact duplicates, spans, sensitivity, and merge provenance. `GeneratedTmpfs` is non-exhaustive and
retains scalar/list form, duplicates, documented assignments, and well-shaped raw target options;
omission remains omission and service `tmpfs` stays distinct from volume type `tmpfs`.
Additive `Sysctls`/`SysctlsForm` getters retain omission, explicit empty mapping/list forms,
ordered mapping entries, exact scalar kinds and spelling, ordered list strings, duplicates, spans,
sensitivity, and generic merge provenance. `ProjectSysctls` retains per-entry provenance and key
locations. `GeneratedSysctls` emits only quoted resolved strings, rejects unsafe or exact-duplicate
input, and performs no namespace, privilege, runtime, or cross-format interpretation.
Additive `ProjectUlimits`, ordered `ProjectUlimit`, `ProjectUlimitValue`, `ProjectUlimitRange`, and
`ProjectUlimitScalar` getters retain omission, explicit empty/reset mappings, lowercase keys,
single/range form, authored/effective scalar spelling and kind, nested recursive-merge provenance,
sensitivity, replacement, and override. Existing authored `Ulimits` types and diagnostic codes
remain available. `GeneratedUlimits`/`GeneratedUlimit` preserve order and explicit empty maps,
emit only quoted `-1` or non-negative ASCII decimals, and reject duplicate or non-lowercase names,
missing range members, deferred/multiline/NUL values, and arbitrary schema strings such as `host`.
Additive `Logging`, `LoggingOptions`, and `LoggingOptionValue` getters retain empty mappings,
uninterpreted string drivers, ordered non-empty option keys, exact string/number/null scalar kinds,
source spans, extensions, unknowns, and malformed-entry recovery. `ProjectLogging` retains nested
replacement/recursive-merge/reset/override provenance, authored/effective string spelling, and
sensitivity. `GeneratedLogging` requires an explicit string driver, emits ordered unique-key
string/validated-number/null options, and performs no defaulting or provider interpretation.
Additive `Hostname` getters retain the exact YAML string scalar and classify resolved RFC-1123
literals, deferred dollar-bearing expressions, and invalid literals without deleting the service.
`HostnameKind` and `GeneratedHostname` are non-exhaustive; generated output accepts only a
validated resolved literal, and omission remains omission.
Additive `CapabilityAdd`/`CapabilityAddItem` and `CapabilityDrop`/`CapabilityDropItem` getters
retain omission versus explicit empty state, exact item strings and spans, schema-duplicate
diagnostics, order, and lexical exact-candidate classification. Effective values retain full field
and per-item merge provenance and sensitivity. `GeneratedService::set_cap_add` and
`GeneratedService::set_cap_drop` each set one independent complete vector exactly once, including
an explicit empty vector, and reject unsafe or exact-duplicate items without a whitelist, case
normalization, or cross-field reconciliation.
Additive `Devices`, `Device`, `ShortDevice`, and `LongDevice` getters retain omission versus
explicit empty state, ordered mixed short/long forms, exact duplicates, raw CDI/deferred/opaque
short strings, required long string `source`, optional raw `target`/`permissions`, spans,
extensions, unknown fields, sensitivity, and target-keyed merge provenance. `ProjectDevice` and
`ProjectLongDevice` expose item and nested provenance. `GeneratedDevice`/`GeneratedLongDevice` and
`GeneratedService::set_devices` emit safe resolved quoted strings, retain order and duplicates,
and parse-back validate without host-device, colon-triple, CDI, permissions, or runtime checks.
Additive DNS types and getters cover `dns`, `dns_opt`, and `dns_search`, retaining
authored form, ordering, duplicates, provenance, reset/override state, and sensitivity. Their
generated APIs accept only resolved physical-line-safe values and parse back the emitted document.

Additive `Expose` and `Annotations` types retain scalar or syntax identity and their documented
field-specific merge evidence. Generated construction requires safe, unambiguous values.

Additive `SecurityOptions` APIs preserve the raw ordered sequence and expose non-selecting lexical
candidates for AppArmor, no-new-privileges, seccomp, SELinux labels, Mask, and Unmask. Near misses,
duplicates, and conflicts stay observable; no profile, filesystem, provider, runtime, or
cross-format policy is inferred.
Additive `GeneratedNetworkAttachment` IPv4/IPv6 setters preserve omission, raw `GeneratedString`
spelling, sensitivity, aliases, and named-network scope. Each address is set at most once, and
generation applies no IP grammar, IPAM-pool, default, provider, or runtime validation.
Additive `GeneratedNetworkDefinition`, `GeneratedNetworkDriverOption`, and
`GeneratedNetworkDriverOptionValue` preserve the existing `GeneratedResource` basic/external
network API while adding application-owned optional opaque drivers and ordered unique
string-or-number `driver_opts`. External networks remain `GeneratedResource::external` because
their only allowed attribute is `name`. Number spelling is validated without coercing quoted string
values, and no driver/plugin or option semantics are inferred.

Additive `GeneratedVolumeDefinition`, `GeneratedVolumeDriverOption`, and
`GeneratedVolumeDriverOptionValue` use a distinct public contract rather than reusing network
driver-option types. They represent application-owned volumes only, retain optional exact names,
opaque drivers, ordered unique string-or-number `driver_opts`, explicit empty maps, scalar spelling,
and sensitivity. The compatible `GeneratedResource` API remains the sole generated external-volume
path; driver/plugin/provider/runtime/default/image semantics are not inferred.

The same additive `GeneratedVolumeDefinition` API accepts ordered unique volume `labels` through
the existing `GeneratedLabel` type. They preserve omission versus an explicitly empty mapping, set
once, render deterministically, and propagate sensitivity. Authored literal `external: true` plus
any `labels` attribute, including an explicit empty mapping or list, reports the distinct stable
`compose.volume.external-labels-configuration` diagnostic while retaining both values. This does
not replace the existing driver-configuration diagnostic, so each violation remains independently
actionable when both fields are present.

The same additive `GeneratedNetworkDefinition` API accepts ordered unique network `labels` through
the existing `GeneratedLabel` type. They preserve omission versus an explicitly empty mapping,
set once, and propagate sensitivity; key-only, null, number, boolean, provider-injected, and
runtime-equivalence forms are outside this generated subset. The resolved unique `key=value`
subset is the cross-format exactness boundary, not an extra restriction on this Compose-native
constructor.

The same additive `GeneratedNetworkDefinition` API supports set-once literal `enable_ipv6` and
`internal` choices. Their `Option<bool>` accessors preserve omission versus explicit `false` and
`true`; generation injects no default and exposes no generated `enable_ipv4` counterpart.

## Changes before 1.0

Rust's semantic-versioning convention permits breaking changes in the next pre-1.0 minor release.
ComposeLens still requires an ADR when the processing architecture changes, release notes with a
migration section, and a new 0.x minor version for an intentional public break. Consumers that
cannot absorb that cadence should use an exact dependency requirement or commit their lockfile.

Adding a variant to one of the public compatibility-context enums marked `#[non_exhaustive]` is not
a breaking change. Other public enums may become non-exhaustive only in a breaking release because
adding that attribute itself affects downstream exhaustive matches.

## Not promised by 0.1

The 0.1 contract does not claim:

- complete coverage of every Compose field;
- structural source editing beyond the documented scalar boundary;
- behavior parity among the Compose Specification, Docker Compose, and `podman-compose`;
- runtime effects from provider-only `config` observations; or
- long-term 1.x compatibility.

Before 1.0, the project will define supported release lifetimes, deprecation periods, and the
1.x diagnostic-code policy through a superseding ADR.
