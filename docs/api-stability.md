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
Additive lifecycle getters expose `stop_signal` independently from the lifecycle-specific
`StopGracePeriod` state at both authored-document and effective-project layers. Generated setters
retain caller spelling and sensitivity without applying target-runtime normalization.
Additive `PullPolicy` getters retain exact authored spelling and separate documented, aliased,
deferred, schema-only, and other classifications. `GeneratedPullPolicy` is non-exhaustive and
emits documented forms only; `pull_refresh_after` remains source-addressable unmodeled evidence.
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
