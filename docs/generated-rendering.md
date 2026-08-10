# Generated Compose documents

ComposeLens has two deterministic output paths with different inputs:

- canonical rendering consumes a provenance-bearing `MergedProject`; and
- generated rendering consumes caller-constructed Compose-native values that have no authored
  YAML source.

`ComposeDocumentBuilder` is the generated-document boundary. It performs no file, environment,
provider, or runtime access. `build` emits deterministic two-space/LF YAML and immediately parses
the bytes through ComposeLens's loss-aware syntax and typed-document model. A successful
`GeneratedComposeDocument` therefore contains both deployable text and a parse-back-validated
`ComposeDocument`.

## Initial generated subset

`GeneratedService` supports the first runtime-migration fields:

- an optional resolved service `hostname`, independently validated with conservative ASCII
  RFC-1123 rules and never derived from another name;
- an optional explicit runtime `container_name` validated against Compose's portable grammar;
- image references, including `name:tag@digest` spellings;
- independent string/list entrypoints and exec/shell commands, including explicitly empty forms;
- independent explicit boolean `init`, `stdin_open`, `tty`, and `privileged` choices;
- ordered environment-file short and long declarations with optional `required` and `format: raw`;
- ordered literal and host-resolved environment entries;
- ordered service metadata labels with explicit, potentially empty string values;
- an optional complete ordered service annotation mapping, including an explicit empty map and
  unique resolved non-empty names with explicit resolved string values;
- combined `user[:group]`, user namespace, supplementary groups, working directory, and explicit
  read-only-root state;
- independent optional complete ordered `cap_add` and `cap_drop` vectors, including explicitly
  configured empty vectors;
- an optional complete ordered mixed short/long service `devices` vector, including explicit empty
  state, raw CDI-like strings, exact duplicates, and long source/target/permissions;
- raw service DNS scalar/list form, including explicit empty lists and exact duplicates;
- unlimited or positive integral service PID limits;
- quoted positive canonical service shared-memory sizes with explicit documented lowercase units;
- quoted positive canonical service memory limits with explicit documented lowercase units;
- scalar/list service-level temporary filesystems, including explicit empty lists and raw options;
- mapping/list service sysctls, including explicit empty collections and ordered quoted strings;
- service logging with an explicit uninterpreted string driver and ordered string/number/null options;
- ordered service ulimits with quoted single or required soft/hard values and explicit empty maps;
- service-level `no`, `always`, `on-failure[:max-retries]`, and `unless-stopped` restart policies;
- documented service image pull policies, including exact custom interval spelling;
- independent raw stop signals and raw-preserving Compose stop grace periods;
- ordered short-form host mappings;
- protocol-aware target/published/host-address ports;
- named, anonymous, and bind mounts;
- ordered network attachments with aliases and optional raw per-attachment IPv4/IPv6 addresses; and
- application-owned top-level network definitions with optional opaque drivers and scalar-kind-aware
  driver options; and
- application-owned top-level volume definitions with optional opaque drivers and distinct
  scalar-kind-aware driver options; and
- application-owned or external top-level volume definitions, with optional exact platform-level names.

Generated collections retain insertion order and reject duplicate names where Compose mapping
syntax would otherwise overwrite intent. Singleton service fields reject a second assignment.

`GeneratedService::set_annotations` distinguishes omission from an explicit empty map and accepts
only unique resolved names with explicit string values. Output is quoted, sensitivity propagates,
and the native annotation model must parse it back successfully.

Strings are always double-quoted by the private renderer, so YAML scalar inference cannot change
their type.

`GeneratedService::set_cap_add` and `GeneratedService::set_cap_drop` configure their complete
vectors independently and exactly once. Not calling a setter omits its field; an empty vector emits
the corresponding `[]`. Non-empty items retain order and exact case and render as quoted sequence
strings. Empty strings, NUL, carriage return, line feed, and exact case-sensitive duplicates are
rejected. Case variants and other single-line schema strings, including strings containing spaces,
remain accepted. Generation does not invent a capability whitelist, lowercase values, apply
target/runtime policy, or reconcile additions with drops.

`GeneratedService::set_devices` configures the complete device vector exactly once. Not calling it
omits the field; an empty vector emits `devices: []`. `GeneratedDevice` retains mixed short/long
order and exact duplicates, and `GeneratedLongDevice` requires `source` while optionally retaining
raw `target` and `permissions`. All emitted values are quoted resolved single-line strings; NUL,
line breaks, dollar-bearing deferred output, empty short items, and empty long sources are rejected.
Parse-back must recover the same ordered forms. Generation does not inspect host devices, split
colon triples, validate CDI or permission letters, infer GPU behavior, or claim runtime access.

`GeneratedRestartPolicy` cannot represent deferred or provider-specific values because generated
documents require a reviewed semantic choice. `GeneratedService::set_restart` rejects a second
assignment instead of overwriting the first policy. The renderer quotes every policy, including
`no`, and the parse-back model must recover the same policy family and retry count.

`GeneratedHostname` is non-exhaustive and currently represents a resolved `GeneratedString`.
`GeneratedService::set_hostname` rejects empty values, every dollar-bearing expression, non-ASCII
text, overlong names or labels, empty labels, underscores, leading/trailing hyphens, and trailing
dots. Uppercase and digit-leading labels are accepted. Successful output is quoted and must parse
back to the exact resolved value. Leaving the setter unused omits the field; generation does not
invent a hostname or couple it to `container_name`.

`GeneratedPidsLimit` is non-exhaustive and represents only `Unlimited` or `Finite(String)`.
Unlimited emits unquoted `-1`. Finite spellings must be non-empty positive ASCII decimals; the
setter rejects zero (including all-zero spellings), signs, fractions, exponents, interpolation,
and arbitrary strings. Validation does not parse into a fixed-width integer, so arbitrarily large
positive values and leading zeros remain exact and parse back into `PidsLimitKind::Finite`.
Generation does not inspect cgroups, inject a default, emit an ambiguous zero state, or validate
against `deploy.resources.limits.pids`.

`GeneratedShmSize` is non-exhaustive and represents an explicit `GeneratedString` amount plus one
typed documented unit from `b`, `k`, `kb`, `m`, `mb`, `g`, or `gb`. The amount must match
`[1-9][0-9]*`; this rejects zero, leading zeros, signs, fractions, exponents, whitespace,
expressions, and non-ASCII digits without fixed-width parsing. Because the unit is mandatory and
typed, successful output cannot be a bare number or use uppercase or IEC units. The renderer
always quotes the combined value, and parse-back must recover the exact amount, unit, YAML string
category. Caller-marked sensitivity still propagates to the generated document. Leaving the setter
unused omits `shm_size`; generation does not inject
Podman's 64 MiB default, normalize a provider-dependent value, inspect `/dev/shm`, or encode IPC,
pod-grouping, cross-format, CPU, or memory policy.

`GeneratedMemLimit` is a distinct non-exhaustive service-memory type with its own `MemLimitUnit`.
It emits only a quoted positive `[1-9][0-9]*` amount plus `b`, `k`, `kb`, `m`, `mb`, `g`, or `gb`,
rejects duplicate assignment and unsafe values, and requires native parse-back to recover the exact
parts. Omission remains omission. Generation does not normalize units, infer deploy consistency or
defaults, inspect a host/cgroup, enforce runtime policy, or conflate memory with shared memory,
reservation, swap, or deploy limits. Only explicit `b` values can be candidates for exact
cross-format treatment; other units and authored zero/deferred/bare/provider forms are loss-aware.

`GeneratedTmpfs` is non-exhaustive and preserves the selected scalar or ordered list form,
including an explicit empty list and exact duplicates. Each item must be a non-empty
`<path>[:<options>]` string without interpolation or line breaks. `mode`, `uid`, and `gid`
assignments are documented Compose options; other well-shaped raw assignments or flags remain
exact so a caller can retain target-only evidence. The renderer quotes every item and parse-back
recovers its exact form, spelling, order, and duplicates. Leaving the setter unused omits `tmpfs`;
generation neither selects mount defaults nor conflates this field with volume type `tmpfs`.

`GeneratedDns` and `GeneratedDnsSearch` preserve scalar/list form, explicit empty lists,
ordering, and duplicates. `set_dns_options` sets one complete unique sequence. All values must be
resolved, non-empty, and physical-line safe; no resolver grammar is applied.

`set_expose` accepts one complete sequence of unique documented decimal port/range forms with an
optional `tcp` or `udp` suffix. It does not infer a default protocol or runtime publication.

`set_security_options` preserves one complete ordered raw sequence, including exact duplicates.
Safe resolved values are quoted and parsed back without option, profile, SELinux, path, provider,
runtime, or target-format normalization.

`GeneratedSysctls` is non-exhaustive and preserves the selected ordered mapping or list form,
including explicit empty collections. `GeneratedSysctl` accepts one unique non-empty resolved map
name and one `GeneratedString` value; list items must also be resolved single-line strings and exact
duplicates are rejected. The renderer quotes mapping names, mapping values, and list items, so
`true`, `1`, and `null` remain strings on parse-back. Generation rejects NUL, multiline, and
dollar-bearing deferred forms but deliberately applies no sysctl namespace, privilege, host-kernel,
provider, runtime, or cross-format policy.

`GeneratedUlimits` preserves caller order and explicit empty mappings. Each `GeneratedUlimit`
uses either one scalar or a range with both `soft` and `hard`; all values render as quoted strings.
Names must match lowercase ASCII `[a-z]+`, remain unique, and never interpolate. Values must be a
resolved non-negative ASCII decimal or `-1`; missing range members, multiline, NUL-bearing,
dollar-bearing, provider-specific strings, and `host` are rejected. Generation injects no default,
normalizes no provider value, and makes no runtime enforcement or host-resource claim.

`GeneratedLogging` always emits one explicit quoted driver plus `options`, including an explicit
empty map. Option keys are non-empty and unique; values select quoted string, validated unquoted
YAML number, or explicit null. Ordering and sensitivity are retained. No driver vocabulary,
defaults, option meanings, provider behavior, or runtime behavior are inferred.

`GeneratedPullPolicy` is non-exhaustive and represents documented literal forms plus the retained
`if_not_present` alias. Its custom `Every(GeneratedString)` form receives the duration after the
`every_` prefix, validates integer `w`, `d`, `h`, `m`, and `s` components, and preserves exact
spelling and sensitivity. Fractions, `us`, and `ms` are rejected. Schema-valid `0s` is retained even
though its prose semantics are ambiguous. The generator deliberately does not emit schema-only
`refresh`, `pull_refresh_after`, deferred expressions, or provider-specific values, and makes no
provider or cross-format equivalence claim.

`GeneratedService::set_init` accepts an explicit boolean and rejects a second assignment. The
renderer emits an unquoted YAML boolean and parse-back validation must recover the same literal.
Leaving the setter unused omits `init`; generation does not invent `false` for an omitted choice.
The same set-once, omitted-versus-literal, deterministic parse-back boundary applies independently
to `set_stdin_open`, `set_tty`, and `set_privileged`; no terminal, security, runtime, or
cross-format policy is inferred.

`GeneratedService::set_stop_signal` accepts any NUL-free `GeneratedString`, including named,
numeric, and empty spellings, without inventing a signal grammar. The quoted empty spelling remains
distinct from null. `set_stop_grace_period` applies a `ComposeLens` raw-preserving policy based on
the documented `us`, `ms`, `s`, `m`, and `h` units, including composite, `0s`, and fractional
values. Consistent with existing native-field conventions, any retained scalar containing `$` is
classified as interpolation-shaped; that lexical state does not prove that interpolation is
eligible or valid. Both setters reject duplicate assignment and retain the exact caller spelling
and sensitivity. The renderer does not normalize values into target-runtime seconds.

`GeneratedResource::set_custom_name` emits Compose's top-level `name:` field. Runtime migration
uses it when an application-owned observed network or volume must keep its exact platform name;
without that field Compose would derive a new project-scoped name. Lifecycle ownership and exact
runtime naming remain separate choices.

`GeneratedNetworkDefinition` is a distinct additive API for top-level networks that need optional
opaque `driver`, ordered unique `driver_opts`, literal `enable_ipv6` and `internal` choices, or
ordered unique string-valued `labels`;
`GeneratedResource` remains the basic/external network and volume API. Each boolean preserves
omission versus explicit `false` or `true`; no default is injected. Driver option values explicitly
select quoted string or validated unquoted number YAML shape, so text such as `"2"` is never
conflated with numeric `2`. Empty options remain explicit when selected. Definitions are
application-owned because Compose external networks may only configure `name`; use
`GeneratedResource::external` for that compatible path. Generation does not validate driver,
IPAM, plugin, provider, runtime, or option semantics, and does not generate `enable_ipv4`.
Network labels are set once and retain omission versus an explicit empty mapping. They reuse
`GeneratedLabel`, emit deterministic mapping syntax, and do not represent key-only, null, number,
boolean, provider-injected, or runtime-equivalent labels. Resolved unique `key=value` labels are
the cross-format exact subset; Compose-native construction retains `GeneratedLabel`'s established
acceptance contract.

`GeneratedNetworkAttachment` always emits long service-network syntax. Aliases retain order, and
optional `ipv4_address` and `ipv6_address` values remain attached to that named network in a fixed
field order. Address setters preserve omission and caller spelling through `GeneratedString`,
reject duplicate assignment, and deliberately perform no IP, IPAM-pool, provider, or runtime
validation.

`GeneratedVolumeDefinition` uses the same set-once ordered mapping contract for volume `labels`
through `GeneratedLabel`. Omission remains distinct from `labels: {}`, duplicate names fail before
rendering, and explicit string values are quoted deterministically before parse-back validation.
One sensitive label value redacts generated debug output. The compatible
`GeneratedResource::external` path remains the sole external-volume lifecycle API. Authored literal
`external: true` with any labels attribute, including `{}` or `[]`, retains the labels and emits
`compose.volume.external-labels-configuration`; it does not suppress the independent
driver-configuration diagnostic.

## Short and long syntax

The builder does not treat Compose short and long forms as universally interchangeable. TCP/UDP
ports, ordinary binds, named volumes, and anonymous volumes use explicit long syntax. Long-form
`published` values are quoted strings as required by the Compose specification. SCTP ports use
short syntax because that form permits platform-specific protocols while the long form defines
only TCP and UDP. An SCTP host address therefore requires an explicit published port so its short
form remains unambiguous. A bind carrying
`GeneratedSelinux` uses short syntax because current Compose behavior does not make the long
`bind.selinux` spelling equivalent for this migration requirement. The builder rejects `:` in a
source or target that would make this selected short form ambiguous; it does not discard the
relabel request or silently switch forms.

Environment and `extra_hosts` use quoted sequence short forms. That preserves insertion order and
allows repeated effective environment names without constructing a duplicate-key YAML mapping.
Environment files retain the caller-selected form: `GeneratedEnvironmentFile::short` emits a
quoted path item, while `GeneratedEnvironmentFile::long` emits a mapping with only the explicitly
selected `required` and `format: raw` options. Neither constructor reads or resolves its path.
Labels use quoted mapping syntax because label names must be unique and explicit mapping values
represent empty strings and embedded `=` characters without short-form ambiguity.

## Sensitivity and explicit output access

`GeneratedString::sensitive` marks values whose contents must not appear in `Debug`. Sensitivity
propagates through the service, builder, and final document. `GeneratedComposeDocument::text` is
the explicit access boundary for deployable bytes. The parse-back model is also hidden from the
final document's debug representation when any generated value is sensitive.

Generated label values use this same sensitivity boundary. Label names remain visible identifiers;
the builder rejects empty/NUL-bearing names and duplicate names but does not enforce Docker's
non-binding reverse-DNS naming recommendations.
Generated environment-file paths also use `GeneratedString`; one sensitive path redacts the
complete builder and generated document from `Debug`.
Generated lifecycle values use the same boundary, so caller-marked stop signals or grace periods
redact the complete generated document while remaining available through explicit output access.
The custom pull-policy duration uses the same boundary and redacts the builder and generated
document when caller-marked sensitive.
Generated hostnames use the same boundary, so a caller-marked sensitive hostname redacts the
builder and generated document while remaining available only through explicit output access.
Generated shared-memory amounts use the same boundary, so a caller-marked sensitive amount redacts
the builder and generated document while exact deployable text remains explicitly accessible.
Generated `tmpfs` items use the same boundary; one sensitive path or raw option redacts the builder
and generated document while explicit output access retains the exact scalar/list value.
Generated device short strings and long source/target/permissions use the same boundary; one
sensitive member redacts the builder and complete generated document without entering diagnostics.
Generated DNS server strings use the same boundary; one sensitive item redacts the builder and
generated document while explicit output access retains exact scalar/list form and duplicates.
Generated sysctl mapping values and list items use the same boundary; one sensitive value redacts
the builder and generated document while map names remain ordinary uninterpolated keys.
Generated ulimit values use the same boundary; one sensitive single, soft, or hard value redacts
the builder and generated document while limit names remain ordinary uninterpolated keys.
Generated capability-add and capability-drop items use the same boundary; one sensitive item
redacts the builder and generated document while explicit output access still returns the exact
ordered strings.
Generated per-network IPv4 and IPv6 addresses use the same boundary; one sensitive address redacts
the builder and generated document while deployable text remains available explicitly.
Generated network drivers and driver-option values use the same boundary; one sensitive value
redacts the builder and generated document while deployable text remains available explicitly.
Generated volume drivers and their distinct driver-option values use the same boundary. External
volumes retain the compatible `GeneratedResource::external` lifecycle API and cannot be configured
with drivers through the application-owned volume-definition API.
Generated volume-label values use the same boundary; one sensitive label redacts the complete
generated document's debug output while explicit text access remains available.

## Validation boundary

Parse-back validation proves that ComposeLens can read the exact bytes it generated and recover
the supported native forms. It does not establish that every Compose provider or container
runtime version implements those forms equivalently. Callers still run the separate compatibility
profile appropriate for Docker Compose, `podman-compose`, and the selected backend.

The builder does not perform interpolation, merge files, select profiles, apply defaults, resolve
paths, or validate source references. Those remain explicit processing stages for authored input.
See [ADR 0017](decisions/0017-parse-back-validated-compose-generation.md).
