# ADR 0019: Consolidated 0.2 public API

- Status: accepted
- Date: 2026-08-14
- Supersedes: [ADR 0013](0013-versioned-public-api-and-release-contract.md)

## Context

The 0.1.x line accumulated additive entry points while ComposeLens was establishing its complete
source-aware model. Resource `external` values consequently exposed a modern boolean getter plus a
second complete-syntax getter. That split served patch compatibility, not the clearest long-term
API. ComposeLens is still pre-1.0 and can remove that cost explicitly.

## Decision

ComposeLens publishes a supported 0.2.x API line.

- `NetworkDefinition`, `VolumeDefinition`, `ConfigDefinition`, and `SecretDefinition` expose one
  `external()` getter returning `ResourceExternal`.
- `ResourceExternal::Boolean` represents current boolean or deferred-expression syntax.
- `ResourceExternal::NameMapping` retains deprecated Compose `external: { name: ... }` input as an
  `ExternalNameMapping`; parsing old input is format compatibility, not a library API alias.
- `external_syntax()`, `LegacyExternalName`, and compatibility-only names are removed rather than
  deprecated.
- Patch releases inside 0.2.x preserve the documented public paths. A later intentional public
  break requires another 0.x minor release and migration guidance.
- CI, release, and local SemVer checks derive the release type from Cargo package versions instead
  of forcing patch semantics.
- The existing side-effect, diagnostic, Rust 1.85, package, and auditable-evidence contracts remain.

## Consequences

Callers have one complete source-aware resource API and must migrate once from the 0.1.x getters.
The 0.2.0 version communicates that break. Deprecated Compose documents remain inspectable without
normalization or information loss.

## Alternatives considered

### Preserve both getters indefinitely

Rejected because the second getter exists only for 0.1.x source compatibility and makes the normal
API incomplete.

### Stop parsing deprecated Compose input

Rejected because real Compose documents still contain it; removing input support would reduce
format fidelity without simplifying the public model meaningfully.
