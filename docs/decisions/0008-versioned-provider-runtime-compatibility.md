# ADR 0008: versioned provider/runtime compatibility profiles

- Status: accepted
- Date: 2026-07-31

## Context

Compose syntax acceptance does not prove that a construct works on a selected implementation and
runtime. A parser can accept a field that an older provider did not understand, a provider can
translate equivalent-looking short and long forms differently, and a backend runtime can impose
additional platform constraints.

“Podman Compose” is also ambiguous. Podman documents `podman compose` as a thin wrapper around an
external provider such as Docker Compose or `containers/podman-compose`. A Podman version alone
does not identify the parser, merge implementation, interpolation rules, or supported commands.

Compatibility knowledge grows incrementally. Treating absent evidence as support would recreate
the strict-specification problem ComposeLens is intended to avoid. Treating every unknown as a
parse error would make tolerant migration analysis impossible.

## Decision

1. `CompatibilityProfile` records a `ComposeProvider` and an optional `ContainerRuntime` as
   separate values. Docker Compose and `containers/podman-compose` require exact released versions.
   Docker Engine and Podman runtime contexts also use exact versions when supplied.
2. `podman compose` is not a provider enum variant because it delegates to another executable. The
   caller must report the provider that actually ran.
3. `ImplementationVersion` is a comparable three-component numeric version. ComposeLens does not
   infer “current” or silently discard pre-release/build text. Evidence uses inclusive
   `VersionRange` minimum and maximum bounds.
4. Compatibility features are detected non-destructively from a merged, optionally profile-selected
   view. Findings retain feature identity, semantic path, source span, and sensitivity, not the raw
   feature value.
5. Rules classify features as supported, extension, implementation-specific, deprecated,
   unsupported, or unknown. Unsupported produces an error; implementation-specific, deprecated,
   and ordinary unknown results produce warnings. Tolerant unknowns produce notes but remain
   unknown.
6. Every rule may cite specification text, official documentation, a public versioned issue
   reproduction, or a ComposeLens-controlled runtime conformance result. Evidence scopes provider
   and runtime versions independently.
7. The initial feature boundary is combined image tags/digests, short and long bind `SELinux`
   relabeling, `!reset`, `!override`, and `x-` extensions. New features extend the catalog without
   changing syntax parsing.
8. The known long-bind `SELinux` failure is classified as unsupported only for the exact reported
   Docker Compose 2.40.3/Podman 5.6.2 pair. Uncovered pairs remain unknown.
9. Compatibility validation never invokes providers or runtimes. Scheduled conformance adapters
   supply future evidence through a separate side-effect boundary.

## Consequences

- BoxFerry can distinguish a syntax error from a target-version limitation or an evidence gap.
- Reports are useful even when every finding is supported because they expose the decisions and
  evidence used.
- Callers must discover and supply actual provider/runtime versions; this explicit work prevents
  misleading “works with Podman” claims.
- Conservative unknown classifications will initially be common for `podman-compose` until the
  runtime matrix exists.
- Numeric release candidates are not representable by `ImplementationVersion`; a future version
  type extension needs an ADR if pre-release conformance becomes a requirement.

## Alternatives considered

### One Podman Compose version

Rejected because Podman's command delegates parsing and Compose behavior to an external provider.

### Assume the latest installed implementation

Rejected because library results would change over time and across machines without an explicit
input change.

### Boolean supported/unsupported flags

Rejected because extensions, deprecations, implementation-specific behavior, and missing evidence
have different meanings and remediation.

### Put compatibility rules in the typed parser

Rejected because native syntax and model preservation must remain independent from a chosen target
version and backend runtime.
