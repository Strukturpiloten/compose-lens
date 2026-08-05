# Native Compose coverage

This document distinguishes source preservation from typed consumer coverage. It was audited
against the current official
[Compose service reference](https://docs.docker.com/reference/compose-file/services/) on
2026-08-05.

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
| Document model and project view | `image`, `command`, `environment`, `labels`, `extra_hosts`, `user`, `userns_mode`, `group_add`, `working_dir`, `read_only`, `healthcheck`, `depends_on`, `ports`, `volumes`, `networks`, `profiles`, `configs`, `secrets` |
| Document model only | `build`, `ulimits`, `deploy` |
| Preserved, not typed | `annotations`, `attach`, `blkio_config`, CPU controls, `cap_add`, `cap_drop`, `cgroup`, `cgroup_parent`, `container_name`, `credential_spec`, `develop`, `device_cgroup_rules`, `devices`, `dns`, `dns_opt`, `dns_search`, `domainname`, service `driver_opts`, `entrypoint`, `env_file`, `expose`, `extends`, `external_links`, `gpus`, `hostname`, `init`, `ipc`, `isolation`, `label_file`, `links`, `logging`, `mac_address`, memory controls, `models`, `network_mode`, OOM controls, `pid`, `pids_limit`, `platform`, lifecycle hooks, `privileged`, `provider`, `pull_policy`, `restart`, `runtime`, `scale`, `security_opt`, `shm_size`, `stdin_open`, `stop_grace_period`, `stop_signal`, `storage_opt`, `sysctls`, `tmpfs`, `tty`, `use_api_socket`, `uts`, `volumes_from` |

The preserved row follows the current Docker documentation grouping. Provider-specific additions
remain preserved even when they are not part of the compose-spec repository.

## Current top-level boundary

`name`, `services`, `networks`, `volumes`, `configs`, and `secrets` have both document-model and
project-view support. Their implemented definition fields are listed in
[Typed Compose model](typed-model.md). Other top-level or nested values remain source-addressable
and appear as typed field references where the current boundary supports them.

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

Restart policy, entrypoint behavior, and resource limits are the next high-value promotion groups.

## Promotion checklist

A service field moves into the project view only with:

1. correct Compose merge behavior;
2. source provenance for the field and meaningful nested values;
3. malformed-form diagnostics and partial recovery;
4. multi-file tests; and
5. an additive public API that keeps authored forms distinct where semantics can differ.
