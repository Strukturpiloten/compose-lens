# Roadmap

This roadmap orders ComposeLens work by consumer value and records the exact current specification
surface that is not yet available through a native typed API. It is not a delivery schedule.

## Coverage language

ComposeLens has several independent coverage layers:

| State | Meaning |
| --- | --- |
| Syntax-preserved | The YAML parser retains the key, spelling, order, source range, extensions, and unknown fields where recoverable. |
| Document typed | One authored document exposes a native source-aware value. |
| Project typed | The effective merged/profile-selected project exposes the value with provenance. |
| Generated | New Compose YAML can be constructed through typed values and parse-back validation. |
| Compatibility-evidenced | Provider/runtime/version behavior is backed by documented or observed evidence. |

“Not typed” does not mean “cannot be read.” Every key below remains available through the
loss-aware syntax document. It means callers cannot yet consume that key through a dedicated
ComposeLens semantic type.

## Specification snapshot

This ledger was audited on 2026-08-06 against the current official
[Compose JSON schema](https://github.com/compose-spec/compose-spec/blob/master/schema/compose-spec.json),
[Compose Specification](https://github.com/compose-spec/compose-spec/blob/main/spec.md), and
[Docker Compose file reference](https://docs.docker.com/reference/compose-file/).
Provider additions remain eligible when real Docker Compose or Podman Compose accepts them; the
compose-spec repository is not the sole compatibility authority.

The audited schema currently contains 9 top-level keys and 93 service keys.

| Surface | Project typed | Document typed only | Syntax-preserved only |
| --- | ---: | ---: | ---: |
| Top level | 6 | 0 | 3 |
| Service | 34 | 3 | 56 |

`x-*` extensions are intentionally open-ended and preserved. They are not counted as missing
closed-schema keys.

## Exact top-level gaps

The following current top-level keys are syntax-preserved but have no native project type:

- `version` — obsolete but retained for backward compatibility;
- `include` — including long-form `path`, `env_file`, and `project_directory`; and
- `models` — including definition keys `name`, `model`, `context_size`, and `runtime_flags`.

`name`, `services`, `networks`, `volumes`, `configs`, and `secrets` are document- and
project-typed.

## Exact service gaps

### Project-typed service keys

The effective project view currently exposes:

`annotations`, `command`, `configs`, `container_name`, `depends_on`, `entrypoint`, `env_file`,
`environment`, `extra_hosts`, `cap_add`, `cap_drop`, `devices`, `dns`, `dns_opt`, `dns_search`,
`expose`, `group_add`, `healthcheck`, `hostname`, `image`, `init`, `labels`, `networks`, `ports`,
`profiles`, `read_only`, `pids_limit`, `pull_policy`, `restart`, `secrets`, `security_opt`,
`shm_size`, `stop_grace_period`, `stop_signal`, `sysctls`, `tmpfs`, `ulimits`, `user`,
`userns_mode`, `volumes`, and `working_dir`.

### Document-only service keys

These keys are source-aware in one document but are not yet effective-project values:

- `build` — all current immediate subkeys are recognized as field references:
  `additional_contexts`, `args`, `cache_from`, `cache_to`, `context`, `dockerfile`,
  `dockerfile_inline`, `entitlements`, `extra_hosts`, `isolation`, `labels`, `network`,
  `no_cache`, `no_cache_filter`, `platforms`, `privileged`, `provenance`, `pull`, `sbom`,
  `secrets`, `shm_size`, `ssh`, `tags`, `target`, and `ulimits`;
- `deploy` — all current immediate subkeys are recognized as field references: `endpoint_mode`,
  `labels`, `mode`, `placement`, `replicas`, `resources`, `restart_policy`, `rollback_config`, and
  `update_config`.

Build and deploy field recognition is not a claim that every nested value is semantically typed.

### Syntax-preserved-only service keys

The following 50 current service keys do not yet have a dedicated typed model:

`attach`, `blkio_config`, `cgroup`, `cgroup_parent`,
`cpu_count`, `cpu_percent`, `cpu_period`, `cpu_quota`, `cpu_rt_period`, `cpu_rt_runtime`,
`cpu_shares`, `cpus`, `cpuset`, `credential_spec`, `develop`, `device_cgroup_rules`,
`domainname`, `extends`,
`external_links`, `gpus`, `ipc`, `isolation`, `label_file`, `links`,
`logging`, `mac_address`, `mem_limit`, `mem_reservation`, `mem_swappiness`, `memswap_limit`,
`models`, `network_mode`, `oom_kill_disable`, `oom_score_adj`, `pid`, `platform`,
`post_start`, `pre_start`, `pre_stop`, `privileged`, `provider`, `pull_refresh_after`, `runtime`,
`scale`, `stdin_open`,
`storage_opt`, `tty`, `use_api_socket`,
`uts`, and `volumes_from`.

## Nested resource gaps

Current top-level network and volume definition keys are typed, including network IPAM fields.
The remaining current resource-definition gaps are:

- config definitions: `labels` and `template_driver`;
- secret definitions: `driver`, `driver_opts`, `labels`, and `template_driver`; and
- every top-level model definition key, because top-level `models` is not typed yet.

The schema's legacy object form `external: { name: ... }` is also not typed for network, volume,
config, or secret definitions. The ordinary sibling `name` field is typed for all four resource
definitions.

The existing service `ports`, `volumes`, `networks`, `configs`, and `secrets` types retain their
documented short/long forms. Future schema additions must first enter this ledger before support is
claimed.

## Exact nested semantic gaps

The 93-key service ledger above classifies immediate service keys. The following closed nested
keys also remain without dedicated semantic value types. Open-ended user maps such as labels,
environment variables, driver options, and extension fields are intentionally not enumerated.

- `blkio_config`: `device_read_bps`, `device_read_iops`, `device_write_bps`,
  `device_write_iops`, `weight`, and `weight_device`; each rate entry has `path` and `rate`, and
  each weight entry has `path` and `weight`;
- `credential_spec`: `config`, `file`, and `registry`;
- GPU/device-reservation entries: `capabilities`, `count`, `device_ids`, `driver`, and `options`;
- `extends`: `file` and `service`;
- `logging`: `driver` and `options`;
- service `models` entries: `endpoint_var` and `model_var`;
- `provider`: `options` and `type`;
- long volume mounts: `consistency`, `image`, `tmpfs`, and `volume`; additionally
  `bind.recursive`, `image.subpath`, `tmpfs.mode`, `tmpfs.size`, `volume.labels`,
  `volume.nocopy`, and `volume.subpath` are not typed. The other current long-mount and bind
  fields are typed;
- `develop.watch[]`: `action`, `exec`, `ignore`, `include`, `initial_sync`, `path`, and `target`;
  the nested `exec` hook has `command`, `environment`, `privileged`, `user`, and `working_dir`;
- `post_start[]` and `pre_stop[]` hooks: `command`, `environment`, `privileged`, `user`, and
  `working_dir`;
- `pre_start[]` hooks: `command`, `environment`, `image`, `per_replica`, `privileged`, `user`,
  and `working_dir`.

`build` and `deploy` currently recognize their immediate keys as source field references, not as
semantic value types. Their complete closed-key boundary is therefore still open:

- `build`: `additional_contexts`, `args`, `cache_from`, `cache_to`, `context`, `dockerfile`,
  `dockerfile_inline`, `entitlements`, `extra_hosts`, `isolation`, `labels`, `network`,
  `no_cache`, `no_cache_filter`, `platforms`, `privileged`, `provenance`, `pull`, `sbom`,
  `secrets`, `shm_size`, `ssh`, `tags`, `target`, and `ulimits`;
- `deploy`: `endpoint_mode`, `labels`, `mode`, `placement`, `replicas`, `resources`,
  `restart_policy`, `rollback_config`, and `update_config`;
- `deploy.placement`: `constraints`, `max_replicas_per_node`, and `preferences[].spread`;
- `deploy.resources.limits`: `cpus`, `memory`, and `pids`;
- `deploy.resources.reservations`: `cpus`, `devices`, `generic_resources`, and `memory`;
- `deploy.resources.reservations.devices[]`: `capabilities`, `count`, `device_ids`, `driver`,
  and `options`;
- `deploy.resources.reservations.generic_resources[].discrete_resource_spec`: `kind` and
  `value`;
- `deploy.restart_policy`: `condition`, `delay`, `max_attempts`, and `window`; and
- both `deploy.rollback_config` and `deploy.update_config`: `delay`, `failure_action`,
  `max_failure_ratio`, `monitor`, `order`, and `parallelism`.

Conversely, the current nested keys under `depends_on`, `env_file`, `healthcheck`, service
`networks`, service `ports`, service config/secret grants, network `ipam`, and service `ulimits`
already have dedicated document types. This distinction prevents an immediate parent key from
hiding a nested semantic gap.

## Generated-document boundary

Generated documents currently cover project `name`, services, networks, and volumes. Generated
services cover `hostname`, `container_name`, `image`, `entrypoint`, `command`, `init`, `env_file`, `environment`, `labels`, `annotations`, `user`,
`userns_mode`, `group_add`, `cap_add`, `cap_drop`, `working_dir`, `read_only`, `pids_limit`, `shm_size`, `tmpfs`, `sysctls`, `ulimits`, `pull_policy`, `restart`, `stop_signal`,
`stop_grace_period`, `extra_hosts`, `ports`,
`volumes`, and `networks`.

All other typed or preserved service/resource keys remain open for generated construction. A key
is generated only after syntax-form choice, validation, sensitivity, deterministic rendering, and
parse-back tests are defined.

## Implementation order

### Phase 1: high-value process and lifecycle parity

- [x] Type `entrypoint` at document and project layers without conflating it with `command`, and
  add deterministic generated string, list, and empty forms.
- [x] Type `init` as a source-aware/interpolation-preserving boolean and add deterministic
  generated output.
- [x] Type independent `stop_grace_period` and `stop_signal` fields through generated output.
- [x] Type and generate raw-preserving `pull_policy` values while keeping schema-only `refresh`
  distinct and provider evidence planned.
- [ ] Type `stdin_open` and `tty`.
- [ ] Type `pull_refresh_after`, `platform`, and `runtime` with deferred-value retention and
  provider-specific compatibility evidence.
- [ ] Add generated construction only after each field's null/empty/short/long behavior is fixed.

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
- [ ] Promote `build` and `deploy` into the effective project view before deepening their semantic
  types.
- [ ] Type all CPU, memory, PID, OOM, and block-I/O keys without applying host defaults.
- [x] Type service `devices` through authored, Compose-Go-compatible target merge,
  effective-project, and generated boundaries while preserving mixed raw short/long forms,
  CDI/deferred/opaque evidence, duplicates, nested provenance, reset/override, and planned-only
  provider evidence without device, permissions, CDI, GPU, or runtime validation.
- [ ] Type GPU reservations, `gpus`, `privileged`, `credential_spec`, `storage_opt`,
  cgroup, IPC, PID, and UTS namespace choices.
- [x] Type service-level `tmpfs` through authored, ordinary-append merge, effective-project, and
  generated boundaries while preserving scalar/list form, duplicates, colon-delimited raw options,
  provenance, sensitivity, reset/override, and planned-only provider evidence.
- [x] Type service `sysctls` through authored, generic map/list merge, effective-project, and
  generated boundaries while preserving form, scalar spelling, duplicate evidence, provenance,
  sensitivity, reset/override, and planned-only provider evidence without runtime interpretation.
- [ ] Type `volumes_from` and remaining mount-specific nested semantics.

### Phase 3: networking, identity, and metadata

- [x] Type and generate service `hostname` with conservative RFC-1123 validation, deferred and
  invalid authored states, complete merge provenance, and planned-only provider evidence.
- [x] Type and generate service DNS settings with their documented merge rules and raw evidence.
- [x] Type and generate exposed ports with scalar-kind-aware uniqueness.
- [x] Preserve and generate raw service security options with non-selecting diagnostic candidates.
- [ ] Type domain name, MAC addresses, network modes,
  external links, and links.
- [x] Type service annotations through authored mapping/list syntax, keyed effective merge,
  provenance-preserving diagnostics, and safe generated mapping output.
- [ ] Type `label_file`, logging, and remaining config/secret metadata fields.
- [ ] Preserve provider/runtime-specific value spellings and attach compatibility evidence instead
  of enforcing one implementation's grammar globally.

### Phase 4: orchestration and processing-only features

- [ ] Implement top-level `include` as explicit caller-authorized project loading with cycle,
  provenance, project-directory, and environment-file rules.
- [ ] Type `extends` and define its processing order relative to include, interpolation, and merge.
- [ ] Type `develop`, lifecycle hooks, `provider`, service/top-level `models`, `scale`, and
  `use_api_socket` without implying that every provider executes them.
- [ ] Keep file reads, environment access, and provider invocation outside parsing APIs.

### Phase 5: generation, compatibility, and conformance

- [ ] Expand generated documents in the same order as project-typed consumer demand.
- [ ] Add Docker Compose and Podman Compose provider/version evidence for promoted keys.
- [ ] Promote real-world corpus gaps into minimal licensed fixtures.
- [ ] Add a maintained schema-audit manifest and a policy test that fails when official closed-key
  inventories change without a roadmap classification.

## Completion rule

A key is complete only when the repository documents its syntax forms, exposes source-aware native
types, implements effective merge/profile behavior where applicable, tests malformed recovery and
provenance, and separately records generation and provider compatibility status. Syntax
preservation alone is valuable, but never counts as semantic completion.
