# Roadmap

This roadmap orders ComposeLens work by consumer value and records the exact current specification
surface that is not yet available through a native typed API. It is not a delivery schedule.

The [quality plan](quality-plan.md) groups the remaining work into maintainable priorities and
defines what “good enough” means for testing and conformance.

## Coverage language

ComposeLens has several independent coverage layers:

| State                   | Meaning                                                                                                           |
| ----------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Syntax-preserved        | The YAML parser retains the key, spelling, order, source range, extensions, and unknown fields where recoverable. |
| Document typed          | One authored document exposes a native source-aware value.                                                        |
| Project typed           | The effective merged/profile-selected project exposes the value with provenance.                                  |
| Generated               | New Compose YAML can be constructed through typed values and parse-back validation.                               |
| Compatibility-evidenced | Provider/runtime/version behavior is backed by documented or observed evidence.                                   |

“Not typed” does not mean “cannot be read.” Every key below remains available through the
loss-aware syntax document. It means callers cannot yet consume that key through a dedicated
ComposeLens semantic type.

## Specification snapshot

This ledger was audited on 2026-08-14 against the official
[Compose JSON schema snapshot](../schema/compose-spec.json) from
[compose-spec commit 11296e387ba76c77db1db768b9153a4304a3c9bd](https://github.com/compose-spec/compose-spec/blob/11296e387ba76c77db1db768b9153a4304a3c9bd/schema/compose-spec.json),
[Compose Specification](https://github.com/compose-spec/compose-spec/blob/main/spec.md), and
[Docker Compose file reference](https://docs.docker.com/reference/compose-file/).
Provider additions remain eligible when real Docker Compose or Podman Compose accepts them; the
compose-spec repository is not the sole compatibility authority.

The audited schema currently contains 9 top-level keys and 93 service keys.

| Surface   | Project typed | Document typed only | Syntax-preserved only |
| --------- | ------------: | ------------------: | --------------------: |
| Top level |             9 |                   0 |                     0 |
| Service   |            93 |                   0 |                     0 |

`x-*` extensions are intentionally open-ended and preserved. They are not counted as missing
closed-schema keys.

The closed root/service-key snapshot and its strict classification inventory are maintained in
[`schema/`](../schema/). Offline repository policy tests verify the pinned digest, the closed
`additionalProperties: false` shape, the sole `^x-` extension allowance, and exact 9/93 key sets.
Scheduled or manually dispatched upstream drift checks only report differences; they never modify
the repository or create issues. They label added/removed root or service keys as inventory drift;
a digest-only change with those sets unchanged is content-only drift requiring nested, prose, or
other non-inventory review. This bounded phase deliberately leaves all nested-schema drift outside
the inventory; nested coverage remains tracked below.

## Exact top-level gaps

There are no remaining closed-schema top-level gaps. `version` is retained with an obsolete-field
warning and never selects a provider or schema. `include` retains short paths and long
`path`/`env_file`/`project_directory` forms. Its completed traversal slice accepts
caller-authorized loading, uses effective no-interpolation declarations in depth-first order, and
retains origins, partial graphs, cycles, and global source-ID diagnostics. Its opt-in composition
slice recursively imports absent child services, networks, volumes, configs, secrets, and
individual models after each parent merge; local/earlier selections win exact-name collisions with
explicit source-aware warnings and no cross-project merge. Its opt-in project-directory plan
defaults root/undeclared children from retained first-document directories and delegates only
explicit declarations to caller policy with nested effective-parent context. It does not itself
canonicalize/join paths, read environment files or `.env`, infer names, cache diamonds, render a
composed document, or select provider behavior. `models` retains `name`, required `model`,
`context_size`, and `runtime_flags`. None of these operations performs implicit file, environment,
or provider access.
Malformed or unknown nested include/model members remain reachable through the effective view's
root or service `unmodeled_fields` references rather than being normalized away.

`name`, `services`, `networks`, `volumes`, `configs`, and `secrets` are document- and
project-typed.

## Exact service gaps

### Project-typed service keys

The effective project view currently exposes:

`annotations`, `blkio_config`, `cgroup`, `cgroup_parent`, `command`, `configs`, `container_name`, `cpu_count`, `cpu_percent`, `cpu_period`, `cpu_quota`, `cpu_rt_period`, `cpu_rt_runtime`, `cpu_shares`, `cpus`, `cpuset`, `device_cgroup_rules`, `ipc`, `mem_reservation`, `mem_swappiness`, `memswap_limit`, `network_mode`, `oom_kill_disable`, `oom_score_adj`, `pid`, `scale`, `volumes_from`, `credential_spec`, `depends_on`, `entrypoint`, `env_file`,
`environment`, `extends`, `extra_hosts`, `provider`, `build.additional_contexts`, `build.context`, `build.args`, `build.cache_from`, `build.cache_to`, `build.dockerfile`, `build.dockerfile_inline`, `build.entitlements`, `build.extra_hosts`, `build.target`, `build.network`, `build.isolation`, `build.platforms`, `build.no_cache`, `build.privileged`, `build.sbom`, `build.pull`, `build.shm_size`, `build.tags`, `build.labels`, `build.secrets`, `build.ssh`, `build.ulimits`, `cap_add`, `cap_drop`, `devices`, `dns`, `dns_opt`, `dns_search`,
`expose`, `group_add`, `healthcheck`, `hostname`, `image`, `init`, `platform`, `stdin_open`, `tty`, `privileged`, `attach`, `labels`, `logging`, `networks`, `ports`,
`post_start`, `pre_stop`, `pre_start`, `profiles`, `read_only`, `pids_limit`, `pull_policy`, `pull_refresh_after`, `restart`, `runtime`, `secrets`, `security_opt`,
`shm_size`, `mem_limit`, `stop_grace_period`, `stop_signal`, `sysctls`, `tmpfs`, `ulimits`, `user`,
`userns_mode`, `volumes`, and `working_dir`.

`deploy.endpoint_mode`, `deploy.labels`, `deploy.mode`, `deploy.placement`, `deploy.replicas`, and
`deploy.resources.limits.cpus`, `deploy.resources.limits.memory`, `deploy.resources.limits.pids`,
`deploy.resources.reservations.cpus`, `deploy.resources.reservations.devices[].capabilities`,
`deploy.resources.reservations.devices[].driver`,
`deploy.resources.reservations.devices[].count`,
`deploy.resources.reservations.devices[].device_ids`,
`deploy.resources.reservations.devices[].options`,
`deploy.resources.reservations.generic_resources`, `deploy.resources.reservations.memory`, `deploy.rollback_config`, and `deploy.update_config` are also native in
the effective project view: exact `vip`/`dnsrr` and `global`/`replicated` remain distinct from raw
portability-diagnosed `Other` strings, replicas preserves its exact YAML number spelling or distinct
string category, labels retain mapping scalar/null or ordered raw list forms separately from
service container labels, placement retains raw constraints, preferences, and maximum scalar
categories with complete merge provenance, resource-limit CPUs retain number/string categories,
memory retains string-only conservative classification, PIDs retain integer/string categories, and
reservation CPUs retain number/string categories while reservation memory reuses the string-only
classification. Reservation generic resources retain schema-only ordered list evidence: ordinary
append/reset/override provenance, mapping/unmodeled item form, and optional raw discrete kind/value
members. They make no prose, version, provider, matching, scheduling, device, runtime, or
conversion claim. Reservation-device capabilities, strict YAML-string drivers, raw integer/string
counts, and ordered strict-string IDs retain schema-only ordered evidence with duplicate and
conflict diagnostics, without selection/loading, allocation, grammar, scheduling, CDI, host,
runtime, or conversion claim. Options retain map/list syntax, scalar fidelity, malformed evidence,
duplicates, and generic provenance without provider interpretation. All current immediate deploy
children are native values; only malformed, extension, and future-unknown child evidence, plus the
explicitly bounded nested resource forms, remain unmodeled.
The prose `vip` default and schema lack of an effective default conflict, so no default,
integer/positive/zero rule, mode coupling, container-label, platform, discovery, VIP, DNS, replica,
scale, allocation, scheduling, placement, job, deployment, runtime, or conversion interpretation is
applied.

The effective project view also exposes `develop`, `domainname`, `external_links`, `gpus`,
`isolation`, `label_file`, `links`, `mac_address`, `models`, `storage_opt`, `use_api_socket`, and
`uts`. Model references are validated locally; GPU selectors, label files, links, sockets, and
watch actions remain inert data with no runtime, host, file, or provider access.
Malformed or future nested GPU and develop members are retained through the containing service's
`unmodeled_fields` references.

### Document-only service keys

All current service and immediate deploy keys have an effective-project path. Nested resource
coverage remains limited to the explicitly listed forms; malformed, extension, and future-unknown
deploy child evidence remains source-addressable.

The effective build view promotes raw list/scalar-map `additional_contexts`, scalar/long `context`, ordered raw `cache_from`/`cache_to`/`entitlements`, non-empty `dockerfile`, exact-string `dockerfile_inline`, Build-specific list/map `extra_hosts` with scalar or nested-list raw addresses, opaque `target`/`network`/`isolation`, ordered raw `platforms`/`tags`, map/list `args`/`labels`, boolean/string `no_cache`/`sbom`, boolean/expression `privileged`/`pull`, raw-preserving `shm_size`, service-equivalent ordered `ulimits`, short/long `secrets`, and sensitive list/scalar-map `ssh` with form, sensitivity, provenance, duplicates, empties, reset/override, and partial recovery.
Cache descriptors and platforms remain raw, `no_cache` and `sbom` strings remain uncoerced, and `pull` remains unresolved: none receives reference, path, credential, default, or build-execution inference. `sbom` does not parse generators or expose generated data.
`build.ssh` does not parse identifiers, paths, PEM, sockets, agents, mounts, or builder behavior; all grant values remain redacted by default. `build.entitlements` has no allowlist, privilege, BuildKit/platform, execution, or runtime claim; Docker Compose v2.27.0 is a badge with earlier/removal boundaries unknown. `build.dockerfile_inline` retains exact strings and conflict evidence with `dockerfile` but performs no Containerfile parsing, path/context access, secret scanning, build, Docker, BuildKit, or runtime inference; Docker Compose v2.17.0 is a badge with earlier/removal boundaries unknown. `build.shm_size` does not infer builder defaults, host state, allocation, or runtime behavior. All closed immediate Build keys are typed; malformed input, extensions, future unknown members, and deliberately bounded nested semantics remain source-addressable unmodeled evidence without I/O, builder execution, or runtime inference.
`build.privileged` retains literal booleans or deferred dollar expressions. Ordinary quoted
non-expression strings remain diagnosed source evidence rather than coerced booleans. Docker
Compose v2.15.0 is a badge with earlier/removal boundaries unknown; no privilege, platform,
runtime, or build behavior is inferred.
`build.provenance` retains only YAML boolean or opaque string form and no attestation parsing, generation, publication, validation, builder execution, or runtime claim; Docker Compose v2.39.0 is a badge with earlier/removal boundaries unknown.

### Syntax-preserved-only service keys

There are no remaining closed-schema service-key gaps. `develop.watch` retains its documented
members and local shape diagnostics without watching paths or executing actions. `gpus` retains
the exact `all` scalar or list selectors without allocating devices. `label_file`,
`external_links`, and `links` preserve authored order without reading or resolving their targets.

## Nested resource coverage

All currently audited top-level resource-definition members are typed. Configs retain labels and
opaque strict-string template drivers. Secrets retain opaque strict-string drivers/template drivers,
ordered string-or-number driver options, and labels. Networks, volumes, configs, and secrets retain
the current boolean/expression `external` form separately from deprecated `external: { name: ... }`.
Legacy objects receive a migration warning and a two-span conflict diagnostic when they coexist with
current `name`; they are not normalized. The Compose schema deprecates this object for networks,
volumes, and configs, while Docker Compose 5.4.0 warned for all four: ComposeLens documents that
divergence rather than claiming universal provider behavior.

The existing service `ports`, `volumes`, `networks`, `configs`, and `secrets` types retain their
documented short/long forms. Future schema additions must first enter this ledger before support is
claimed.

## Exact nested semantic coverage

The 93-key service ledger above classifies immediate service keys. The following closed nested
keys have dedicated typed forms with intentionally bounded semantics. Open-ended user maps such
as labels, environment variables, driver options, and extension fields are intentionally not
enumerated. Service logging's `driver` and ordered scalar `options` are typed.

`develop.watch[].exec` retains null/scalar/list `command`, strict-string `user` and `working_dir`,
literal/deferred `privileged`, and list/map `environment` without watching, executing, resolving a
user/path, or reading environment files. `sync+exec` still requires a non-empty command.

`deploy.endpoint_mode`, `deploy.labels`, `deploy.mode`, `deploy.placement`, `deploy.replicas`,
`deploy.resources.limits.cpus`, `deploy.resources.limits.memory`, `deploy.resources.limits.pids`,
`deploy.resources.reservations.cpus`, `deploy.resources.reservations.devices[].capabilities`,
`deploy.resources.reservations.devices[].driver`,
`deploy.resources.reservations.devices[].count`,
`deploy.resources.reservations.devices[].device_ids`,
`deploy.resources.reservations.devices[].options`,
`deploy.resources.reservations.generic_resources`,
`deploy.resources.reservations.memory`, `deploy.restart_policy`, `deploy.rollback_config`, and `deploy.update_config` complete the current immediate deploy children. Nested resource coverage remains
limited to the explicitly listed forms, while malformed, extension, and future-unknown deploy
children remain source field references. Placement retains ordered raw
constraints/preferences and YAML integer/string maximum categories with merge provenance, but no
constraint grammar, node selection, count/default, scheduling, runtime, or conversion
interpretation. Restart-policy members retain raw spelling and member provenance without
service-restart fallback/default/precedence, simulation, runtime, or conversion interpretation.
map/list `build.additional_contexts`, `build.context`, map/list `build.args`, `build.labels`, and Build-specific `build.extra_hosts`, ordered raw `build.cache_from`/`build.cache_to`/`build.entitlements`, non-empty `build.dockerfile`, exact-string `build.dockerfile_inline`, opaque
`build.target`/`build.network`/`build.isolation`, ordered raw `build.platforms`/`build.tags`, boolean/string `build.no_cache`/`build.sbom`/`build.provenance`, scalar/list `build.no_cache_filter`, boolean/expression `build.privileged`/`build.pull`, raw-preserving `build.shm_size`, service-equivalent `build.ulimits`, and short/long `build.secrets`
are the promoted build values; their complete closed-key boundary remains open:

- `build`: all current immediate subkeys are promoted;
- `deploy`: all current immediate subkeys are promoted; nested resource coverage remains bounded;
- `deploy.resources.limits`: complete;
- `deploy.resources.reservations`: complete;
- `deploy.resources.reservations.devices[]`: complete;

Conversely, the current nested keys under `depends_on`, `env_file`, `healthcheck`, service
`networks`, service `ports`, service config/secret grants, network `ipam`, and service `ulimits`
already have dedicated document types. This distinction prevents an immediate parent key from
hiding a nested semantic gap.

## Generated-document boundary

Generated documents currently cover project `name`, services, networks, volumes, file-backed
configs, and file-backed secrets. Generated
services cover `hostname`, `container_name`, `image`, `entrypoint`, `command`, `init`, `stdin_open`, `tty`, `privileged`, `env_file`, `environment`, `labels`, `annotations`, `logging`, `user`,
`userns_mode`, `group_add`, `cap_add`, `cap_drop`, `working_dir`, `read_only`, `pids_limit`, `shm_size`, `tmpfs`, `sysctls`, `ulimits`, `pull_policy`, `restart`, `stop_signal`,
`stop_grace_period`, `extra_hosts`, `ports`, `cpu_rt_runtime`, `cpu_shares`, `cpus`, `cpuset`,
`device_cgroup_rules`, `ipc`, `mem_reservation`, `mem_swappiness`, `memswap_limit`,
`network_mode`, `oom_kill_disable`, `oom_score_adj`, `pid`, `scale`, `volumes_from`,
`volumes`, `networks`, raw resolved `domainname`/`isolation`/`mac_address`/`uts`, literal
`use_api_socket`, and scalar `gpus: all`. Generated GPU lists, driver metadata, resource metadata,
and develop watches remain intentionally outside this boundary.
Generated long-form service-network attachments retain aliases plus optional raw `ipv4_address`
and `ipv6_address` values with omission, sensitivity, and named-network scope intact.
Generated top-level network definitions add optional opaque `driver` and ordered unique
string-or-number `driver_opts` without changing the shared basic/external `GeneratedResource`
network API. They are application-owned; external definitions remain `GeneratedResource::external`
because Compose permits only `name` alongside `external`. Driver/plugin and provider-specific option
semantics remain outside generation.

Generated top-level volume definitions use a separate application-owned API with optional opaque
`driver` and ordered unique string-or-number `driver_opts`. It preserves explicit empty maps and
scalar shape without accepting driver-configured external volumes; `GeneratedResource::external`
remains the compatible external lifecycle API. BoxFerry owns conversion outcomes for general
volume driver options, image drivers, external lifecycle, and platform names.
Application-owned generated volume definitions also retain ordered unique explicit-string `labels`,
including omission, explicit empty maps, deterministic parse-back output, and sensitivity. Literal
external volumes that retain labels have distinct source-aware diagnostics; external lifecycle
remains unavailable on the application-owned definition API.

Generated application-owned network definitions also retain optional literal `enable_ipv6` and
`internal` choices, including omission versus explicit `false` or `true`, without defaults or
driver/IPAM/provider/runtime validation. `enable_ipv4` remains deliberately absent from this
generated API because it has no native Quadlet/Podman network-create counterpart; BoxFerry owns
the non-representable diagnostic.

Generated config and secret definitions accept one unique resolved single-line name plus one
required resolved single-line `file` value. Their deterministic quoted mappings parse back through
the native model and propagate caller-marked file sensitivity. Content, environment, external
lifecycle, drivers, labels, template drivers, and file access remain deliberately outside this
minimal generated subset.

### Recurring generation-admission rule

Generated construction is demand-driven, not a remaining-key count. A maintainer may admit a
field only after its syntax-form choice, invalid-value rejection, sensitivity behavior,
deterministic rendering, and parse-back tests are defined. All other typed or preserved fields stay
intentionally ungenerated until a concrete consumer need justifies that review.

## Implementation order

### Phase 1: high-value process and lifecycle parity

- [x] Type `entrypoint` at document and project layers without conflating it with `command`, and
      add deterministic generated string, list, and empty forms.
- [x] Type `init` as a source-aware/interpolation-preserving boolean and add deterministic
      generated output.
- [x] Type independent `stop_grace_period` and `stop_signal` fields through generated output.
- [x] Type and generate raw-preserving `pull_policy` values while keeping schema-only `refresh`
      distinct and provider evidence planned.
- [x] Type `stdin_open` as an independent source-aware/interpolation-preserving boolean and add
      deterministic generated output.
- [x] Type `tty` as an independent source-aware/interpolation-preserving boolean and add
      deterministic generated output.
- [x] Type `privileged` as an independent source-aware/interpolation-preserving boolean and add
      deterministic generated output without inferring security or runtime behavior.
- [x] Type `attach` as an independent source-aware/interpolation-preserving boolean through the
      authored and effective views, without a default, generated API, logging, runtime, provider, CLI,
      compatibility, or cross-format behavior.
- [x] Type `pull_refresh_after` as a strict raw YAML string with deferred-value retention and no
      refresh, provider, or compatibility inference.
- [x] Type `runtime` as a strict raw YAML string with deferred-value retention and no provider or
      compatibility inference.
- [x] Type `platform` as a strict raw YAML string with deferred-value retention and no OCI, host,
      image, build, provider, or compatibility inference.
- Generation admission is a recurring maintainer rule: add a field only after its complete
  null/empty/short/long contract, validation, sensitivity, deterministic rendering, and
  parse-back tests are reviewed.

### Phase 2: limits, security, devices, and storage

- [x] Type service `pids_limit` through the authored, effective-project, and generated boundaries
      without normalizing zero or conflating it with `deploy.resources.limits.pids`.
- [x] Type service `shm_size` through the authored, effective-project, and generated boundaries
      with exact YAML scalar provenance, documented lowercase units, ambiguous zero, provider-dependent
      states, and no injected default or runtime inspection.
- [x] Type service `cap_drop` through authored, exact-scalar merge, effective-project, and generated
      boundaries while preserving explicit empty state, case, provenance, and planned-only provider
      evidence without target logic or a capability whitelist.
- [x] Type service `cap_add` through authored, exact-scalar merge, effective-project, and generated
      boundaries while preserving explicit empty state, case, provenance, independent coexistence
      with `cap_drop`, and planned-only provider evidence without target logic or a capability
      whitelist.
- [x] Promote `ulimits` through recursive mapping merge, the effective project view, and safe
      generated output while retaining ordered names, single/range form, nested provenance,
      sensitivity, empty/reset/override state, and planned-only provider evidence.
- [x] Promote the complete closed immediate Build-key family—`additional_contexts`, context, args, labels, Build-specific `extra_hosts`, raw `cache_from`/`cache_to`/`entitlements`, Dockerfile/inline Dockerfile/target/network/isolation/platforms/no_cache/privileged/sbom/pull/shm_size/tags, `ulimits`, and short/long `build.secrets`—with source form, sensitivity, provenance, recovery, and retained conflict evidence. Only malformed input, extensions, future unknown members, and deliberately bounded nested semantics remain unmodeled.
- [x] Promote `deploy.endpoint_mode`, map/list `deploy.labels`, `deploy.mode`, and raw-preserving `deploy.replicas`
      into the effective project view with provenance and nested unmodeled siblings; no container-label, integer/default,
      mode-coupling, scheduling, runtime, or conversion semantics are inferred before deepening further deploy types.
- [x] Promote deploy-specific `restart_policy` members through authored and effective views with
      raw condition/duration/attempt spelling, member provenance, and nested malformed/reset evidence;
      no service-restart fallback/default/precedence, simulation, runtime, or conversion behavior is
      inferred.
- [x] Promote deploy `placement` constraints, preferences, and max-replicas-per-node scalar
      categories through authored and effective views with nested provenance and recovery, without
      scheduling, node-selection, default, runtime, or conversion semantics.
- [x] Promote deploy `resources.limits.pids` through authored and effective views with exact
      integer/string spelling, nested provenance, recovery, and no service-PID, host, cgroup, runtime,
      or conversion semantics.
- [x] Promote deploy `resources.limits.cpus` through authored and effective views with exact
      number/string spelling, nested provenance, recovery, and no service CPU, `mem_limit`, host,
      cgroup, runtime, or conversion semantics.
- [x] Promote deploy `resources.limits.memory` through authored and effective views with raw
      YAML-string spelling, conservative lowercase-unit classification, nested provenance, recovery,
      and no service `mem_limit`, reservation, host, cgroup, runtime, or conversion semantics.
- [x] Promote deploy `resources.reservations.cpus` through authored and effective views with exact
      number/string spelling, nested provenance, recovery, and no limit/service CPU, scheduling, host,
      cgroup, runtime, target, or conversion semantics.
- [x] Promote deploy `resources.reservations.memory` through authored and effective views with raw
      YAML-string spelling, conservative lowercase-unit classification, nested provenance, recovery,
      and no limit/service `mem_limit`, scheduling, host, cgroup, runtime, target, or conversion semantics.
- [x] Promote schema-backed deploy `resources.reservations.generic_resources` lists with optional
      discrete-specification kind/value members, raw scalar spelling, provenance, recovery, and no
      matching, scheduling, GPU/device, runtime, target, or conversion semantics.
- [x] Promote schema-only deploy `resources.reservations.devices[]` capability lists, strict
      YAML-string `.driver`, raw integer-or-string `.count`, and ordered strict-string `.device_ids`
      with duplicate/conflict diagnostics, nested provenance, recovery, and no selection/loading,
      allocation, grammar, scheduling, CDI, host, runtime, cgroup, provider/version, or conversion
      semantics. Options retain map/list syntax, scalar fidelity, malformed evidence, duplicates, and
      generic provenance without provider interpretation.
- [x] Type all CPU, memory, PID, OOM, and block-I/O keys without applying host defaults.
- [x] Type service `devices` through authored, Compose-Go-compatible target merge,
      effective-project, and generated boundaries while preserving mixed raw short/long forms,
      CDI/deferred/opaque evidence, duplicates, nested provenance, reset/override, and planned-only
      provider evidence without device, permissions, CDI, GPU, or runtime validation.
- [x] Type immediate service `gpus`, `storage_opt`, and UTS namespace choices through authored
      and effective-project views without device allocation, label-file I/O, or runtime inference.
- Deploy reservation devices are complete as bounded structured source data: capabilities, driver,
  count, IDs, and options remain source evidence rather than allocation requests. Allocation,
  selection, and provider/runtime behavior stay consumer-led unless a separately reviewed ADR
  establishes a narrower native contract.
- [x] Type service-level `tmpfs` through authored, ordinary-append merge, effective-project, and
      generated boundaries while preserving scalar/list form, duplicates, colon-delimited raw options,
      provenance, sensitivity, reset/override, and planned-only provider evidence.
- [x] Type service `sysctls` through authored, generic map/list merge, effective-project, and
      generated boundaries while preserving form, scalar spelling, duplicate evidence, provenance,
      sensitivity, reset/override, and planned-only provider evidence without runtime interpretation.
- [x] Type `volumes_from` through authored, effective-project, reference-validation, and generated
      boundaries while preserving order, duplicates, reset/override provenance, and raw access modes.
- [x] Type the remaining long service-volume members through authored and effective views:
      `consistency`, `bind.recursive`, and the `image`, `tmpfs`, and `volume` blocks including
      `image.subpath`, `tmpfs.size`, `tmpfs.mode`, `volume.labels`, `volume.nocopy`, and
      `volume.subpath`. Generation remains intentionally out of scope. `tmpfs.mode` carries a
      Compose v2.14 badge and `image.subpath` a Compose v2.35 badge; all other availability
      boundaries remain unknown. No path, image, volume, permission, provider, runtime, default,
      or filesystem behavior is inferred.

### Phase 3: networking, identity, and metadata

- [x] Type and generate service `hostname` with conservative RFC-1123 validation, deferred and
      invalid authored states, complete merge provenance, and planned-only provider evidence.
- [x] Type and generate service DNS settings with their documented merge rules and raw evidence.
- [x] Type and generate exposed ports with scalar-kind-aware uniqueness.
- [x] Generate optional raw per-attachment IPv4/IPv6 addresses in deterministic long-form service
      networks without inferring IPAM defaults or validating address/pool relationships.
- [x] Generate top-level network drivers and scalar-kind-aware ordered driver options without
      changing the shared basic/external resource API, while keeping external networks on its
      name-only-compatible path and not validating plugins or provider semantics.
- [x] Preserve and generate raw service security options with non-selecting diagnostic candidates.
- [x] Type network modes through authored, effective-project, reference-validation, and generated
      boundaries without inferring runtime namespaces or provider behavior.
- [x] Type domain name, MAC addresses, external links, and links through authored and effective
      project views without network or runtime resolution.
- [x] Type service annotations through authored mapping/list syntax, keyed effective merge,
      provenance-preserving diagnostics, and safe generated mapping output.
- [x] Type service `logging` through authored, recursively merged, effective-project, and generated
      boundaries with uninterpreted drivers, ordered string/number/null options, and no provider policy.
- [x] Type `label_file` scalar/list syntax with source and merge provenance; label files remain
      unread. Config/secret metadata stays in the nested-resource ledger.
- Provider spellings are complete as raw preservation with a bounded current evidence audit; this
  is not a blanket compatibility claim. Future provider/version observations are added only
  when they affect a supported contract or explain a concrete regression.

### Phase 4: orchestration and processing-only features

- [x] Implement the bounded top-level `include` traversal: caller-authorized recursive loading,
      effective no-interpolation ordering, origins, partial graph diagnostics, cycles, and global
      source-ID rejection.
- [x] Compose included resources without I/O: recursive local-wins imports for all six top-level
      namespaces, explicit conflicts, and retained occurrence/source evidence.
- [x] Plan include project directories without I/O: first-document defaults, explicit caller-owned
      resolver outcomes, nested parent propagation, and redacted source-aware unresolved errors.
- [x] Resolve selected included service bind sources plus config and secret `file` paths lexically
      from authorized occurrence bases, with explicit unavailable/mismatched-plan diagnostics and
      no root fallback.
- Other path families—including build, `env_file`, `label_file`, `extends.file`, develop-watch,
  include loading, URI/non-local policy, and composed rendering—are explicitly deferred to a
  field-specific consumer need and ADR. Environment-file/`.env` precedence, project-name rules,
  and provider-specific behavior remain separate application or evidence decisions.
- [x] Type `develop`, service/top-level `models`, and `use_api_socket` without implying that every
      provider executes them.
- Parser I/O purity is a permanent invariant: file reads, environment access, and provider/runtime
  invocation remain outside parsing APIs and are protected by focused regression and policy
  tests.

### Phase 5: generation, compatibility, and conformance

- Generated expansion is recurring and demand-driven: admit only consumer-needed fields under the
  generation-admission rule above.
- The current Docker Compose and Podman Compose evidence audit is complete for supported claims;
  it makes no blanket provider/version compatibility claim. New evidence is collected only for
  a supported behavior change, contradiction, or regression.
- Real-world corpus intake is recurring and regression-driven: add a minimal licensed immutable
  fixture only when it captures a concrete missing behavior or prevents a regression.
- [x] Maintain the commit-pinned root/service schema snapshot and a classified inventory with
      offline digest, closed-shape, extension-allowance, and exact-key-set policy tests. Scheduled
      upstream drift reporting is manual/scheduled only. Nested-schema drift is a recurring intake
      boundary and expands only when a concrete changed field enters the supported contract.

## Completion rule

A key is complete only when the repository documents its syntax forms, exposes source-aware native
types, implements effective merge/profile behavior where applicable, tests malformed recovery and
provenance, and separately records generation and provider compatibility status. Syntax
preservation alone is valuable, but never counts as semantic completion.
