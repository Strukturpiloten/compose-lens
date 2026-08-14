# Testing strategy

ComposeLens must be built around tests because YAML syntax, Compose processing, and implementation behavior contain many interacting edge cases.

## Investment boundary

Pull-request checks must remain deterministic and reasonably fast. New behavior should normally
have focused positive and negative coverage, including malformed input, merge/reset behavior,
public API use, or representative end-to-end behavior when relevant.

The project does not require fuzzing or 100% code coverage. Coverage floors detect broad
regressions; they do not replace meaningful assertions. Networked provider runs and other
environment-dependent checks remain opt-in or scheduled unless they are the only practical way to
protect a supported contract. See the [quality plan](quality-plan.md) for the prioritized work.

## Test layers

### Syntax tests

Cover scalars, mappings, sequences, anchors, aliases, comments where supported, duplicate keys, malformed YAML, Unicode, line endings, spans, and error recovery.

### Typed-model tests

Cover every field in the documented [typed boundary](typed-model.md), all supported syntax variants,
unknown fields, extensions, image references, ports, volumes, host/container path separation,
environment values, environment-file short/long forms and options, service-label forms,
entrypoints, commands, service `attach` booleans/expressions and malformed recovery, raw
`cpu_count`/`cpu_percent`/`cpu_period`/`cpu_quota`/`cpu_rt_period` scalar categories and malformed recovery,
the related CPU/memory/namespace/OOM/scale and `volumes_from` service keys with interpolation,
merge provenance, boundaries, malformed recovery, local-reference validation, and retained
cross-field conflict diagnostics,
extra hosts, service hostnames, service annotations, service logging drivers and options, ordered service capability additions and drops, ordered mixed service devices, raw service DNS scalar/list forms, raw service security options, raw identities, service-level PID limits, service and Build shared-memory sizes, service memory limits, service-level temporary filesystems, service sysctls, restart and image pull policies, raw `pull_refresh_after` and `platform` strings, independent stop signals and raw
lifecycle grace periods, ulimits, health checks, dependency conditions, short/long build contexts,
non-empty Dockerfile, map/list build arguments, opaque target/network scalars, ordered raw platforms/tags, and
map/list-preserving build labels with retained conflict evidence and unmodeled build/deploy identities, networks,
profiles, configs, secrets, top-level resources, and discriminated unions.
Deploy rollback-config tests cover the authored six-member map, explicit empties, malformed sibling
recovery, duplicate and extension/unknown evidence, strict YAML-string boundaries, portability
diagnostics, recursive merge, reset/override provenance, sensitivity redaction, and separation from
update-config values.
Credential-spec tests cover mapping-only authored recovery, strict-string scalar boundaries,
spans, duplicates, extensions, unknown members, recursive merge, reset/override/null-reset
provenance, sensitivity redaction, and the public authored/effective API.
Extends tests cover schema-supported short strings and long mappings, strict-string `service`/`file`
members, explicit empty service values, missing-service diagnostics, spans, duplicates,
extensions, malformed/unknown recovery, generic recursive merge, reset/override/null-reset,
sensitivity redaction, and the public authored/effective API without service expansion or file
lookup. Separate project-processing coverage validates local long-form `service` edges without
`file` while retaining the no-expansion and no-I/O boundary.
Provider tests cover strict-string type recovery, empty types, ordered scalar/sequence option
categories, duplicate/empty/malformed evidence, spans, interpolation redaction, generic merge,
reset/override provenance, public APIs, and the no-execution/discovery boundary.

Long-volume option tests cover every currently typed option block, valid and invalid scalar
categories, list/map labels, duplicates, extensions, unknown nested fields, interpolation,
reset/override, and the public authored/effective API without generation or runtime access.

### Processing tests

Cover file ordering, merge rules, reset/override behavior, interpolation operators, `.env` handling, profile selection, include behavior, path origins, defaults, and reference resolution.

### Round-trip and property tests

Verify that parsing never panics, preservation edits retain unrelated syntax, canonical output is deterministic, and supported typed values survive parse-render-parse cycles.

The implemented canonical-rendering tier compares exact golden bytes, repeats rendering to prove
determinism, and verifies parse-merge-render stability. It also covers profile filtering, retained
tags, unresolved-alias recovery, and sensitive-output redaction. A regression fixture places an
empty environment value immediately before later service fields so parser recovery cannot silently
reparent ports, volumes, or extensions into the environment mapping.

Generated-rendering tests construct the runtime-migration subset through public Compose-owned
types, compare exact deterministic bytes, and inspect the parse-back native model. They protect
ordered duplicate-capable environment syntax, ordered environment-file short/long forms and
options, explicit `init`, `stdin_open`, `tty`, and `privileged` true/false choices and omission,
ordered unique service-label mappings, spec-shaped
ordered capability-add and capability-drop omission, explicit empty output, exact-case order,
duplicate and multiline rejection, independent coexistence, parse-back fidelity, and sensitivity
redaction,
long TCP/UDP ports, short SCTP ports,
ordinary mounts, deliberate short `SELinux` bind syntax, network aliases and raw per-attachment
IPv4/IPv6 address omission, ordering, duplicate-set rejection, sensitivity, and parse-back,
all service-level restart-policy forms and optional maximum retries,
all documented pull-policy forms, alias, integer `w`/`d`/`h`/`m`/`s` interval components,
schema-valid zero, and fractional or subsecond interval rejection,
named, numeric, and quoted-empty stop-signal spellings plus composite, zero, fractional, and
interpolation-shaped stop grace periods,
resolved uppercase and digit-leading RFC-1123 hostnames plus empty, deferred, non-ASCII, overlong,
and invalid-label rejection in generated output,
unlimited and arbitrary-precision positive PID limits plus rejection of zero, signs, fractions,
exponents, expressions, and arbitrary strings in generated output,
quoted positive canonical shared-memory sizes with every documented lowercase unit plus rejection
of zero, leading zeros, signs, fractions, exponents, uppercase/IEC units, whitespace, expressions,
and bare numbers,
quoted positive canonical memory limits with every documented lowercase unit plus rejection of
zero, leading zeros, signs, fractions, exponents, whitespace, and expressions,
service `tmpfs` omission/scalar/list/empty forms, colon-delimited documented and raw options,
ordering, exact duplicates, sensitivity, malformed-item rejection, and exact typed parse-back,
service `sysctls` omission/map/list/empty forms, ordered unique map names, quoted string values,
ordered exact-unique list strings, unsafe/deferred rejection, sensitivity, and typed parse-back,
service `ulimits` omission/empty/single/range forms, ordered unique lowercase names, quoted
non-negative decimal or `-1` values, missing-member and unsafe-value rejection, sensitivity, and
typed parse-back,
service `logging` omission/empty/configured/malformed authored forms, recursive option merge,
replacement/reset/override provenance, interpolation sensitivity, generated scalar kinds,
duplicate/unsafe rejection, deterministic bytes, and typed parse-back,
service `devices` omission/explicit-empty/mixed short/long forms, exact duplicates, order, CDI-like
and opaque raw strings, required long source, optional raw target/permissions, unsafe/deferred
rejection, sensitivity, deterministic bytes, and exact typed parse-back,
service DNS, DNS-option, and DNS-search authored/merged/generated forms, including empties,
duplicates, provenance, safety rejection, and typed parse-back,
service exposed-port scalar identity, merge behavior, validation, and typed parse-back,
raw security-option ordering, candidate/near-miss/conflict diagnostics, interpolation, and
duplicate-preserving generated parse-back,
service annotation syntax, keyed merge, ambiguity diagnostics, generation, and typed parse-back,
application/external resource lifecycle, duplicate
rejection, label duplicate rejection, empty and embedded-equals label values, ambiguous short-form
failures, and sensitive debug redaction.

Top-level volume-label regressions cover authored mapping/list fidelity, literal external-label
diagnostics including explicit empty collections, simultaneous external driver and label findings,
interpolation, generic map/list merge behavior, reset/override provenance, deterministic generated
maps, duplicate rejection, and parse-back sensitivity redaction.

Preservation-editing tests compare exact authored and expected files after changing typed scalar
spans. They prove that comments, whitespace, ordering, unknown fields, extensions, flow syntax, and
untouched quoting stay byte-identical. Failure tests cover foreign sources, key and non-scalar
targets, overlaps, block scalars, invalid numbers, atomic rollback, successful reparsing, and
sensitive replacement redaction.

Formatting tests prove that default options remain byte-identical canonical-v1, custom indentation
and line-ending output is exact, zero-width indentation is unrepresentable, document/final markers
are explicit, and customized output reparses and merges to the same canonical semantic bytes.

### Implementation conformance tests

Run selected fixtures through known Docker Compose and Podman Compose versions. Record:

- exact implementation and version
- command and arguments
- input files and environment map
- working directory assumptions
- stdout, stderr, exit status, and normalized result

Observed behavior becomes evidence, not an unquestionable specification. Conflicts between implementations are represented through compatibility profiles.

The first repository-side harness and exact provider-config matrix are implemented. The harness is
an ignored integration test: normal tests validate its complete matrix and authored fixture, but
external execution requires an absolute launcher, exact expected version, caller-verified artifact
URL and SHA-256 metadata, a full fixture revision, explicit platform and path inputs, and a new
result directory. It clears inherited environment variables and retains raw outputs for review.
See the [conformance guide](conformance.md) and
[ADR 0012](decisions/0012-repository-conformance-harness.md).

### Real-world fixtures

Use licensed projects and minimal reproductions of reported behavior. Every fixture needs provenance, redistribution permission, secret review, and a statement of what it protects.

The first real-world fixture is a generated PostgreSQL variant of
`Strukturpiloten/typo3-container`. It exercises five interacting services, typed Podman-specific
user namespace values, short-form SELinux mounts, dependencies, external networks, tag-plus-digest
images, caller-owned interpolation, sensitive-value redaction, reference validation, and stable
canonical rendering. An independent, byte-identical Docker Awesome Compose fixture adds build
definitions, health checks, dependency conditions, a top-level secret and grant, a named volume,
and a long read-only bind mount. The generation, licensing, sanitization, and update rules are
documented in the [real-world corpus guide](real-world-corpus.md).

## Test organization

Cargo-discovered integration tests live in [`../tests/`](../tests/README.md), with private helpers in `tests/support/`. Fixtures live in [`../fixtures/`](../fixtures/README.md) and are validated against the versioned [fixture manifest contract](fixture-format.md). Product suites are added only with implemented behavior and meaningful assertions.

The initial syntax corpus exercises comments, anchors, aliases, duplicate keys, extension fields, scalar spelling, interpolation-shaped text, tag-plus-digest image references, Unicode, CRLF input, malformed flow syntax, complete comma-containing block plain scalars, hyphenated anchor names and direct aliased block values, unquoted option-like sequence items, blank lines before indented mapping values, incomplete syntax-tree fail-safe detection, source spans, and exact parse/render/parse stability. The Phase 2 typed-model corpus covers its complete field boundary, capability-add and capability-drop omission/empty/duplicate/case/malformed recovery and coexistence, PID-limit, pull-policy, strict raw `pull_refresh_after` strings, service-tmpfs classification, service-sysctls form/scalar/duplicate behavior, service-ulimits name/range/value recovery, malformed recovery, deferred expressions, empty and null values, extensions and unknowns, partial invalid-input recovery, and stable source-spanned diagnostics. It also keeps short `:z`, short `:Z,ro`, and long `bind.selinux: Z` volume mounts distinct. The first processing corpus protects every interpolation operator, nested expressions, escaped dollars, missing-variable policies, required-value redaction, sensitivity propagation, and nesting recovery. It also covers ordered multi-file loading, explicit origin retention, first-file base-directory selection, duplicate source IDs, recoverable diagnostics, and one per-file interpolation overlay before merge. Field-aware merge fixtures cover mapping recursion, including nested soft/hard ulimit mappings, ordinary append including duplicate-preserving service `tmpfs` and `sysctls` lists, exact-key sysctls maps, independent exact-scalar capability-add and capability-drop uniqueness, command replacement, mixed environment and label forms, ordered environment files, unique ports, volumes, devices, configs, and secrets, YAML merge keys, unknown fields, reset/override tags, provenance, and sensitive-value debug redaction. Native project-view fixtures cover profile filtering, direct native images, commands, environments, environment-file short/long forms, capability-add and capability-drop field/item provenance and sensitivity, raw PID-limit, pull-policy, and `pull_refresh_after` replacement provenance, service-tmpfs and service-sysctls form/item provenance and sensitivity, ordered service-ulimits field/member provenance and sensitivity, sequence and mapping extra hosts, `host-gateway`, bracketed IPv6, ports, volumes, service config/secret grants, networks, top-level resources, unmodeled-field references, mismatched selections, recoverable invalid forms, sensitive-value redaction, unique-by-target nested-field retention, and field/item/collection provenance across two source files. Post-merge fixtures cover explicit and all-profile selection, profile reset behavior, inactive-service exclusion, relative and caller-supplied home path origins, named-resource references, inactive and missing service edges, documented defaults, no-default policy behavior, and rejection of selections from another project. Compatibility fixtures cover exact version parsing and ranges, selected-service feature discovery, Docker's documented `!override` boundary, distinct provider/runtime identities, conservative `podman-compose` unknowns, tolerant notes, source preservation, evidence scope, stable diagnostics, and sensitive-value redaction. Canonical-rendering fixtures cover exact presentation, multi-file output, retained Compose forms and tags, profile filtering, parse-render stability, recoverable aliases, trailing empty values, redaction, default formatting compatibility, and customized semantic stability. Preservation-edit fixtures cover typed exact-span changes, scalar-style retention and fallback, atomic failures, byte-identical unrelated syntax, reparsing, and redaction.

DNS regressions protect authored forms, merge/reset behavior, provenance, generation safety, and
typed parse-back.

Build regressions protect short/long context, non-empty Dockerfile, opaque target/network, raw ordered `cache_from`/`cache_to`/platforms/tags, boolean/string `no_cache`/`sbom`, boolean/expression `privileged`, raw-preserving `shm_size`, map/list args/labels, short/long secrets, and sensitive map/list `ssh`.
They cover empties, duplicates, per-file interpolation sensitivity, source/provenance, append/replacement/reset/override, partial malformed recovery, and `dockerfile`/`dockerfile_inline` conflict evidence.
All remaining build siblings stay source-addressable unmodeled evidence.

Lifecycle-hook regressions protect post-start, pre-stop, and pre-start ordered hook recovery, explicit null commands, local environment
forms, extension/duplicate evidence, interpolation redaction, and generic append/reset/override provenance.

Runtime, pull-refresh, and platform regressions protect strict YAML-string recovery, empty/deferred
spelling, scalar replacement/reset/override provenance, sensitivity redaction, and no runtime,
refresh, or OCI interpretation.

Attach regressions protect literal/deferred parsing, duplicate and malformed recovery, source and
merge provenance, reset/override, sensitivity redaction, and the absence of default, generated,
logging, runtime, provider, CLI, compatibility, or cross-format behavior.

Blkio regressions protect all six schema members, integer/string spelling, ordered duplicate items,
partial recovery, and generic mapping/sequence merge provenance without `extends` inheritance claims.

Cgroup regressions protect strict-string classification, invalid retained spelling, duplicate and
malformed recovery, scalar replacement/reset/override provenance, and sensitive interpolation redaction.

Cgroup-parent regressions protect strict-string preservation for empty, whitespace, deferred, and
arbitrary spellings; duplicate/malformed recovery; scalar replacement/reset/override provenance;
and sensitive interpolation redaction without path or runtime interpretation.

CPU-count regressions protect exact zero, unbounded, base, separator, negative-zero, negative,
quoted, empty, and deferred spelling; malformed/duplicate recovery; scalar merge provenance;
sensitivity redaction; and canonical rendering without quota or runtime interpretation.

Deploy endpoint-mode regressions protect authored documented/provider-specific/interpolated values,
duplicates, malformed recovery, scalar merge/reset/override provenance, sensitivity, and nested
resource coverage plus malformed, extension, and future-unknown deploy evidence.
Deploy-mode regressions additionally protect global/replicated/raw/empty/deferred values, omission,
and global coexistence with replicas and service scale without scheduling diagnostics. Deploy-replica
regressions protect raw YAML number spelling, distinct string/empty/deferred values, malformed
recovery, duplicate retention, merge/reset/override provenance, sensitivity, and the absence of
integer, default, mode-coupling, allocation, scheduling, runtime, or conversion inference.
Deploy-label regressions protect mapping/list forms, scalar/null and bare/`KEY=VALUE` evidence,
keyed merge/list append, duplicate retention, reset/override provenance, sensitivity, and malformed
recovery without container, service, runtime, platform, deployment, or conversion inference.
Deploy restart-policy regressions protect all members, condition spelling, raw duration and
attempt scalar categories, partial nested merge and replacement provenance, reset/malformed nested
evidence, and separation from service `restart` without defaults, precedence, simulation, runtime,
or conversion inference.
Update-config regressions protect map recovery, scalar boundaries, retained extensions and unknowns,
provider-specific order diagnostics, generic merge/reset/override provenance, and redaction.
Deploy placement regressions protect YAML-string constraints and optional preference spreads,
empty and duplicate items, YAML integer/string maximum categories, nested extensions/unknowns,
partial malformed recovery, append/replacement/reset/override provenance, and sensitive
interpolation redaction without constraint grammar, node-selection, default, scheduling, runtime,
or conversion inference.
Deploy resource-limit PID regressions protect integer/string scalar categories, nested recursive
merge, leaf/mapping reset and override provenance, extensions/unknowns, malformed recovery,
sensitivity redaction, and independence from service `pids_limit` without unlimited, range/default,
host, cgroup, runtime, or conversion inference.
Deploy resource-limit CPU regressions protect number/string scalar categories, exact numeric
spelling, nested recursive merge, leaf/mapping reset and override provenance, extensions/unknowns,
malformed recovery, sensitivity redaction, and independence from service CPU and `mem_limit`
without range/default, host, cgroup, runtime, or conversion inference.
Deploy resource-limit memory regressions protect YAML-string-only acceptance; documented lowercase
units; lexical zero, deferred, and provider-dependent states; nested recursive merge; leaf/mapping
reset and override provenance; malformed recovery; sensitivity redaction; and independence from
service `mem_limit` and reservations without amount parsing, range/default, host, cgroup, runtime,
or conversion inference.
Deploy resource-reservation CPU and memory regressions protect numeric/string and string-only raw
categories respectively, exact spelling, recursive merge, leaf/mapping/resource reset and override
provenance, extensions/unknowns, malformed recovery, sensitivity redaction, and independence from
limits and service CPU/`mem_limit` without validation, rounding, defaults, scheduling, provider,
runtime, target, or conversion inference.
Generic-resource regressions protect schema-only list evidence: ordinary append and
reset/override provenance, item order/forms, optional raw discrete `kind` and number-or-string
`value`, malformed recovery, nested evidence, spans, and sensitivity. They make no prose,
version, provider, matching, scheduling, device, runtime, or conversion claim; reservation
device semantics remain a separate boundary.
Reservation-device regressions protect required ordered raw capability strings, optional strict
YAML-string drivers, raw integer-or-string counts, ordered strict-string device IDs, exact duplicate
capability diagnostics without collapsing items, allocation-selector conflicts, mapping/unmodeled
recovery, nested unknown evidence, spans, sensitivity, and append/reset/override provenance. They
reject timestamp/regex styles, other scalar kinds, and collections for strict strings; infer no
device selection/loading, capability/driver grammar, count range/sign/default/`all`, allocation or
GPU/device matching, CDI, host, scheduling, provider/version, runtime, cgroup, or conversion
behavior. Options regressions cover map/list forms, scalar kinds, exact duplicate list strings,
malformed recovery, sensitivity, and generic merge provenance.

Logging regressions protect empty mappings, scalar-kind fidelity, malformed sibling recovery,
recursive merge provenance, value-only interpolation, generation safety, and typed parse-back.

The built-in compatibility rules are unit/integration evidence, not a substitute for the runtime
conformance tier. Phase 5 expands the exact Docker Compose, `podman-compose`, Docker Engine, and
Podman matrices. Planned matrix entries make no support claim. A runtime observation may be
promoted into a built-in rule only with an exact command, provider version, runtime version,
platform assumptions, and reviewed retained result.

The issue-derived regression tier adds authored fixtures for both valid and malformed
`extra_hosts`, user/group interpolation, unlimited ulimits, service-level restart policies and
maximum retries, health checks, dependency conditions,
anonymous Linux container paths, and independently identified build/deploy fields. The licensed
TYPO3 fixture demonstrates the Podman `keep-id` consumer. Compatibility tests detect
`host-gateway` and Podman user-namespace modes and require official Podman 5.4 evidence while
keeping unobserved provider pass-through classified as unknown.

## Regression rule

Every bug fix adds the smallest fixture that failed before the fix. When an external implementation changes, preserve old-version expectations if ComposeLens still claims compatibility with that version.

## Canonical commands

The crate uses Rust 2024 with an MSRV of 1.85.0. `rust-toolchain.toml` pins the normal development toolchain; the explicit MSRV command prevents that pin from hiding accidental use of newer language or library features.

```shell
./scripts/check-all.sh
./scripts/check-files.sh --check
cargo fmt --all -- --check
cargo ci-check
cargo ci-policy
cargo ci-clippy
cargo ci-test
cargo ci-doctest
RUSTDOCFLAGS="-D warnings" cargo ci-doc
cargo llvm-cov --locked --workspace --all-features --all-targets --summary-only \
  --fail-under-regions 88 --fail-under-functions 87 --fail-under-lines 89
cargo +1.85.0 ci-check
cargo +1.85.0 ci-policy
cargo deny check
cargo test --locked --test conformance
cargo test --locked --test runtime_conformance
cargo test --locked --test real_world
cargo test --locked --test public_api
cargo test --locked --test generated_rendering
cargo package --locked
```

`scripts/check-all.sh` is the one-command local gate. It formats owned files before checking the
same deterministic Rust, coverage, MSRV, dependency, offline-link, and patch-SemVer boundaries
used for release preparation. The ignored provider capture remains opt-in.

The `ci-*` aliases use `--locked`, all workspace features, and all targets where the Cargo command
supports them. CI also runs markdownlint and lychee over the documentation. The ordinary
conformance command validates matrix policy and leaves its external runner ignored. The explicit
collection command is documented in [`../conformance/README.md`](../conformance/README.md). Add
deterministic property-style commands here before any additional harness becomes a required check.

The pinned `cargo-llvm-cov` 0.8.7 gate runs the locked workspace with all features and targets.
Its coarse integer floors—88% regions, 87% functions, and 89% lines—are regression guards, not a
claim that line execution proves behavior. Positive and negative assertions remain required at
each supported boundary.
