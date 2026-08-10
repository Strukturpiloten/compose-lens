# ComposeLens

ComposeLens is a Rust library for reading, understanding, transforming, validating, and rendering real-world Compose documents.

It is designed for tools that need more than strict deserialization: source-aware diagnostics, implementation extensions, optional interpolation, multi-file projects, profiles, and round-trip-safe transformations.

## Goals

- Parse Compose YAML without forcing immediate normalization or interpolation.
- Represent syntax, typed Compose concepts, extensions, and unknown fields explicitly.
- Preserve field-specific short and long syntax when their defaults or runtime behavior can differ.
- Preserve enough source information for actionable diagnostics and safe editing.
- Support multi-file project loading, merging, profile selection, and configurable interpolation.
- Model behavior found in real Docker Compose and Podman Compose projects.
- Render deterministic Compose documents.
- Construct new deterministic Compose documents through Compose-owned, parse-back-validated values.
- Preserve, merge, inspect, and generate service metadata labels without exposing sensitive values
  through debug output.
- Preserve, merge, inspect, and generate explicit runtime container names with field provenance.
- Preserve, merge, inspect, and generate explicit service hostnames with conservative RFC-1123
  validation, deferred-expression retention, and no synthesized default.
- Preserve, merge, inspect, and generate service-level restart policies without confusing them
  with dependency-update or deploy restart settings.
- Preserve, merge, inspect, and generate service `stdin_open` choices without inferring terminal,
  runtime, or cross-format behavior.
- Preserve, merge, inspect, and generate service `tty` choices without inferring terminal,
  runtime, or cross-format behavior.
- Preserve, merge, inspect, and generate service `privileged` choices without inferring security,
  runtime, or cross-format behavior.
- Preserve, merge, and inspect service `attach` choices without a default, generated API, logging,
  runtime, provider, CLI, compatibility, or cross-format behavior.
- Preserve authored and effective service `blkio_config` scalar spelling and ordered device entries
  without defaults, controller, runtime, provider, I/O, or cross-format interpretation.
- Preserve authored and effective service `cgroup` namespace spelling, validity classification, and
  provenance without defaults, controller, runtime, provider, I/O, or cross-format interpretation.
- Preserve authored and effective raw service `cgroup_parent` strings with source and merge
  provenance without path, controller, runtime, provider, or cross-format interpretation.
- Preserve authored and effective service `cpu_count` YAML integer/string categories and exact
  spelling without numeric conversion, quota, host, runtime, provider, or cross-format interpretation.
- Preserve authored and effective service `cpu_percent` YAML integer/string categories, including
  out-of-range integer evidence, without percentage calculation, CPU, host, runtime, provider, or
  cross-format interpretation.
- Preserve authored and effective service `cpu_period` YAML number/string categories without numeric,
  duration, CPU, host, runtime, provider, or cross-format interpretation.
- Preserve authored and effective service `cpu_quota` YAML number/string categories without numeric,
  quota, duration, CPU, host, runtime, provider, or cross-format interpretation.
- Preserve authored and effective service `cpu_rt_period` YAML number, duration, expression, and
  other-string categories without CPU, scheduler, host, runtime, provider, or cross-format interpretation.
- Preserve, merge, and inspect service image pull policies and raw `pull_refresh_after` strings
  without inventing provider behavior, refresh semantics, or defaults.
- Preserve, merge, inspect, and generate independent service stop signals and raw Compose stop
  grace periods without normalizing them into another lifecycle manager's units.
- Preserve, merge, inspect, and generate service PID limits without fixed-width parsing, default
  injection, runtime probing, or conflation with deploy resource limits.
- Preserve, merge, inspect, and safely generate service shared-memory sizes without injecting the
  Podman default, normalizing provider-dependent values, or inspecting `/dev/shm`.
- Preserve, merge, inspect, and safely generate service memory limits without fixed-width parsing,
  provider/runtime enforcement, host/cgroup inspection, or conflation with deploy memory policy.
- Preserve, merge, inspect, and generate service-level `tmpfs` scalar/list forms, colon-delimited
  documented or raw options, duplicates, provenance, and sensitivity without conflating volume mounts.
- Preserve, merge, inspect, and safely generate service `sysctls` mapping/list forms, scalar
  spelling, order, provenance, and sensitivity without namespace or runtime interpretation.
- Preserve, recursively merge, inspect, and safely generate ordered service `ulimits` single and
  soft/hard forms without injecting defaults or claiming runtime enforcement.
- Preserve, merge, inspect, and generate independent ordered service `cap_add` and `cap_drop`
  strings with explicit empty state, exact-case uniqueness, provenance, and no capability
  whitelist or target normalization.
- Preserve, target-key merge, inspect, and safely generate ordered mixed short/long service
  `devices`, including raw CDI/deferred/opaque short forms, explicit empty state, duplicates,
  nested provenance, and no host-device, permissions, or runtime-access validation.
- Preserve, merge, inspect, and safely generate raw service DNS settings and exposed ports without
  resolver, network, or runtime access.
- Preserve and safely generate keyed service annotations without losing mapping/list syntax or
  ambiguous key-only entries.
- Preserve, recursively merge, inspect, and safely generate service logging drivers and ordered
  string/number/null options without interpreting provider semantics.
- Preserve Build `no_cache` YAML boolean/string distinctions, interpolation provenance, and
  recovery without inferring defaults, builder behavior, or cache behavior.
- Preserve Build `sbom` YAML boolean/string distinctions, interpolation provenance, and recovery
  without parsing generators, exposing generated SBOM data, or inferring builder behavior.
- Preserve Build `privileged` literal booleans and deferred expressions through authored and
  effective views without inferring privilege, platform, runtime, or build behavior.
- Preserve sensitive BuildKit `build.ssh` mapping/list forms, complete provenance, and redacted
  inspection without parsing grants or accessing sockets, agents, files, or a builder.
- Preserve opaque Build `isolation` YAML strings with interpolation, provenance, and recovery
  without validating modes, platforms, privileges, or builder behavior.
- Preserve raw ordered Build `cache_from` and `cache_to` descriptors with source spans,
  interpolation provenance, and generic sequence merge behavior without interpreting cache types
  or locations.
- Preserve opaque ordered Build `entitlements` strings with interpolation, provenance, and
  recovery without inferring allowlists, privilege state, BuildKit/platform support, execution,
  or runtime effect.
- Preserve exact Build `dockerfile_inline` string scalars with source spans, interpolation,
  provenance, recovery, and conflict diagnostics without parsing Containerfiles or building.
- Preserve Build `shm_size` through authored and effective views with the same raw scalar,
  lowercase-unit, zero, deferred-expression, and provider-dependent states as service
  `shm_size`, without default, host, allocation, or builder inference.
- Preserve Build `ulimits` through authored and effective views with the same ordered single and
  soft/hard forms, scalar spelling, recursive merge, and source evidence as service `ulimits`,
  without defaults, normalization, host-limit validation, or builder/runtime inference.
- Preserve Build-specific `extra_hosts` list/map forms, raw host/address spelling, nested address
  lists, interpolation provenance, and generic merge evidence without conflating service hosts or
  performing address validation, DNS/host access, build generation, or conversion.
- Preserve Deploy `endpoint_mode`, `mode`, raw `replicas` scalars, and distinct map/list deployment labels through
  authored and effective views while retaining malformed, extension, and future-unknown deploy evidence and inferring
  no count, container, platform, discovery, or runtime behavior.
- Preserve deploy restart-policy members with raw condition, duration, and attempt spellings without service-restart
  defaults, precedence, simulation, or runtime interpretation.
- Preserve deploy update_config member spelling, malformed evidence, and merge provenance without rollout,
  scheduling, runtime, or conversion interpretation.
- Preserve distinct deploy rollback_config member spelling, malformed evidence, and merge provenance without rollout,
  scheduling, runtime, or conversion interpretation.
- Preserve deploy placement constraints, preferences, and max-replicas-per-node spelling through
  authored and effective views with append/reset/override provenance, malformed recovery, and no
  scheduling, node-selection, default, runtime, or conversion interpretation.
- Preserve deploy resource-limit CPU number/string, memory string, and PID integer/string spelling,
  plus reservation CPU number/string and memory string spelling, through authored and effective views with nested
  merge/reset/override provenance and no service, host, cgroup, runtime, or conversion interpretation.
- Preserve schema-backed reservation generic-resource lists with raw nested kind/value spelling,
  collection/item/member provenance, and no scheduling, device, runtime, or conversion interpretation.
- Preserve schema-only reservation device lists with raw counts, IDs, map/list options, capabilities,
  drivers, conflict and malformed evidence, merge provenance, and no device selection, runtime, or conversion interpretation.
- Preserve service credential-spec mappings and raw config/file/registry references through authored and effective
  views without resolving configurations, files, registries, accounts, platforms, or runtime behavior.
- Preserve raw service `extends` short references and long service/file mappings through authored
  and effective views, including provenance and recovery, without expanding referenced services or
  resolving files, paths, cycles, resources, providers, platforms, runtimes, or conversion behavior.
- Preserve raw service provider mappings with strict-string types, scalar/sequence options,
  provenance, and recovery without execution, discovery, provider grammar, compatibility, or
  conversion behavior.
- Preserve ordered service `post_start`, `pre_stop`, and `pre_start` hooks plus raw `runtime`,
  `pull_refresh_after`, and `platform` strings with provenance and recovery without executing,
  scheduling, or otherwise interpreting lifecycle, refresh, or OCI behavior.
- Preserve raw service security options and expose conflict-aware lexical candidates without
  selecting policy, inspecting the host, or claiming runtime behavior.
- Preserve, inspect, and generate ordered service `env_file` short/long syntax and options; retain
  interpolation and multi-file provenance without reading environment files.
- Allow callers to choose strict, implementation-specific, or tolerant validation profiles.

## Non-goals

- Running a Compose project
- Reimplementing Docker or Podman
- Converting Compose directly to Quadlet or Kubernetes
- Enforcing OCI rules that real Compose implementations do not enforce
- Treating the Compose Specification as the only source of real-world behavior

Cross-format conversion belongs to [BoxFerry](https://github.com/Strukturpiloten/boxferry). Quadlet handling belongs to [QuadletLens](https://github.com/Strukturpiloten/quadlet-lens).

## Processing levels

```text
source text
  → loss-aware syntax document
  → typed Compose document
  → loaded multi-file project
  → optional per-file interpolation
  → provenance-preserving merged project
  → optional profile selection and native project view
  → references, paths, defaults, and compatibility
  → validated semantic view
  → rendered Compose document
```

Callers may stop at any appropriate level. Parsing a document must not implicitly read environment variables or contact a runtime.

## Documentation

- [Published Rust API documentation](https://docs.rs/compose-lens/latest/compose_lens/)
- [Documentation index](docs/README.md)
- [Software architecture](docs/architecture.md)
- [Target project structure](docs/project-structure.md)
- [Processing model](docs/processing-model.md)
- [Preservation-oriented editing](docs/preservation-editing.md)
- [Render formatting](docs/render-formatting.md)
- [Generated documents](docs/generated-rendering.md)
- [Phase 2 typed model](docs/typed-model.md)
- [Native coverage](docs/coverage.md)
- [Roadmap and exact specification gaps](docs/roadmap.md)
- [Testing strategy](docs/testing.md)
- [Development environment](docs/development-environment.md)
- [Compose implementation conformance](docs/conformance.md)
- [Real-world fixture corpus](docs/real-world-corpus.md)
- [Cross-repository implementation plan](docs/implementation-plan.md)
- [API stability policy](docs/api-stability.md)
- [0.1.16 release notes](docs/releases/0.1.16.md) — Build, Deploy, source-aware service configuration, and generated network/volume support
- [0.1.15 release notes](docs/releases/0.1.15.md) — DNS, expose, annotations, and security options
- [0.1.14 release notes](docs/releases/0.1.14.md)
- [Release process](docs/releasing.md)
- [Changelog](CHANGELOG.md)
- [Architecture decisions](docs/decisions/README.md)

Repository-specific guidance for coding agents is in [AGENTS.md](AGENTS.md).

## Origin

ComposeLens is implemented from scratch. It is not a fork of `compose_spec_rs` and does not copy or mechanically translate its source code.

## Stewardship

ComposeLens is created and maintained by [Martin “Becks” Beckert](https://github.com/TheRealBecks) through [Strukturpiloten OHG](https://www.strukturpiloten.de/). The project is part of Strukturpiloten's work on open, maintainable, and portable container infrastructure.

## License

ComposeLens is licensed under the [Mozilla Public License 2.0](LICENSE).
