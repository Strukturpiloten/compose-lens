# Phase 2 typed Compose model

Phase 2 is complete for the first BoxFerry Compose-to-Quadlet conversion boundary. It is a
source-aware native Compose model, not a claim that every field in the Compose Specification
is already typed.

Fields outside this boundary remain available through the loss-aware syntax document and as
source-spanned unknown-field references. Adding another field must not require redesigning the
Phase 2 representation rules.

## Parse contract

`ComposeDocument::parse` extracts typed data from a `SyntaxDocument` and returns recoverable
diagnostics plus partial data. It does not:

- interpolate environment expressions;
- read the process environment;
- select profiles;
- apply Compose defaults;
- merge multiple files;
- normalize short syntax into long syntax; or
- validate runtime support in Docker Compose or Podman Compose.

Those operations belong to Phase 3 processing and compatibility profiles.

Multi-file consumers use `build_project_view` after merge and optional profile selection. That
operation types effective values directly and wraps them in `ProjectValue<T>` with complete
multi-file provenance. It never renders canonical YAML and reparses it as a single document. See
[ADR 0016](decisions/0016-native-merged-project-view.md).

The merged-project view exposes `extra_hosts` in both sequence and mapping form. Each effective
entry retains its hostname key sources, address provenance, authored collection form, raw IPv4 or
IPv6 spelling, and a distinct `host-gateway` classification. This is additive to the
single-document `ExtraHosts` model and lets adapters preserve explicit runtime mappings.

The merged-project view also exposes `healthcheck` without collapsing scalar/list command forms.
Every effective timing, retry, disable, and command field has its own merge provenance. Compose
`start_interval` remains distinct from provider-specific startup-healthcheck features.

Service `entrypoint` is native at both document and merged-project layers through a distinct
`Entrypoint` type. Explicit null, scalar, list, empty scalar, and empty list forms remain distinct.
It is not represented as `Command`: null selects the image entrypoint, empty values clear it, and
Compose replaces rather than appends the complete value during multi-file merge.

Service `init` is a source-aware boolean at both layers. Literal `true`/`false` and deferred
interpolation remain distinct, omission remains omitted, and complete-value replacement retains
every contributing source. ComposeLens does not invent a default or select or inspect the
platform-specific init binary.

Execution identity and context are native at both document and merged-project layers. Effective
`user`, `userns_mode`, `group_add`, `working_dir`, and `read_only` values retain raw spelling,
field provenance, and, for supplementary groups, per-item provenance. No account, group, path, or
namespace lookup is performed.

Service `cap_add` and `cap_drop` are independent optional source-aware YAML sequences of string
scalars. Omission differs from an explicit empty sequence. Every valid item retains exact text,
order, span, and sensitivity; booleans, numbers, nulls, mappings, and nested sequences diagnose
without erasing valid sibling items or service fields. Exact duplicates violate schema
`uniqueItems` and remain available with source-spanned diagnostics. Ordinary multi-file merge
appends and then removes exact case-sensitive duplicates while combining provenance. `!reset`
produces an explicit empty value, and `!override` replaces the complete sequence without
deduplicating its replacement. A lexical future-conversion helper classifies only non-empty
whitespace-free strings as exact candidates; it does not lowercase, consult a capability
whitelist, apply target policy, or reconcile additions with drops.

Service `devices` is an optional source-aware sequence whose omission differs from an explicit
empty sequence. Items retain ordered mixed syntax: raw string short forms are conservatively
classified only as path-like, CDI-like, deferred, or opaque, while long mappings require a string
`source` and optionally retain raw string `target` and `permissions`. Exact duplicates, spans,
extensions, unknown fields, and malformed-item recovery remain visible. The effective view adds
field, item, nested-member, and contributor provenance plus sensitivity. Existing target-keyed
replacement for path forms, `!reset`, and `!override` is preserved; current Compose prose and
Compose-Go `extends` metadata disagree on whether `devices` is an ordinary append exclusion.
ComposeLens does not inspect host devices, parse colon triples, validate permissions or CDI, infer
GPU meaning, or claim runtime access.

Service `dns` and `dns_search` retain optional scalar/list syntax, explicit empty lists,
exact strings, duplicates, spans, provenance, and sensitivity. List merging appends while
cross-form updates replace. `dns_opt` retains one ordered sequence with whole-sequence replacement.
Resolver grammar and runtime meaning remain outside the model.

Service `expose` retains ordered YAML string/number identity and classifies documented decimal
port/range forms without integer conversion. Unsupported, deferred, and malformed values remain raw
and source-addressable.

Service `security_opt` retains an optional ordered raw sequence with append/reset/override
provenance. Exact AppArmor, no-new-privileges, seccomp, SELinux-label, Mask, and Unmask shapes are
independent diagnostic candidates; near misses and conflicts remain unselected. The model validates
no profile, policy, path, filesystem, provider, runtime, or cross-format semantics.

Service `annotations` retains mapping/list syntax, scalar evidence, raw list items, and keyed
effective contributors. Mapping keys do not interpolate, and key-only list items remain explicit
ambiguity rather than becoming empty label values.

Service `logging` retains an optional uninterpreted YAML string `driver`, an ordered `options`
mapping including explicit empty state, and exact string/number/null option kinds. Option keys are
non-empty and never interpolate; extensions, unknown fields, malformed entries, and valid siblings
remain source-addressable.

An explicit `container_name` is a source-aware scalar at both layers. The effective project view
retains ordinary Compose scalar replacement provenance across files. The parser does not confuse
the custom runtime name with the service key or infer one when the field is absent.

Service `hostname` is a separate source-aware YAML string scalar at both layers. Its exact value
and span are retained. Any scalar containing `$` is deferred; resolved literals are conservatively
validated as ASCII RFC-1123 hostnames with a total length of 1 through 253 and dot-separated labels
of 1 through 63 characters. Labels may contain ASCII letters in either case, digits, and interior
hyphens, and must start and end alphanumeric. Invalid literals remain inspectable with diagnostics;
null, boolean, numeric, mapping, and sequence shapes diagnose without deleting their service.
Omission stays omitted, and no value is derived from `container_name`, service keys, or runtime
state.

Service `pids_limit` is raw-preserving at both layers and remains separate from
`deploy.resources.limits.pids`. Omission stays omitted; `-1` is `Unlimited`; positive ASCII-decimal
spellings are `Finite`; and all-zero spellings are a distinct ambiguous and unportable `Zero`
state. Finite values retain their complete decimal spelling and are never parsed into a fixed-width
integer, so leading zeros and values beyond `u64` remain lossless. Interpolation-shaped strings are
deferred. Fractions, signs other than the exact `-1`, exponents, and arbitrary strings remain
`Other` with diagnostics. YAML booleans, null, mappings, and sequences are rejected as field forms
without deleting their service. ComposeLens injects no default and performs no runtime or cgroup
inspection.

Service `shm_size` is raw-preserving at both layers and remains separate from `build.shm_size`,
IPC and pod grouping, CPU or memory limits, and runtime `/dev/shm` inspection. YAML number and
string scalars are accepted and retain their exact value, span, and scalar category. Dollar-bearing
strings are deferred. Strings ending in the documented lowercase `b`, `k`, `kb`, `m`, `mb`, `g`,
or `gb` family expose that unit and retain the complete `amount_raw` without imposing an integer,
fraction, sign, or leading-zero grammar that Compose does not define. All-zero integral spellings
remain a distinct ambiguous state. Other schema-accepted numbers and strings remain separate
provider-dependent states with actionable diagnostics. Null, booleans, mappings, and sequences
diagnose without deleting their service. Omission stays omitted; ComposeLens does not synthesize
Podman's 64 MiB default, normalize units, or parse values into a fixed-width integer.

Service `mem_limit` is independently raw-preserving and remains distinct from `mem_reservation`,
`memswap_limit`, deploy resource memory, and `shm_size`. YAML number and string scalars retain exact
text, span, and scalar category. Dollar-bearing strings are deferred; documented lowercase `b`,
`k`, `kb`, `m`, `mb`, `g`, and `gb` suffixes retain an unconstrained raw amount; lexical zero,
schema-only numbers, and other provider-dependent strings remain distinct with recoverable
diagnostics. ComposeLens does not normalize units, parse a machine integer, reconcile deploy
values, inspect host/cgroup state, or claim non-byte values are exactly transferable.

Service-level `tmpfs` is distinct from long-syntax volume type `tmpfs` and retains omission,
scalar/list form, explicit empty lists, ordering, exact duplicates, source spans, and sensitivity.
Each exact string uses `<path>[:<options>]`: a non-empty path alone or colon-delimited non-empty
`mode`, `uid`, and `gid` assignments is `Documented`; dollar-bearing values are `Expression`;
other raw or malformed options are retained as `ProviderDependent` with an actionable diagnostic.
No path or option normalization occurs. Ordinary list-to-list multi-file merge appends without
deduplication; scalar/list mismatches replace normally, while `!reset` and `!override` remain explicit.

Service `sysctls` retains mapping versus list syntax, including explicit empty collections.
Mapping keys are non-empty literal strings and remain uninterpolated; values retain exact YAML
string, number, boolean, or null kind and spelling. List items retain exact string spelling,
ordering, spans, interpolation sensitivity, and duplicate evidence. Invalid keys, values, items,
and collection forms diagnose without erasing valid siblings. ComposeLens does not interpret
namespaces, privileges, kernel availability, or runtime coercion.

Service `ulimits` is mapping-only and accepts an explicit empty mapping. Names follow lowercase
ASCII `[a-z]+` and do not interpolate. Each ordered entry retains either one number/string scalar
or a soft/hard mapping whose members are both required; scalar spelling, value span, and malformed
siblings remain source-aware. Existing authored `Ulimits`, `Ulimit`, `UlimitValue`, `UlimitRange`,
and `LimitValue` APIs remain unchanged. The effective project types additionally retain outer-key,
field, entry, and range-member provenance, authored versus interpolated scalar spelling, YAML
number/string kind, sensitivity, omission, explicit empty/reset mappings, recursive merge,
scalar/range replacement, and override without applying runtime semantics.

Service-level `restart` is raw-preserving at both layers. Known policies are classified as `no`,
`always`, `on-failure[:max-retries]`, and `unless-stopped`; a deferred interpolation remains
distinct, and invalid/provider-specific values remain available with diagnostics. The optional
decimal retry spelling is not normalized, so an authored `on-failure:003` remains exact.
This field is separate from `depends_on.<service>.restart`, which describes an explicit
Compose-controlled dependency update, and from `deploy.restart_policy`.

Service `pull_policy` is raw-preserving at both layers. `always`, `never`, `missing`, `build`,
`daily`, `weekly`, the `if_not_present` alias, and valid `every_<duration>` values receive distinct
classifications without changing caller spelling. Custom intervals match the schema grammar
`every_([0-9]+[wdhms])+`: integer week, day, hour, minute, and second components can be combined;
fractions, `us`, and `ms` are retained as `Other`. `every_0s` is schema-valid and therefore remains
`Every`, while its prose semantics stay explicitly ambiguous. Interpolation-shaped values remain
deferred. The schema-only `refresh` spelling is classified separately from both documented forms
and invalid/provider-specific values; that classification is not a provider support claim.
`pull_refresh_after` remains an unknown/unmodeled field with source and merge provenance until its
relationship to `refresh` has a separate native contract and exact provider evidence.

Service `stop_signal` and `stop_grace_period` are independent optional values at both layers.
Signals retain the complete scalar without imposing a token grammar not defined by Compose. A
quoted empty signal remains distinct from a missing or null field. `StopGracePeriod` retains its
authored scalar as a policy-accepted value, interpolation-shaped value, or explicit
invalid/provider-specific value. ComposeLens's raw-preserving policy uses the documented `us`,
`ms`, `s`, `m`, and `h` units and accepts composite, zero-with-unit, and fractional segments;
health-check-only `ns`, `µs`, and `μs` acceptance is not inherited. Following existing typed-field
conventions, any retained scalar containing `$` is classified as interpolation-shaped. This is a
lexical classification, not proof that the scalar is eligible for or contains a valid Compose
interpolation expression. No target lifecycle normalization is performed.

Service `env_file` is native at both layers without performing file I/O. A lone scalar and each
ordered sequence item retain short syntax; mapping items retain long syntax with source-aware
`path`, `required`, and `format`. `required` distinguishes a literal boolean from deferred
interpolation. `format` retains its complete scalar while classifying `raw`, a deferred
expression, or an invalid/provider-specific value. The effective project view preserves ordinary
sequence append order plus collection, item, and nested-field provenance. Relative-path
resolution, file existence, and parsing file contents remain caller-owned operations.

Service config and secret grants are native at both layers. The merged-project view retains short
versus long syntax, collection/item provenance, and separate provenance for long-form `source`,
`target`, `uid`, `gid`, and `mode`. This preserves Compose's unique-by-target merge behavior,
including nested values inherited from an earlier file.

Compose processing tags `!reset` and `!override` remain intact in the syntax document. The typed
parser reads their inner value when it matches a typed field, allowing valid override documents to
participate in loading without incorrectly reporting the wrapper as a field-type error. The merge
stage, not the typed parser, applies the tag's semantics.

## Typed boundary

| Location | Phase 2 fields |
| --- | --- |
| Document | `name`, `services`, `networks`, `volumes`, `configs`, `secrets` |
| Service | `hostname`, `container_name`, `image`, `build`, `entrypoint`, `command`, `init`, `environment`, `env_file`, `labels`, `annotations`, `logging`, `extra_hosts`, `user`, `userns_mode`, `group_add`, `cap_add`, `cap_drop`, `devices`, `dns`, `dns_opt`, `dns_search`, `expose`, `security_opt`, `working_dir`, `read_only`, `pids_limit`, `shm_size`, `mem_limit`, `tmpfs`, `sysctls`, `pull_policy`, `restart`, `stop_signal`, `stop_grace_period`, `ulimits`, `depends_on`, `healthcheck`, `deploy`, `ports`, `volumes`, `networks`, `profiles`, `configs`, `secrets` |
| Network definition | `driver`, `driver_opts`, `attachable`, `enable_ipv4`, `enable_ipv6`, `external`, `internal`, `ipam`, `labels`, `name` |
| Volume definition | `driver`, `driver_opts`, `external`, `labels`, `name` |
| Config definition | `file`, `environment`, `content`, `external`, `name` |
| Secret definition | `file`, `environment`, `external`, `name` |

Every implemented mapping retains `x-` extensions and unrecognized fields with source spans.
Collections retain authored order.

## Representation fidelity

Field-specific variants retain forms whose behavior or meaning can differ:

- entrypoint and command: independent explicit null, scalar, or list values, including empty
  scalar and empty list;
- init: a literal boolean or deferred interpolation expression;
- capability additions and drops: independent omitted or explicit ordered string sequences with
  exact duplicates retained for diagnostics and case variants kept distinct;
- service devices: an omitted or explicit ordered sequence of mixed raw short strings and long
  mappings with required `source`, optional raw `target`/`permissions`, exact duplicates, and
  retained CDI/deferred/opaque evidence;
- service DNS servers: an omitted raw scalar or ordered list, including explicit empty lists and
  exact duplicate strings;
- environment: list or mapping, including `NAME`, `NAME=`, empty strings, and null values;
- environment files: a scalar path, ordered path list, or ordered long entries with `path`,
  `required`, and `format` retained independently;
- ports: scalar short syntax or mapping long syntax;
- service volumes: scalar short syntax or mapping long syntax;
- service networks: name sequence or options mapping;
- service config and secret grants: name short syntax or mapping long syntax;
- extra hosts: hostname/address sequence or mapping, retaining delimiters and IPv6 brackets;
- service hostname: resolved RFC-1123 literal, deferred dollar-bearing expression, or retained
  invalid string literal;
- dependencies: service-name sequence or condition/options mapping;
- health-check tests: shell-command scalar or tokenized list;
- PID limit: unlimited, arbitrary-precision positive decimal, ambiguous zero, deferred expression,
  or retained unsupported scalar;
- shared-memory size: exact YAML number/string provenance, documented lowercase unit plus raw
  amount, ambiguous zero, deferred expression, or distinct provider-dependent number/string state;
- service temporary filesystems: omitted, scalar, or ordered list form with explicit empty list,
  exact duplicates, colon-delimited documented assignments, and retained raw target options;
- service sysctls: omitted, ordered mapping or ordered list form with explicit empty collections,
  exact scalar kind/spelling, uninterpolated map keys, and retained duplicate list evidence;
- service logging: omitted or mapping form, optional uninterpreted string driver, ordered
  string/number/null options including explicit empty state, extensions, unknowns, and malformed
  sibling recovery;
- restart policy: a known literal, optional raw-preserving `on-failure` retry count, deferred
  expression, or retained invalid/provider-specific scalar;
- pull policy: documented literals, a retained alias, exact custom interval spelling, deferred
  expression, schema-only refresh classification, or retained invalid/provider-specific scalar;
- stop lifecycle: an unconstrained raw signal scalar and an independent raw-preserving Compose
  duration value/expression/other state;
- ulimits: an ordered lowercase-name mapping with one number/string scalar or required separate
  soft/hard values, including an explicit empty mapping;
- build: scalar context or a mapping of independently identified specification fields;
- service and resource labels: list or mapping. Service-label list entries retain the complete
  scalar so values containing additional `=` characters are not truncated.

The raw short volume and port strings remain authoritative. Conservative helper parsing must not
turn platform-dependent path or address grammars into false certainty. ADR 0003 defines the full
[syntax-form policy](decisions/0003-preserve-compose-syntax-forms.md).

Container-side mount targets additionally expose a lexical `ContainerPath` classification for Unix
absolute, Windows drive-letter, Windows UNC, relative, and deferred paths. This classification is
independent of the machine running ComposeLens. Host bind sources continue through the separate
host-path resolver with explicit origins and home context.

## Issue-derived model expansion

Raw `user` values are authoritative. User/group helpers recognize names, numeric IDs, empty
components, and deferred interpolation without resolving accounts. The split ignores colons inside
`${VAR:-default}`, avoiding a common false decomposition. `ulimits` recognizes authored `-1` as an
explicit unlimited value in both scalar and soft/hard forms. Its generated boundary deliberately
narrows output to quoted `-1` or non-negative ASCII decimals and never emits `host`, arbitrary
schema strings, or provider defaults.

`service_healthy`, `service_started`, and `service_completed_successfully` dependency conditions
are native types. `ComposeDocument::validate_dependencies` checks one document; post-merge
`validate_references` checks the selected project. A missing Compose health check produces a
warning because the image may contain health metadata. An explicitly disabled health check is an
error for a required `service_healthy` edge. `required: false` downgrades unavailable dependency
diagnostics to warnings without hiding the reference.

Build and deploy definitions expose every current top-level subfield as its own stable kind and
source reference. This is intentionally field-level evaluation, not a claim that every nested
value is already semantically typed or supported by every converter.

Podman `keep-id`, `auto`, and `nomap` user-namespace values and the `host-gateway` token have native
classifications. Their compatibility findings cite official implementation documentation and do
not promote untested provider pass-through to supported. See
[ADR 0014](decisions/0014-issue-derived-native-model-expansion.md).

## Deferred values

Interpolation is not parsing. Boolean-capable fields therefore distinguish a YAML boolean literal
from a deferred scalar expression such as `${EXTERNAL:-false}`. Environment and option mappings
retain null, boolean, numeric, and string scalars without applying interpolation or coercion.

Image references are intentionally tolerant. A combined tag and digest such as
`registry.example/app:1.2@sha256:abcdef` is accepted and retains its complete raw value. ComposeLens
does not reject a real implementation-supported reference merely because an unrelated stricter
grammar would reject it.

## Empty YAML values

An omitted mapping value, for example `INHERITED:` in an environment mapping or `cache:` in a
top-level volume collection, is an explicit null-like authored value. The typed extractor uses
source columns to recover same-level sibling entries when the private YAML dependency nests them
under that empty value. Recovery crosses the private tree boundary when an empty value is the last
entry of a child mapping, so later parent fields are not silently reparented. Regression fixtures
require the empty entry and all following siblings to remain independently visible.

## Diagnostics and evidence

Invalid collection shapes, invalid short/long alternatives, incomplete long forms, duplicate
fields, and invalid typed scalars produce stable diagnostic codes and source labels. Extraction
continues wherever partial data remains useful.

The authored `typed-model` fixtures cover valid Phase 2 forms, issue-derived post-0.1 forms,
raw and malformed PID limits, shared-memory sizes, and service temporary filesystems, invalid recoverable forms, combined
image tags and digests, deferred expressions, empty values, extension fields, unknown fields, and
the short/long service-volume SELinux asymmetry.

Primary format references:

- [Compose Specification](https://github.com/compose-spec/compose-spec/blob/main/spec.md)
- [Docker Compose services reference](https://docs.docker.com/reference/compose-file/services/)
- [Docker Compose networks reference](https://docs.docker.com/reference/compose-file/networks/)
- [Docker Compose volumes reference](https://docs.docker.com/reference/compose-file/volumes/)
- [Docker Compose configs reference](https://docs.docker.com/reference/compose-file/configs/)
- [Docker Compose secrets reference](https://docs.docker.com/reference/compose-file/secrets/)
- [Docker Compose build reference](https://docs.docker.com/reference/compose-file/build/)
- [Docker Compose deploy reference](https://docs.docker.com/reference/compose-file/deploy/)
- [Podman 5.4 run reference](https://docs.podman.io/en/v5.4.0/markdown/podman-run.1.html)
