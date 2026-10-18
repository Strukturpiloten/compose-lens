# ADR 0029: Align minimum and normal Rust toolchains

- Status: accepted
- Date: 2026-10-18
- Amends: [ADR 0019](0019-consolidated-0.2-public-api.md), minimum Rust version only

## Context

ComposeLens's normal toolchain is already pinned to Rust 1.98.1 while its manifest, Clippy
configuration, and supported API policy still promise Rust 1.85.0. The Rust repositories in the
BoxFerry workspace are aligning their minimum and normal compiler versions.

## Decision

Require Rust 1.98.1 for the current ComposeLens package and align `Cargo.toml`, `clippy.toml`,
`rust-toolchain.toml`, active documentation, and command examples. This explicitly replaces the
earlier minimum-version promise retained by ADR 0019 and the API stability policy. Other public
API and behavioral contracts remain in force.

CI and the complete local gate continue to read the minimum version from Cargo metadata. Release
validation continues to reuse complete CI. Keep the MSRV jobs even though both compiler versions
currently match. This decision neither bumps the package version nor changes dependency update
automation or authorizes future automatic MSRV increases.

Historical changelog entries, ADR text, and dated parser evaluation results retain their original
versions. Current parser-update validation follows the manifest's minimum version.

## Consequences

Consumers using an older compiler must upgrade before adopting this change. Repository policy
checks enforce compiler, Clippy, and supported-documentation agreement, while existing local,
CI, and release checks consume the new manifest version without additional pins.

## Alternatives considered

Retaining Rust 1.85.0 would preserve the former compiler promise but would keep the minimum and
normal toolchain policies divergent. Removing MSRV validation would conceal future drift instead
of preserving the explicit compiler contract.
