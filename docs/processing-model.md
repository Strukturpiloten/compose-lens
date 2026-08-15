# Processing model

## Principle

A Compose file is not the same thing as a fully resolved Compose project. ComposeLens represents the intermediate stages so callers retain control over environment access, merging, profile selection, defaults, and normalization.

## Core representations

### Syntax document

Represents what was written, including source spans and unrecognized constructs. It is the basis for preservation-oriented editing and precise diagnostics.

### Typed document

Represents one Compose document with native Compose types. Values may still contain interpolation expressions, relative paths, implementation extensions, and unresolved references.

### Loaded project

Represents an ordered collection of documents plus their origins. The implemented loader retains
each document's display label and directory, requires unique caller-managed source IDs, and uses the
first document's directory as the multi-file project base. It does not imply that any optional
transformation has run.

### Semantic view

Represents a chosen interpretation of the project for a particular implementation profile and processing context. It records which operations and defaults were applied.

### Native project view

Represents the effective first-conversion fields of a merged and optionally profile-selected
project as native Compose values. `ProjectValue<T>` retains the complete merge operation, every
contributing span, the effective source, and sensitivity state. It is a consumer boundary, not a
replacement for the generic merged tree.

## Explicit operations

### Load

Accepts named source texts and origins supplied by the caller. The core loader performs no file or
environment access. Recoverable syntax and typed-model diagnostics stay attached to their document
and are aggregated without preventing analysis. File discovery and I/O belong in application
adapters; paths retain the origin needed for later resolution.

### Traverse and compose includes

`IncludeResolution::load` accepts a caller-created root and an `IncludeLoader`. For each reached
project it performs the existing ordered load, authored no-interpolation merge, and native project
view before visiting effective includes depth-first. The loader receives raw path, `env_file`, and
`project_directory` declarations plus their spans, origin, and parent context; it is the only I/O
and authorization boundary. Cycles use caller-defined identities on the active stack, source IDs
are unique across the whole traversal, and failures retain a partial graph with stable
`compose.include.*` diagnostics.

`IncludeResolution::compose` is an independent, I/O-free operation over those retained views. It
composes children before importing them into their already merged parent. It imports only absent
names in each of the services, networks, volumes, configs, secrets, and models namespaces. A local
or earlier child selection wins; a same-name incoming candidate is retained as an explicit conflict
with two source labels and never runs the ordinary merge rules. Its result retains the traversal
diagnostics plus warnings and reports incomplete traversal or conflicts explicitly. It still does
not canonicalize or join paths, interpolate values, read environment or `.env` files, infer project
names, cache diamonds, render a composed document, or select provider behavior.

### Plan include project directories

`IncludeResolution::plan_project_directories` is an independent I/O-free planning operation. Root
and undeclared child occurrences use their retained first-document directories. For an explicit raw
`project_directory`, a caller-owned resolver receives the include edge, request, occurrence
identities/indices, declaration span, effective parent directory when available, and child first
document directory. It can return an authorized directory, defer non-fatally, or report typed
unresolved status. The plan preserves traversal diagnostics, emits no path text, and does not join,
canonicalize, open, interpolate, expand, or otherwise interpret paths. Cycle edges are skipped.

### Resolve selected included resource paths

`resolve_included_resource_paths` combines an `IncludeCompositionResult`, its matching
`IncludeProjectDirectoryPlan`, and caller-owned `PathContext`. It visits the selected top-level
config and secret `file` values in deterministic namespace order and applies the existing lexical
relative, Unix absolute, Windows drive, UNC, and home-relative classifications. Each result retains
the exact supplying occurrence and its authorized base.

Occurrence index and identity must agree with the directory plan. Missing, deferred, unresolved,
or mismatched entries stay unresolved with stable diagnostics; no root or parent fallback is used.
The operation is authored and uninterpolated and performs no file access, canonicalization,
existence check, URI interpretation, or resolution of another path family. Path and identity text
is always redacted from debug output.

### Interpolate

Evaluates supported variable expressions using an explicit provider. The provider may expose process environment variables, a supplied map, an `.env` document, or a test fixture. No provider is consulted during parsing.

The result retains a distinction between the original expression and the resolved value. Sensitive values are redacted from diagnostics.

The implemented interpolation kernel supports direct, default, required, alternative, nested, and escaped-dollar expressions. `EmptyEnvironment`, `MapEnvironment`, and the public `EnvironmentProvider` trait keep variable access explicit. A provider value can be marked sensitive; that classification propagates only when its content enters the result. Diagnostics never contain resolved values or the caller-authored message operand of a required expression.

The document overlay applies the kernel to unquoted and double-quoted YAML values while leaving mapping keys and single-quoted values unchanged. A loaded project can create one overlay per file in input order. A future application-level environment loader will construct provider precedence from explicit caller inputs and the selected implementation profile; merge still occurs only after interpolation.

### Merge

Combines loaded documents into a parser-independent semantic tree. The implemented merge retains
source spans and classifies authored, added, replaced, recursively merged, appended, reset, and
overridden values. It handles ordinary mappings and sequences, shell-command replacement,
environment and label keys across map/list forms, unique service resources, YAML merge keys,
`!reset`, and `!override`.
Service `cap_add` and `cap_drop` independently use exact-scalar sequence uniqueness during ordinary
multi-file merge: values append in order and exact case-sensitive duplicates collapse with
combined provenance. Case variants stay distinct. `!reset` yields an explicit empty sequence,
while `!override` replaces the whole sequence and preserves duplicates inside the replacement for
diagnostics. No cross-field reconciliation is performed.
Service `devices` keeps the existing Compose-Go-compatible target-unique rule for path forms.
Matching targets replace or recursively merge in place and retain all contributor provenance;
`!reset` yields an explicit empty sequence and `!override` replaces the complete sequence. CDI,
deferred, opaque, colon-delimited, and permissions spelling remains raw. Current Compose merge prose
does not list `devices` among the ordinary append exclusions, while Compose-Go's `extends` metadata
does; ComposeLens records that discrepancy without silently changing its compatibility behavior.
Service `dns` and `dns_search` retain scalar/list form: list-to-list updates append,
cross-form updates replace, and reset/override stay explicit. `dns_opt` uses whole-sequence
replacement. Exact values, duplicates, provenance, and sensitivity remain visible without resolver
interpretation.

Service `expose` uses exact scalar text and YAML kind for unique-sequence merging. No implicit
protocol or runtime publication is inferred.

Service `security_opt` appends raw values without deduplication. Exact AppArmor,
no-new-privileges, seccomp, SELinux-label, Mask, and Unmask shapes become independent diagnostic
candidates after interpolation; conflicts and near misses remain unselected.

Service `annotations` accepts mapping and list syntax and merges valid entries by name after each
file's value interpolation. Raw list evidence and ambiguous or malformed entries remain
source-addressable.

Service-level `tmpfs` list-to-list merging uses the ordinary append rule with no cross-file
deduplication. Scalar/list mismatches use ordinary replacement; reset and override retain their
explicit operations.
Service `sysctls` mappings use ordinary recursive mapping merge by exact key, while list-to-list
merging appends without deduplication. Map/list mismatches replace normally. Per-file interpolation
applies to mapping values and list items but not mapping keys; duplicate effective list strings are
retained with source-aware diagnostics.
Service `ulimits` uses the same generic recursive mapping merge, including independent `soft` and
`hard` members. Single/range mismatches replace normally. Limit names never interpolate; scalar and
range-member values use the explicit per-file interpolation overlay. Reset produces an explicit
empty mapping, and override replaces the complete mapping with its operation retained.

Build `ulimits` uses the identical recursive mapping rule and native scalar/range representation.
It retains case and source evidence for rejected names, scalar spelling, independent `soft`/`hard`
provenance, explicit empty/reset/override mappings, and malformed siblings without inferring
defaults, host limits, builder behavior, or runtime enforcement.

Service `logging` is likewise an ordinary recursively merged mapping. Driver and colliding option
values replace with full provenance; mapping keys remain uninterpolated while string values use
each file's explicit interpolation overlay. Reset and override retain their generic operations.

Build `platforms` is an ordinary raw scalar sequence: list values append with duplicates retained,
while reset and override remain explicit. It has no OCI grammar, availability, service-platform,
default, or build-execution interpretation.

Build `cache_from` and `cache_to` are ordinary raw string-scalar sequences: list values append
with duplicates retained, while reset and override remain explicit. Cache type, reference, source,
destination, path, image, credential, and builder semantics remain uninterpreted.

Build `entitlements` is an ordinary opaque string-scalar sequence: list values append with
duplicates and empty strings retained, while reset and override remain explicit. Values interpolate
per file before merging; ComposeLens infers no allowlist, privilege state, BuildKit/platform
support, execution, or runtime effect. Docker Compose v2.27.0 is a documented implementation
badge only; earlier and removal boundaries are unknown.

Build `dockerfile_inline` is an exact YAML string scalar: empty and multiline content remain
distinct from omission, values interpolate per file before merging, and scalar replacement plus
reset/override provenance remain explicit. ComposeLens does not parse Containerfile syntax,
resolve paths or contexts, scan content for secrets, build images, or infer Docker, BuildKit, or
runtime behavior. Docker Compose v2.17.0 is an implementation badge only; earlier and removal
boundaries are unknown.

Build `provenance` retains YAML boolean or opaque string form, including empty and interpolated
strings, with scalar replacement/reset/override provenance and malformed recovery. It does not
parse, generate, publish, or validate attestations or infer builder/runtime behavior. Docker
Compose v2.39.0 is an implementation badge only; earlier and removal boundaries are unknown.

Build `no_cache` is a scalar value with an explicit YAML boolean/string distinction. Strings,
including empty and interpolation-shaped values, interpolate per file before merging but never
coerce to booleans. Scalar replacement and reset/override provenance remain visible; no default,
builder execution, or cache behavior is inferred.

Build `sbom` is a scalar value with the same explicit YAML boolean/string distinction. Strings,
including empty, generator-shaped, and interpolation-shaped values, interpolate per file before
merging but never coerce to booleans. Scalar replacement and reset/override provenance remain
visible; ComposeLens neither parses generators nor exposes generated SBOM data or infers builder
behavior.

Build `privileged` accepts YAML literal booleans and deferred dollar expressions. Scalar
replacement and reset/override provenance remain visible. Ordinary quoted non-expression strings
are invalid schema ambiguity rather than coerced booleans, and remain source-addressable unmodeled
evidence with diagnostics. Docker Compose v2.15.0 is an implementation badge only; earlier and
removal boundaries are unknown. ComposeLens infers no privilege, platform, runtime, or build
behavior.

Build `shm_size` is a scalar value using the same raw-preserving number/string, documented
lowercase-unit, lexical-zero, deferred-expression, and provider-dependent classification as
service `shm_size`. It interpolates per file before scalar merging; replacement, reset, override,
and sensitivity remain visible. ComposeLens does not infer builder defaults, allocation, host
shared-memory state, or runtime behavior.

Build `isolation` is an opaque YAML string scalar. It interpolates per file before scalar merging
and retains replacement, reset/override, and sensitivity evidence. ComposeLens does not validate
modes, platforms, defaults, privileges, `BUILDAH_ISOLATION`, or service-level isolation behavior.

Build `ssh` accepts an ordered string sequence or a scalar mapping. Sequences append, mappings
recursively merge, mixed forms replace, and reset/override retain generic provenance. Values
interpolate per file but mapping keys do not; every effective SSH value and mapping entry is
sensitive even without interpolation and is redacted by default. ComposeLens does not parse or
access identifiers, paths, PEM material, sockets, agents, mounts, or a builder.

Build `additional_contexts` uses the generic collection rules: mappings recursively merge by exact
uninterpolated key, sequences append raw string items, mixed forms replace, and reset/override
remain visible. Values interpolate per file before merging; names, `=`, paths, URLs, images, and
`service:` schemes remain uninterpreted.

Build `extra_hosts` is independent from service `extra_hosts`: its raw-string lists append with
duplicates retained, while mappings recursively merge hostname keys and their scalar or nested-list
string addresses. Mixed forms replace; reset and override remain visible. Values interpolate per
file, keys do not, and the view performs no address normalization/validation, DNS or host lookup,
build generation, or conversion.

Top-level network `labels` retain their authored list or mapping form and use the generic
Compose Specification map/sequence merge rules, including `!reset` and `!override`. Current
Compose-Go's observed `mergeToSequence` special case is an implementation divergence, not
normative behavior; ComposeLens does not adopt it without separately versioned evidence and a
decision.

The operation accepts an optional set of matching per-file interpolation overlays. Omitting them is
an explicit request to merge authored expressions. A mismatched project/overlay pair is rejected.
Implementation compatibility classification remains a later stage. The evidence and remaining
runtime test requirements are recorded in [Compose multi-file merge evidence](research/compose-merge.md),
and ADR 0006 defines the [merge representation](decisions/0006-provenance-preserving-compose-merge.md).

### Select profiles

`select_profiles` creates a non-destructive service view from a merged project and an explicit
`ProfileRequest`. Services without an effective `profiles` restriction are active. Restricted
services are active when one of their valid names is requested, or when the caller explicitly
requests all profiles. The operation validates the Compose profile-name grammar and preserves
inactive services in the merged source.

Command-line service targeting can activate a targeted service's profiles in Docker Compose. That
is runtime-command behavior, not an implicit part of this library operation; an application must
model it as an explicit input if it needs that behavior. References do not silently activate a
profile-disabled service.

### Build the native project view

`build_project_view` consumes a `MergedProject` directly and accepts an optional matching
`ProfileSelection`. A matching selection filters inactive services; omitting it includes all
services. A selection created from a different project returns the same stable mismatch diagnostic
used by the other post-merge operations.

The native view covers project name, services, service hostnames, explicit container names, images, entrypoints,
commands, credential specifications, raw extends directives, raw provider mappings, attach, init-process, standard-input-open, terminal-allocation, and privileged choices, merged environment, service labels, extra host
mappings, service annotations, service logging configuration, ordered service capability additions and drops, ordered mixed service devices, raw service DNS servers, ordered service exposed ports, raw service security options, raw service PID limits, raw service shared-memory sizes, raw service memory limits, service-level temporary filesystems, service sysctls, ordered service ulimits, image pull policies, raw runtime strings, health checks, service dependencies, independent stop signals and stop grace periods,
ports, volume mounts, service config and secret grants, service networks, profiles, and top-level
network, volume, config, and secret definitions.
Environment and service-label values are keyed after field-specific Compose merging, while each
entry retains whether its effective spelling came from mapping, `KEY=VALUE`, or key-only list
syntax. A key-only label means an empty value; a key-only environment entry retains its distinct
host-resolution meaning. Extra-host
entries retain sequence or mapping syntax, raw address spelling, lexical address classification,
and the `host-gateway` implementation token. Values and collection items retain complete
multi-file provenance. Dependencies and grants retain effective short or long syntax and nested
field/unknown-option provenance without applying target lifecycle defaults. Unmodeled fields
retain their semantic path, all key locations, value provenance, extension classification, and
sensitivity.

This operation does not render or parse generated YAML. It also does not read files, consult an
environment provider, apply defaults, validate an implementation, or perform conversion. Native
coverage is documented in [ADR 0016](decisions/0016-native-merged-project-view.md).

Service `attach` retains literal booleans or deferred dollar expressions under ordinary scalar
replacement/reset/override merge, while invalid forms remain source-addressable evidence. It has no
default, generated, logging, runtime, provider, CLI, compatibility, or conversion interpretation.

Service `blkio_config` uses generic recursive mapping and sequence append/reset/override behavior.
Its current raw view does not resolve `extends` or claim the specification's path-keyed rate-array
inheritance; weights/rates remain integer-or-string spelling with no units, ranges, defaults,
controller, runtime, provider, I/O, or conversion interpretation.

Service `cgroup` uses ordinary scalar replacement/reset/override behavior. Its strict YAML-string
view retains `host`, `private`, dollar-bearing deferred expressions, and diagnosed `Other` values
without controller, cgroup version, rootless, systemd, provider, runtime, `extends`, generation,
or conversion interpretation.

Service `cgroup_parent` uses ordinary scalar replacement/reset/override behavior. Its strict raw
YAML-string view retains empty, whitespace, deferred, and arbitrary strings without grammar, path,
controller, host, runtime, provider, version, default, `extends`, generation, or conversion interpretation.

Service `cpu_count` uses ordinary scalar replacement/reset/override behavior. Its YAML
integer/string view retains exact spelling and interpolation sensitivity; nonnegative integers are
not normalized, while negative YAML integers diagnose as retained invalid evidence. It applies no
quota, host, scheduler, runtime, provider, OS, version, default, `extends`, generation, or conversion interpretation.

Service `cpu_percent` uses ordinary scalar replacement/reset/override behavior. Its YAML
integer/string view retains exact spelling and interpolation sensitivity; only YAML integers are
classified against the schema's inclusive `0..=100` range, while strings are never coerced or
range-checked. It applies no percentage calculation, CPU, quota, host, scheduler, runtime,
provider, OS, version, default, `extends`, generation, or conversion interpretation.

Service `cpu_period` uses ordinary scalar replacement/reset/override behavior. Its YAML
number/string view retains exact spelling and interpolation sensitivity without numeric conversion
or semantic validation. It applies no duration, microsecond, CFS, CPU, host, runtime, provider,
OS, version, default, `extends`, generation, or conversion interpretation.

Service `cpu_quota` uses ordinary scalar replacement/reset/override behavior. Its YAML
number/string view retains exact spelling and interpolation sensitivity without numeric conversion
or semantic validation. It applies no numeric quota, duration, microsecond, CFS, CPU, host, runtime,
provider, OS, version, default, `extends`, generation, or conversion interpretation.

Service `cpu_rt_period` uses ordinary scalar replacement/reset/override behavior. Its YAML-number,
duration, expression, and other-string view retains exact spelling and interpolation sensitivity;
other strings diagnose without loss. It applies no CPU calculation, microsecond conversion,
realtime scheduler, OS, host, default, provider, version, runtime, generation, or conversion interpretation.

Service `credential_spec` retains an explicit mapping with optional strict YAML-string `config`,
`file`, and `registry` members, nested provenance, reset/override recovery, and interpolation
sensitivity. It performs no top-level-config resolution, file/registry/account access, URI,
Windows/gMSA, platform, provider, runtime, or conversion interpretation.

Service `extends` retains a YAML-string short reference or a mapping with optional strict
YAML-string `service` and `file` members, nested provenance, reset/override recovery,
interpolation sensitivity, and malformed/extension/unknown evidence. Missing long-form `service`
is diagnosed without dropping the mapping. Generic scalar replacement and recursive mapping merge
apply; this stage never expands or merges a referenced service, looks up files, normalizes paths,
traverses cycles, or imports resources. Separately, `validate_references` may validate a same-file
long-form `service` edge when `file` is absent, without performing any of those operations.

Service `provider` retains a strict YAML-string `type` and an optional ordered `options` mapping
of YAML string/number/boolean scalars or sequences. Generic mapping, scalar replacement, sequence
append, reset, and override provenance remains visible; malformed siblings and items remain
evidence. This view never executes or discovers a provider, calls setup/teardown, injects
environment, resolves credentials, validates provider-specific grammar or compatibility, or
performs conversion.

Service `post_start` and `pre_stop` retain ordered hook mappings with a required null/scalar/list `command` and
optional local map/list `environment`, boolean `privileged`, strict YAML-string `user`, and strict
YAML-string `working_dir`. Generic sequence append, reset, and override provenance remains visible;
malformed items and members remain evidence. The view does not execute or schedule hooks, expand
commands, calculate environment inheritance, resolve users or paths, decide privilege, validate
provider compatibility, or perform conversion.

Service `pre_start` retains its distinct ordered hook mappings with optional null/scalar/list
`command`, strict raw `image`, optional local map/list `environment`, boolean `privileged` and
`per_replica`, and strict YAML-string `user` and `working_dir`. Generic sequence append, reset,
and override provenance remains visible; this view does not apply defaults, execute/schedule hooks,
calculate inheritance, resolve images/users/paths, or infer provider, runtime, or conversion behavior.

Service `runtime` retains strict YAML-string values, including empty and deferred spelling, with
ordinary scalar replacement/reset/override provenance. This view supplies no runtime grammar,
default, host/provider/image/platform compatibility, execution, generation, or conversion behavior.

Deploy handling currently exposes `endpoint_mode`, `mode`, `replicas`, placement, resource-limit CPUs, memory, and PIDs, resource-reservation CPUs and memory, and distinct deployment
`labels`: `vip`/`dnsrr` and
`global`/`replicated` classify directly, while other interpolated or raw strings remain a
portability diagnostic with their value and provenance. `replicas` preserves its exact YAML number
spelling or distinct string category, including empty or deferred strings, without integer,
positive/zero/default, mode-coupling, scale, allocation, scheduling, runtime, or conversion
interpretation. All current immediate deploy children are native values; malformed, extension, and
future-unknown child evidence and explicitly bounded nested resource forms remain unmodeled. The
prose names `vip` as a default while the schema does not yield an effective default; ComposeLens records
the conflict and injects neither value. In particular, `global` with `replicas` and service
`scale` does not create a consistency or scheduling inference.

Deployment labels retain mapping scalar/null categories or raw list entries separately from service
container labels. Mapping keys merge by key while lists append and preserve duplicate fallible-input
evidence; reset and override provenance remains visible. They infer no container, service, runtime,
platform, deployment, or conversion behavior.

Deploy `restart_policy` remains independent from service `restart`. Its `condition`, `delay`,
`max_attempts`, and `window` members retain their raw spelling and individual merge provenance.
`max_attempts` accepts a YAML integer or string scalar, never a floating-point number;
malformed or reset members remain nested unmodeled evidence. This view applies no fallback,
default, precedence, attempt simulation, runtime, or conversion interpretation.

Update_config is a separate recursively merged mapping with member provenance and no rollout,
scheduling, pause, rollback, monitoring, failure-rate, default, runtime, or conversion behavior.

Rollback_config is a distinct recursively merged mapping with the same raw scalar preservation and
member provenance boundary; it makes no rollout, execution, order, monitor, failure-rate, default,
scheduler, provider, runtime, version, or conversion claim.

Deploy `placement` retains ordered YAML-string constraints and preference mappings with optional
YAML-string `spread` values; sequence merges append, including duplicates. Its
`max_replicas_per_node` retains a YAML-integer or YAML-string category, with replacement, reset,
and override provenance. Extensions, unknowns, and malformed nested values remain source-addressable
evidence. The view supplies no constraint/spread grammar, node selection, count/range/default,
mode coupling, scheduling, runtime, or conversion interpretation.

Deploy `resources.limits.cpus` retains YAML-number or YAML-string scalar category and exact
spelling; `resources.limits.memory` accepts only YAML strings and conservatively retains raw text
as documented lowercase-unit, lexical-zero, deferred, or provider-dependent; and `limits.pids`
retains YAML-integer or YAML-string categories. Resource and limit mappings merge recursively; leaf
reset remains nested unmodeled reset evidence, while mapping reset/override provenance stays visible.
Extensions, unknowns, malformed values, and sensitivity remain source-addressable. These values
infer no service CPU, `mem_limit`, unlimited, positivity, range/default, host, cgroup, runtime,
consistency, or conversion behavior.

Deploy `resources.reservations.cpus` separately retains YAML-number or YAML-string scalar category
and exact spelling, while `resources.reservations.memory` accepts only YAML strings with the same
raw lowercase-unit, lexical-zero, deferred, and provider-dependent classification as limit memory.
Its resource/reservation mappings merge recursively with replacement, reset,
override, sensitivity, extension, unknown, and malformed evidence. It infers no relationship to
limit CPU, service CPU, scheduling, provider, runtime, target, or conversion behavior.

Deploy `resources.reservations.devices` retains schema-only ordered mapping/unmodeled items. A
mapping's required capabilities sequence preserves exact raw strings, duplicates, malformed
members, sensitivity, and ordinary append/reset/override provenance; its optional `driver` retains
only an exact YAML string scalar. `count` retains raw YAML-integer or strict YAML-string spelling,
including arbitrary strings such as `all`, while `device_ids` retains an ordered strict-YAML-string
sequence with malformed items preserved. Timestamp/regex styles, other scalar kinds, and collections
remain unmodeled evidence. Simultaneous count and IDs are diagnosed but neither is selected or
discarded. This does not select/load devices, parse capability/driver grammar, apply count range,
sign, default, or allocation semantics, interpret CDI, inspect the host/cgroups/runtime, claim
provider/version behavior, or convert. Options retain map/list syntax, scalar fidelity, malformed
evidence, exact duplicate list strings, and generic provenance without provider interpretation.

### Resolve references and paths

`resolve_paths` classifies the supported host paths and retains their raw text, source span,
purpose, first-file project origin, and optional lexically resolved form. Relative paths use the
merged project's retained base directory. `~` expansion requires a caller-supplied
`PathContext`; ComposeLens does not inspect the process home directory. Resolution does not
canonicalize, follow symlinks, test existence, or otherwise access the file system. Inactive
service mounts are not part of the selected view, while top-level config and secret file sources
remain project resources.

`validate_references` inspects active services and distinguishes found, missing, and
profile-inactive targets. The boundary covers networks, named volumes, configs, secrets,
`depends_on`, service namespace modes, links, and local long-form `extends.service` edges without
`file`. A `service_healthy` dependency
with no Compose health check produces a warning because image metadata is unavailable; an
explicitly disabled health check produces an error. Diagnostics never delete or rewrite the
authored reference. Long dependencies with `required: false` remain visible but downgrade missing,
inactive, or disabled-health failures to warnings.

Every resolver rejects a `ProfileSelection` created from a different merged project, even when the
caller reused the same source identifiers.

### Apply defaults

`resolve_defaults` turns omissions into explicit `DefaultRequest` values and asks a caller-owned
`DefaultProvider` for each decision. `NoDefaults` leaves every omission unresolved.
`ComposeDefaults` supplies the documented network, port, volume, config, secret, and restart
defaults for an explicit Linux or Windows container platform. The result is a decision overlay;
the merged tree is unchanged.

Defaulting remains separate from parsing and implementation compatibility. A future Docker Compose
or Podman Compose profile may deliberately supply different values or decline a specification
default when versioned runtime evidence requires it.

### Validate

`validate_compatibility` discovers compatibility-sensitive constructs in the selected merged view
and applies an explicit `CompatibilityProfile`. The initial detector covers combined image tags and
digests, short and long bind `SELinux` relabeling, `!reset`, `!override`, and `x-` extensions.
Supported occurrences remain in the report without diagnostics; implementation-specific,
deprecated, unsupported, and unknown classifications receive stable diagnostics at the authored
span. Validation never deletes or normalizes the construct.

Profiles identify the exact Compose provider independently from the optional backend runtime.
Docker Compose can target Docker Engine or Podman. The independent `containers/podman-compose`
provider is distinct from Podman's `podman compose` wrapper, which delegates to an external
provider. ComposeLens therefore has no ambiguous “Podman Compose” target that guesses which
provider was executed.

Implementation versions are exact three-component numeric values. Evidence can carry inclusive
minimum and maximum provider and runtime ranges. ComposeLens never substitutes “latest” when the
caller omitted a version. Built-in classifications distinguish specification text, official
documentation, public issue reproductions, and future ComposeLens-controlled runtime conformance.
Unknown remains an honest result when evidence does not cover the selected version pair.

The tolerant profile preserves constructs and emits notes for unknown runtime behavior. It does
not turn missing evidence into a support claim. A selection belonging to another merged project is
rejected before compatibility discovery.

### Render

`render_canonical` emits the fixed `compose-lens-canonical-v1` YAML form from a merged project. A
matching optional profile selection filters inactive services while retaining top-level resources.
The result contains output, structured diagnostics, validity, and sensitivity state. Safe retained
YAML tags survive; invalid tags are diagnosed and dropped; unresolved aliases are diagnosed and
recovered as `null` so the output remains parseable.

Canonical rendering does not implicitly load, interpolate, merge, resolve, default, normalize, or
validate. In particular, it retains effective short and long Compose forms instead of reproducing
the normalizing behavior of `docker compose config`. The exact format and security boundary are
documented in [ADR 0009](decisions/0009-deterministic-canonical-rendering.md). Preservation-oriented
syntax editing remains a separate operation with its own source-level contract.

`apply_preservation_edits` atomically replaces existing YAML value scalars at exact source spans.
The initial replacement types are string, sensitive string, boolean, validated number, and null.
The operation patches the original syntax text rather than serializing the typed document, so
comments, whitespace, ordering, extensions, unknown fields, and untouched scalar spelling remain
byte-identical. Any mismatched source, non-scalar target, unsupported block/multiline style,
invalid number, or overlap rejects the complete batch and returns the original text.

Spans are revision-specific. A caller parses successful output again before creating another edit
from the new document. Structural insertions, removals, collection transformations, and block
scalar editing remain outside the initial boundary. See the [editing guide](preservation-editing.md)
and [ADR 0010](decisions/0010-atomic-span-based-preservation-edits.md).

`render_canonical_with_formatting` uses the same merged project and optional selection as canonical
rendering, plus explicit `CanonicalFormatting`. The only choices are a positive space-indentation
width, LF or CRLF, document-marker emission, and final-line-ending emission. Default formatting is
byte-identical canonical-v1. Formatting does not alter ordering, quoting safety, Compose syntax
forms, diagnostics, or sensitivity. See the [formatting guide](render-formatting.md) and
[ADR 0011](decisions/0011-presentation-only-render-formatting.md).

## Unknown and implementation-specific data

- `x-*` extensions receive first-class preservation.
- Unknown fields are retained with source locations.
- Validation profiles decide whether unknown fields are allowed, warned about, or rejected.
- Podman-specific fields remain native data; BoxFerry decides whether a target can represent them.

## Image references

ComposeLens preserves the written reference and may expose a tolerant parsed view. It must support implementation behavior such as references combining a tag and digest when the selected implementation accepts them. Normalization must never erase the original value.
