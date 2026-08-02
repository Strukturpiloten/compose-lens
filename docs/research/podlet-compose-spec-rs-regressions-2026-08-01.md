# Podlet and `compose_spec_rs` Compose regression review

- Reviewed: 2026-08-01
- Upstream trackers: [`containers/podlet`](https://github.com/containers/podlet/issues) and
  [`k9withabone/compose_spec_rs`](https://github.com/k9withabone/compose_spec_rs/issues)
- ComposeLens scope: syntax, native Compose types, processing, compatibility, and rendering

## Purpose

The full upstream review covered 139 Podlet issues and 20 `compose_spec_rs` issues, including open
and closed reports. Pull requests were excluded from the counts. This document retains the subset
that should influence ComposeLens. Cross-format mapping, pod grouping, Quadlet generation, and
runtime inspection belong to BoxFerry or QuadletLens.

An upstream issue is a regression lead, not normative evidence. ComposeLens still requires an
authoritative source or exact provider observation plus an independently authored fixture before
changing compatibility behavior.

## Existing coverage confirmed by the issue corpus

### Tolerant image references

[`compose_spec_rs` #22](https://github.com/k9withabone/compose_spec_rs/issues/22) and
[Podlet #91](https://github.com/containers/podlet/issues/91) show why the final colon in an image
string cannot automatically begin a tag: registries may include ports. [Podlet #101](https://github.com/containers/podlet/issues/101)
documents real projects that combine a human-readable tag and a digest.

ComposeLens already protects both forms. The raw value remains authoritative, and provider support
for tag plus digest is versioned independently from parsing.

### Field-specific short and long forms

[Podlet #26](https://github.com/containers/podlet/issues/26) reports a parser that could not mix
short and long mounts. [Podlet #207](https://github.com/containers/podlet/issues/207) shows that a
valid long port may carry `mode`, `app_protocol`, and `name`, even when a target cannot represent
them. [`compose_spec_rs` #24](https://github.com/k9withabone/compose_spec_rs/issues/24) adds bracketed
IPv6 in short ports.

ComposeLens already uses per-entry short/long variants, types all current long-port fields, retains
unknown metadata, keeps raw short strings, and tests bracketed IPv6. A downstream converter must
classify unrepresentable long-form metadata instead of asking ComposeLens to normalize it away.

### Explicit processing instead of parse-time side effects

- Anchors and fragments: [`compose_spec_rs` #2](https://github.com/k9withabone/compose_spec_rs/issues/2),
  [Podlet #58](https://github.com/containers/podlet/issues/58), and
  [#154](https://github.com/containers/podlet/issues/154).
- Interpolation: [`compose_spec_rs` #3](https://github.com/k9withabone/compose_spec_rs/issues/3) and
  [Podlet #81](https://github.com/containers/podlet/issues/81).
- Multi-file merge: [`compose_spec_rs` #4](https://github.com/k9withabone/compose_spec_rs/issues/4)
  and [Podlet #59](https://github.com/containers/podlet/issues/59).

These reports reinforce the existing separation between loss-aware parsing, caller-owned
interpolation, ordered loading, and provenance-preserving merge. They also show why using
`docker compose config` or `podman compose config` as mandatory preprocessing is insufficient: it
can interpolate values and canonicalize source forms before ComposeLens sees them.

### Unknown fields and recoverable processing

[Podlet #189](https://github.com/containers/podlet/issues/189),
[#143](https://github.com/containers/podlet/issues/143), and
[#206](https://github.com/containers/podlet/issues/206) show the cost of rejecting an entire project
because one field is unknown or unsupported by the eventual target.

ComposeLens already preserves unknown and `x-` fields with spans and returns partial typed data
with diagnostics. Whether target output may be produced is a BoxFerry policy decision; the parser
must never silently drop the original field.

### References and scalar kinds

[`compose_spec_rs` #18](https://github.com/k9withabone/compose_spec_rs/issues/18) motivates explicit
network, volume, config, and secret reference validation. [Podlet #62](https://github.com/containers/podlet/issues/62)
and [#191](https://github.com/containers/podlet/issues/191) demonstrate non-string and empty label
values. [`compose_spec_rs` #29](https://github.com/k9withabone/compose_spec_rs/issues/29) covers
service-network `driver_opts`.

All three behaviors are inside the current typed or post-merge boundary.

## Post-0.1 typed-model expansion — implemented

These items were implemented without reopening or normalizing the completed first BoxFerry
boundary. [ADR 0014](../decisions/0014-issue-derived-native-model-expansion.md) records the public
representation and validation decisions.

### `extra_hosts`

[`compose_spec_rs` #51](https://github.com/k9withabone/compose_spec_rs/issues/51) records short `=`
and `:` delimiters, bracketed and unbracketed IPv6, long mapping syntax, and the
implementation-specific `host-gateway` token also requested in
[Podlet #155](https://github.com/containers/podlet/issues/155).

Required approach: preserve the authored variant and raw address, type standard forms, and classify
special tokens by exact Compose provider and backend runtime evidence.

### User and group values

[`compose_spec_rs` #23](https://github.com/k9withabone/compose_spec_rs/issues/23) and
[#41](https://github.com/k9withabone/compose_spec_rs/issues/41) cover `UID:GID`. Their discussion
also notes that the Compose text is less precise than Docker/Podman runtime syntax and that
providers accept broad strings.

Required approach: never force an identifier into an unsigned integer. Retain raw spelling and
represent independently optional user/group IDs or names only when the decomposition is
unambiguous.

### Limits, dependencies, and health checks

- Unlimited `ulimits` using `-1`: [`compose_spec_rs` #31](https://github.com/k9withabone/compose_spec_rs/issues/31)
  and [Podlet #117](https://github.com/containers/podlet/issues/117).
- Dependency targets and `service_healthy`: [`compose_spec_rs` #48](https://github.com/k9withabone/compose_spec_rs/issues/48),
  [Podlet #145](https://github.com/containers/podlet/issues/145), and
  [#164](https://github.com/containers/podlet/issues/164).
- Health command representation: [Podlet #160](https://github.com/containers/podlet/issues/160).

ComposeLens now types single and soft/hard limits, the `-1` unlimited sentinel, health-check scalar
and command-list forms, durations, retry counts, dependency conditions, `restart`, and `required`.
Document and post-merge validation distinguish missing services, optional dependencies, explicitly
disabled health checks, and health metadata that may exist only in an image.

### Platform-sensitive anonymous volumes

[`compose_spec_rs` #38](https://github.com/k9withabone/compose_spec_rs/issues/38) and
[Podlet #99](https://github.com/containers/podlet/issues/99) report a Windows-host parser rejecting
`/project/node_modules`, an anonymous Linux container volume target, because the host's path API did
not consider it absolute.

Required approach: Compose container paths use the target container-platform grammar. Host path
resolution uses explicit source platform and project origin. Neither should depend implicitly on
the machine running ComposeLens.

### Implementation additions and larger sections

- mount `chown`: [`compose_spec_rs` #47](https://github.com/k9withabone/compose_spec_rs/issues/47)
  and [Podlet #157](https://github.com/containers/podlet/issues/157);
- restart `max-retries`: [`compose_spec_rs` #49](https://github.com/k9withabone/compose_spec_rs/issues/49)
  and [Podlet #185](https://github.com/containers/podlet/issues/185);
- CDI devices: [Podlet #107](https://github.com/containers/podlet/issues/107);
- `userns_mode`: [Podlet #31](https://github.com/containers/podlet/issues/31);
- build entitlements and image/build combinations:
  [`compose_spec_rs` #15](https://github.com/k9withabone/compose_spec_rs/issues/15),
  [Podlet #126](https://github.com/containers/podlet/issues/126), and
  [#173](https://github.com/containers/podlet/issues/173); and
- `deploy`: [Podlet #215](https://github.com/containers/podlet/issues/215).

`userns_mode` is now typed because the licensed Strukturpiloten TYPO3 fixture demonstrates its
consumer, and Podman 5.4 documentation supplies runtime evidence. `host-gateway` receives the same
evidence-aware treatment. Build and deploy top-level subfields now have independent typed
identities and source references, so a converter can evaluate them separately without pretending
that every nested value has one support outcome.

Mount `chown`, restart `max-retries`, and CDI devices remain loss-aware but untyped until a concrete
consumer and compatibility evidence justify their native representation.

## Fixture coverage and remaining candidates

The authored `post-01-issue-backlog` and `post-01-invalid` fixtures now cover:

- an `extra_hosts` matrix with both delimiters, IPv6 forms, and `host-gateway`;
- numeric, named, and interpolated user/group values without splitting default-operator colons;
- `-1` in short and long `ulimits`;
- valid and invalid `service_healthy` dependency graphs;
- anonymous Linux container-volume targets classified without host path APIs;
- scalar and command-list health checks, including invalid modes and durations; and
- field-level build and deploy sections.

Additional ambiguous identity strings, commands containing more shell-sensitive text, and
Dependency-Track, Coop Cloud, Frigate, Invidious, Angular, and Immich
remain useful corpus candidates only after immutable revision, license, secret, and minimality
review.

No upstream Compose file should be copied directly from an issue without the normal corpus
admission process.

## Low-priority or non-product findings

[`compose_spec_rs` #50](https://github.com/k9withabone/compose_spec_rs/issues/50) requests JSON Schema
generation for language bindings. This may become useful after independent consumers stabilize the
typed model, but a schema cannot express all source-spelling, provenance, and recoverable-invalid
contracts and is not an early priority.

Repository moves, packaging, release cadence, CI, and license-file issues were reviewed but do not
change ComposeLens parsing or processing behavior.
