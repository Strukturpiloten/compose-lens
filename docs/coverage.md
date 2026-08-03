# Native Compose coverage

This document distinguishes source preservation from typed consumer coverage. It was audited
against the current official
[Compose service reference](https://docs.docker.com/reference/compose-file/services/) on
2026-08-03.

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
| Document model and project view | `image`, `command`, `environment`, `extra_hosts`, `healthcheck`, `ports`, `volumes`, `networks`, `profiles` |
| Document model only | `build`, `user`, `userns_mode`, `ulimits`, `depends_on`, `deploy`, service `configs`, service `secrets` |
| Preserved, not typed | `annotations`, `attach`, `blkio_config`, CPU controls, `cap_add`, `cap_drop`, `cgroup`, `cgroup_parent`, `container_name`, `credential_spec`, `develop`, `device_cgroup_rules`, `devices`, `dns`, `dns_opt`, `dns_search`, `domainname`, service `driver_opts`, `entrypoint`, `env_file`, `expose`, `extends`, `external_links`, `gpus`, `group_add`, `hostname`, `init`, `ipc`, `isolation`, service `labels`, `label_file`, `links`, `logging`, `mac_address`, memory controls, `models`, `network_mode`, OOM controls, `pid`, `pids_limit`, `platform`, lifecycle hooks, `privileged`, `provider`, `pull_policy`, `read_only`, `restart`, `runtime`, `scale`, `security_opt`, `shm_size`, `stdin_open`, `stop_grace_period`, `stop_signal`, `storage_opt`, `sysctls`, `tmpfs`, `tty`, `use_api_socket`, `uts`, `volumes_from`, `working_dir` |

The preserved row follows the current Docker documentation grouping. Provider-specific additions
remain preserved even when they are not part of the compose-spec repository.

## Current top-level boundary

`name`, `services`, `networks`, `volumes`, `configs`, and `secrets` have both document-model and
project-view support. Their implemented definition fields are listed in
[Typed Compose model](typed-model.md). Other top-level or nested values remain source-addressable
and appear as typed field references where the current boundary supports them.

## Next promotion

Health checks now have a project view that retains field-level merge provenance and keeps
Compose's `start_interval` distinct from target features with different semantics.

Dependency conditions are the next promotion because `service_healthy` needs a health-aware target
plan. Identity, restart policy, entrypoint/working-directory behavior, and resource limits are the
next high-value groups after that vertical slice.

## Promotion checklist

A service field moves into the project view only with:

1. correct Compose merge behavior;
2. source provenance for the field and meaningful nested values;
3. malformed-form diagnostics and partial recovery;
4. multi-file tests; and
5. an additive public API that keeps authored forms distinct where semantics can differ.
