# ADR 0016: Native merged-project consumer view

- Status: accepted
- Date: 2026-08-02
- Additive amendments: 2026-08-03, 2026-08-05, 2026-08-06, 2026-08-07, 2026-08-08

## Context

ComposeLens already retained correct multi-file semantics and provenance in `MergedProject`, but
the public post-merge representation was a generic `MergedValue` tree. A native adapter such as
BoxFerry therefore had two poor choices: duplicate Compose field interpretation over
`MergedValue`, or render canonical YAML and parse it again as a `ComposeDocument`. The latter
replaces original source locations with generated-output spans and cannot explain which input file
contributed an effective value.

The first conversion boundary needs native images, commands, environment, ports, volumes, service
networks, profiles, and top-level resources after interpolation, merge, and profile selection.
It also needs an explicit path for fields outside that boundary without pretending that they were
converted.

## Decision

ComposeLens adds the public `project` module and `build_project_view` operation:

1. The builder consumes a `MergedProject` and an optional matching `ProfileSelection` directly.
   It never renders, reparses, reads files, reads environment variables, or invokes a provider.
2. A matching selection includes only active services. Omitting the selection includes every
   service. A selection from another project returns the existing stable mismatch diagnostic and
   no view.
3. The initial view exposes native effective values for the BoxFerry conversion boundary:
   project name; services; images; commands; environment; extra hosts; health checks; service
   dependencies; ports; volume mounts; network attachments; profiles; and top-level network,
   volume, config, and secret definitions.
4. `ProjectValue<T>` wraps every exposed effective value with the complete `MergeProvenance`, the
   effective source, and sensitivity state. Collections also retain per-item provenance.
5. `ProjectKey` retains every authored key span. Environment entries are normalized by semantic
   key after Compose field-specific merging while retaining each effective `EntrySyntax`.
6. Fields outside the initial project-view boundary remain available as
   `ProjectFieldReference` values with semantic paths, all key sources, value provenance,
   extension classification, and sensitivity state.
7. Sensitive project values redact their contents from `Debug` output.
8. Invalid effective forms return stable source-spanned diagnostics and as much partial native data
   as can be represented.
9. Effective `extra_hosts` entries retain hostname key sources, address provenance, sequence or
   mapping syntax, and raw-preserving address classification. `host-gateway` remains an explicit
   implementation token instead of being forced into an IP-address grammar.
10. Effective `depends_on` retains short versus long syntax, ordered service-name provenance, and
    nested `condition`, `restart`, and `required` provenance. Unknown and extension options remain
    source-addressable. The view exposes authored intent; it does not claim that another lifecycle
    manager has equivalent startup, health, completion, restart-propagation, or optionality
    semantics.
11. Effective execution identity exposes raw-preserving `user` and `userns_mode`, ordered
    `group_add`, `working_dir`, and `read_only` values. The view classifies only lexical forms and
    known namespace modes; it does not resolve host accounts, groups, paths, or runtime state.
12. Effective service config and secret grants retain short versus long syntax, item provenance,
    and field-level provenance for `source`, `target`, `uid`, `gid`, and `mode`. Unknown options
    remain source-addressable, and unique-by-target merging is not flattened into replacement.
13. Effective service labels are normalized by semantic key after Compose field-specific merging.
    Each entry retains mapping, `KEY=VALUE`, or key-only list syntax plus complete key and value
    provenance. Key-only labels expose an empty string rather than inheriting environment-variable
    host-resolution semantics.
14. Effective service devices retain an optional ordered mixed short/long sequence. Raw path,
    CDI-like, deferred, and opaque short strings remain authoritative. Long `source`, `target`, and
    `permissions` values plus extensions and unknown fields retain nested provenance. The existing
    target-keyed replacement rule remains observable alongside reset, override, duplicates, and
    interpolation sensitivity without device, permissions, CDI, GPU, or runtime validation.
15. Effective `dns` and `dns_search` retain scalar/list form, append/replacement rules,
    provenance, duplicates, sensitivity, reset, and override; `dns_opt` retains the same evidence
    with whole-sequence replacement.
16. Effective `expose` retains ordered YAML scalar identity and documented decimal classification
    without inferring a protocol or runtime publication.
17. Effective `annotations` retain mapping/list syntax and keyed contributors. Mapping keys do
    not interpolate, and ambiguous key-only entries receive no invented value.
18. Effective `security_opt` retains the raw appended sequence. Exact AppArmor,
    no-new-privileges, seccomp, SELinux-label, Mask, and Unmask shapes are independent diagnostic
    candidates; conflicts and near misses remain unselected.
19. These views perform no resolver, profile, path, filesystem, provider, runtime, or cross-format
    interpretation.
20. Effective service `logging` retains an uninterpreted optional string driver and optional
    ordered options mapping. String/number/null option kinds, authored and interpolated spelling,
    nested recursive-merge and replacement provenance, empty/reset/override state, sensitivity,
    extensions, unknowns, and malformed entries remain source-aware without provider semantics.
21. Effective service `build` retains short scalar and long mapping context syntax. Long-form
    map/list `args`, optional ordered raw-string `cache_from`, `cache_to`, and `entitlements`, sensitive map/list `ssh`, non-empty `dockerfile`, exact-string `dockerfile_inline`, opaque scalars `target` and `network` (including empty), optional
    ordered raw-scalar `platforms` and `tags`, boolean/expression `privileged`, boolean/string `no_cache` and `sbom`, raw-preserving
    `shm_size`, service-equivalent `ulimits`, and map/list-preserving `labels` become native project values.
    Argument mappings retain string/number/boolean/null values and keyed replacement; raw lists retain append/reset/override,
    bare entries, duplicates, sensitivity, and malformed-item recovery. `no_cache` and `sbom` retain boolean/string YAML type, per-file interpolation sensitivity, scalar replacement/reset/override provenance, and malformed evidence without coercion, defaults, or builder behavior; `sbom` does not parse generators or expose generated data. `shm_size` reuses the service raw scalar classification: YAML number/string spelling, documented lowercase units, lexical zero, deferred expressions, and provider-dependent values remain visible without builder defaults, host, allocation, or runtime inference. `ulimits` reuses the service ordered mapping, scalar/range, nested-provenance, malformed-evidence, and no-default boundary without host or builder interpretation. Labels retain the same list/map merge evidence. Cache descriptors retain order, duplicates, explicit empty state, spans, interpolation sensitivity, and generic append/reset/override provenance without cache-type, reference, source, destination, path, image, credential, or builder interpretation. Entitlements retain raw ordered strings, duplicates, explicit empties, interpolation sensitivity, and generic append/reset/override provenance without allowlist, privilege, BuildKit/platform, execution, or runtime claims. Docker Compose v2.27.0 remains an implementation badge with earlier and removal boundaries unknown. `dockerfile_inline` retains exact empty or multiline string content, interpolation sensitivity, scalar replacement/reset/override provenance, malformed recovery, and a source-spanned mutual-exclusion diagnostic while both values remain available. It does not parse Containerfiles, access paths or contexts, scan secrets, build, or infer Docker, BuildKit, or runtime behavior. Docker Compose v2.17.0 remains an implementation badge with earlier and removal boundaries unknown. All other sibling, extension, unknown, and malformed fields
    remain source-addressable unmodeled evidence. The view does not generate builds or infer build
    execution semantics.

    `build.privileged` retains literal booleans or deferred dollar expressions with scalar
    replacement/reset/override provenance. Ordinary quoted non-expression strings are rejected
    rather than coerced and remain source-addressable unmodeled evidence with diagnostics. Docker
    Compose v2.15.0 is an implementation badge only, with earlier and removal boundaries unknown;
    no privilege, platform, runtime, or build behavior is inferred.
22. Effective service `credential_spec` retains an explicit empty mapping and optional strict
    YAML-string `config`, `file`, and `registry` members with nested provenance, malformed/unknown
    evidence, and generic recursive merge. It does not resolve top-level configs, paths, files,
    registries, accounts, URI forms, Windows/gMSA, platforms, providers, runtimes, or conversion.
23. Effective service `extends` retains the schema-supported YAML-string short form or a long
    mapping with optional strict YAML-string `service` and `file` members. Explicit empties,
    malformed/extension/unknown evidence, member provenance, sensitivity, and generic scalar or
    recursive-mapping merge remain visible; a missing long-form `service` is diagnosed without
    dropping its mapping. The raw view does not expand/merge services, look up files, normalize
    paths, traverse cycles, or import resources. The separate `validate_references` stage may
    validate a same-file long-form `service` edge without `file`, but performs none of those
    operations or provider, platform, runtime, or conversion inference.
24. Effective service `provider` retains a required strict YAML-string `type` and optional ordered
    `options` mapping. String/number/boolean scalar and ordered sequence categories, malformed
    evidence, sensitivity, and generic merge/reset/override provenance remain visible; empty types
    remain valid and missing types diagnose. It does not execute/discover providers, call setup/
    teardown, inject environment, resolve credentials, validate provider grammar/compatibility, or
    perform conversion.
25. Effective service `post_start` and `pre_stop` retain ordered closed hook mappings with a required existing
    null/scalar/list command and optional local map/list environment, BooleanValue privileged, and
    strict YAML-string user/working-directory members. Item/member provenance, malformed evidence,
    and generic append/reset/override merge remain visible. It performs no lifecycle scheduling,
    command execution/expansion, environment inheritance, user/path/default resolution, privilege
    decision, provider compatibility validation, or conversion.
    Effective service `pre_start` remains distinct: its command is optional, while raw strict-string
    image, local environment, BooleanValue privilege/per-replica, and strict-string user/working-directory
    members retain independent provenance without defaults, execution, or runtime interpretation.
    Effective service `runtime` is a strict raw YAML string with ordinary scalar provenance and no
    grammar, default, provider, host, execution, generation, or conversion interpretation.
    Effective service `pull_refresh_after` is independently a strict raw YAML string with ordinary
    scalar provenance. It has no interval grammar, minimum, default, `pull_policy: refresh`
    coupling, refresh-state calculation, provider support, execution, generation, or conversion interpretation.
    Effective service `platform` is independently a strict raw YAML string with ordinary scalar
    provenance. It has no OCI component grammar, normalization, alias/case handling, host or image-manifest
    inspection, build feasibility, provider support, `build.platforms` coupling, default, generation, or conversion interpretation.
    Effective service `attach` retains literal booleans or deferred dollar expressions with ordinary
    scalar replacement/reset/override provenance and source-addressable malformed evidence. It has
    no default, generated API, logging, runtime, provider, CLI, compatibility, or conversion
    interpretation.
    Effective service `blkio_config` retains its optional scalar and ordered nested items with
    complete generic merge provenance. It does not resolve `extends` or claim path-keyed rate-array
    inheritance, and it applies no unit, range, default, controller, runtime, provider, I/O, or
    conversion interpretation.
    Effective service `cgroup` retains a strict raw YAML string as `host`, `private`, a deferred
    dollar-bearing expression, or diagnosed `Other` spelling with scalar provenance and malformed
    evidence. It applies no default, controller, cgroup-version, rootless, systemd, provider,
    version, runtime, `extends`, generated-document, or conversion interpretation.
    Effective service `cgroup_parent` retains a strict raw YAML string with scalar provenance and
    malformed evidence. It applies no grammar, path, controller, host, runtime, provider, version,
    default, `extends`, generated-document, or conversion interpretation.
    Effective service `cpu_count` retains YAML integer/string category, exact spelling, scalar
    provenance, and malformed evidence. Nonnegative unbounded/base/separator/negative-zero integer
    spellings remain typed; negative integers diagnose without loss. It applies no quota, host,
    scheduler, runtime, provider, OS, version, default, `extends`, generated-document, or conversion interpretation.
    Effective service `cpu_percent` retains YAML integer/string category, exact spelling, scalar
    provenance, and malformed evidence. YAML integers are classified against the schema's inclusive
    `0..=100` range without fixed-width conversion; out-of-range integers diagnose without loss,
    while strings are never coerced or range-checked. It applies no percentage calculation, CPU,
    quota, host, scheduler, runtime, provider, OS, version, default, `extends`, generated-document,
    or conversion interpretation.
    Effective service `cpu_period` retains YAML number/string category, exact spelling, scalar
    provenance, and malformed evidence without numeric conversion or semantic validation. It applies
    no duration, microsecond, CFS, CPU, host, runtime, provider, OS, version, default, `extends`,
    generated-document, or conversion interpretation.
    Effective service `cpu_quota` retains YAML number/string category, exact spelling, scalar
    provenance, and malformed evidence without numeric conversion or semantic validation. It applies
    no numeric quota, duration, microsecond, CFS, CPU, host, runtime, provider, OS, version,
    default, `extends`, generated-document, or conversion interpretation.
    Effective service `cpu_rt_period` retains YAML-number, duration, expression, and other-string
    categories, exact spelling, scalar provenance, and malformed evidence. Other strings diagnose
    without loss. It applies no CPU calculation, microsecond conversion, realtime scheduler, OS,
    host, default, provider, version, runtime, generated-document, or conversion interpretation.
26. Effective service `deploy` retains a minimal mapping shell. `endpoint_mode` and `mode` become
    native `vip`/`dnsrr`, `global`/`replicated`, or raw `Other(String)` values; `replicas` retains
    exact YAML number spelling or a distinct YAML string category, including empty and deferred
    strings. `labels` retains distinct mapping scalar/null categories or ordered list bare/`KEY=VALUE`
    entries; mapping keys merge while lists append duplicate fallible-input evidence. Malformed,
    extension, and future-unknown immediate children remain `ProjectFieldReference` values. Other strings
    produce a portability diagnostic rather than rejection, and malformed non-scalar replica forms
    remain evidence. The prose's `vip` default and the schema's lack of an effective default are
    recorded as conflicting evidence, so no default is injected. The view validates no integer
    grammar and performs no container-label, platform, service-discovery, VIP, DNS, replica, scale,
    allocation, scheduling, deployment, runtime, or conversion interpretation. It does not reconcile
    global mode with replicas or service scale.
    `restart_policy` remains distinct from service `restart`: condition, delay, max-attempts, and
    window retain member provenance without fallback, defaults, precedence, simulation, runtime, or
    conversion interpretation. `update_config` and the distinct `rollback_config` are independent recursively merged mappings with member provenance, raw scalar
    categories, malformed evidence, and mapping-specific provider-order diagnostics; they supply no rollout,
    scheduling, default, runtime, or conversion interpretation. `placement` retains ordered YAML-string constraints, preference
    mappings with an optional YAML-string spread, and YAML-integer or YAML-string
    max-replicas-per-node categories. Generic sequence append, replacement, reset, and override
    provenance remains visible at collection, item, and nested-member level; extensions, unknowns,
    and malformed children remain evidence. No constraint/spread grammar, node selection,
    count/range/default, mode coupling, scheduling, runtime, or conversion interpretation is
    supplied. `resources.limits.cpus` retains a YAML-number or YAML-string category,
    `resources.limits.memory` is YAML-string-only with raw lowercase-unit, lexical-zero, deferred,
    and provider-dependent classification, while `resources.limits.pids` retains YAML-integer or
    YAML-string categories. Their exact spelling, recursive resource/limit mapping provenance, and
    nested extension, unknown, malformed, sensitivity, reset, and override evidence remain visible.
    They infer no service CPU, `mem_limit`, unlimited, positivity, range/default, host, cgroup,
    runtime, consistency, or conversion behavior.
    `resources.reservations.cpus` independently retains YAML-number or YAML-string categories, and
    `resources.reservations.memory` reuses the YAML-string-only deploy-memory classification with
    exact spelling, recursive resource/reservation provenance, and nested extension, unknown,
    malformed, sensitivity, reset, and override evidence. They make no limit/service CPU or
    memory-limit, scheduling, provider, runtime, target, or conversion claim.
    `resources.reservations.devices` retains schema-only ordered mapping/unmodeled items; required
    capability strings, optional strict YAML-string drivers, raw YAML-integer-or-string counts, and
    ordered strict-string device IDs retain malformed evidence, sensitivity, and merge provenance.
    Timestamp/regex-style, non-string, and collection strict values remain unmodeled. Simultaneous
    count and device IDs are diagnosed without selecting either. The view does not select/load
    devices or interpret capability/driver names, count range/sign/default/`all`, allocation,
    CDI, host, scheduling, provider/version, runtime, cgroup, or conversion behavior. `options`
    retains map/list form and collection provenance: maps retain ordered strict nonempty
    YAML-string keys with shared scalar values, and lists retain ordered strict YAML strings,
    including explicit empty and duplicate items. Invalid children remain source-addressable;
    generic device-list merge governs the collection with no option-key, driver/provider,
    default, runtime, device-selection, or conversion semantics.

The generic `MergedProject` remains public for advanced analysis, validation, rendering, and future
native-boundary expansion. The new view is additive within the 0.1.x compatibility line.

## Consequences

- BoxFerry can migrate from one-document import to a real multi-file, profile-selected input
  without owning Compose merge-tree interpretation.
- Conversion outcomes can cite every source file that contributed to an effective value.
- Mixed list/mapping environment merges do not have to be forced back into one invented document
  syntax.
- Native project coverage can grow field by field while unmodeled references prevent silent loss.
- `ProjectValue<T>` is intentionally separate from single-document `Located<T>`: one effective
  value can have several contributing spans.

## Alternatives considered

- Canonical render and `ComposeDocument` reparse was rejected because generated spans erase
  original multi-file provenance.
- Exposing only convenience lookup methods on `MergedValue` was rejected because adapters would
  still duplicate field forms, validation, and native parsing.
- Replacing `MergedProject` with one normalized object graph was rejected because validation,
  exact rendering, unknown fields, operations, and syntax evidence still need the generic tree.
- Moving the interpretation into BoxFerry was rejected because Compose semantics belong in
  ComposeLens and should be reusable by independent consumers.
