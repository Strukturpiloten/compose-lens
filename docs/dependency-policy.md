# Dependency and license policy

Dependencies are design decisions. Prefer the standard library and focused, maintained crates whose
APIs preserve ComposeLens's source fidelity and explicit processing boundaries.

## Cargo rules

- Use compatible crates.io requirements by default; wildcard requirements are denied.
- Use an exact pin only for a documented representation or compatibility reason.
- Keep default features only when reviewed and useful.
- Avoid overlapping crates without a clear responsibility difference.
- Commit `Cargo.lock` and use locked resolution in CI and releases.
- Record a dependency that constrains YAML representation, round trips, source locations, or public
  APIs in an ADR.

Unapproved registries and Git dependencies are denied. An exception must be narrowly versioned,
explained in `deny.toml`, and justified in the introducing change; lasting architectural or
distribution exceptions require an ADR.

## Licenses and advisories

`deny.toml` is the machine-readable source of truth for accepted licenses, advisories, bans,
duplicates, and sources. An allowlisted license records project policy even when no current dependency
uses it. Adding a license requires review of its distribution obligations; this policy is not legal
advice.

Do not silence an advisory, source, duplicate, or license finding merely to make CI pass. Run:

```console
cargo deny check
```

## Repository tooling

GitHub Actions, release preparation, the Rust toolchain, Dev Container assets, Node tools, and native
file tools are supply-chain dependencies even though they are not shipped in the Rust graph. Their
exact versions, action SHAs, lockfiles, and checksums live in executable configuration rather than
this guide.

Renovate proposes supported updates. Repository policy verifies immutable action pins, integrity
metadata, single version sources, and locked tooling. Every update still needs the same review and
complete gate as a hand-authored dependency change.

## YAML representation

`yaml-edit` 0.2.3 is pinned exactly with default features disabled. It remains private and no
`yaml-edit` type may appear in the public API. The decision and alternatives are recorded in
[ADR 0002](decisions/0002-loss-aware-yaml-syntax.md) and the
[YAML representation evaluation](research/yaml-representation.md).
