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

Provider-conformance artifact URLs, checksums, and observed-record versions are immutable evidence:
they are not routine Renovate updates. Renovate surfaces the matrix-declared Python runtime and
bootstrap pins for manual provenance and checksum review with automerge disabled. The reusable runner
selects that runtime and rejects a podman-compose row unless `python3` reports the exact declared
version. It preserves the immutable wheel URL basename and installs the verified local wheel through
a hash-locked PEP 508 file reference; changing that bootstrap contract requires its offline install
and wrong-hash regression to change together. A moving provider-discovery candidate belongs in a new
reviewed matrix target, never a rewrite of a retained boundary observation.

Renovate's three-day minimum release age governs direct dependency updates. Renovate cannot prove
the age of versions selected while regenerating a lock file, so lock-file maintenance has a
rule-local zero-day Renovate override and may auto-merge only when the shared, fail-closed
lockfile-release-age guard proves every newly introduced registry release is at least 72 hours old
and the required aggregate PR gate succeeds.

Fixed GitHub-hosted `ubuntu-*`, `macos-*`, and `windows-*` runner labels have one
`github-runners` regex owner across every workflow. Runner upgrades stay grouped with automerge
disabled because their preinstalled tools and operating-system changes affect release evidence.

Every immutable revision has one Renovate owner. Dev Container and checksum-pinned tool updates
remain manual.

## YAML representation

`yaml-edit` is pinned exactly in `Cargo.toml` with default features disabled. It remains private and
no `yaml-edit` type may appear in the public API. Renovate owns the reviewed version and lockfile
update, while the compatibility contract remains version-independent. The decision and alternatives are recorded in
[ADR 0002](decisions/0002-loss-aware-yaml-syntax.md) and the
[YAML representation evaluation](research/yaml-representation.md).
