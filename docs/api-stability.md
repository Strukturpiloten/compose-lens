# API stability policy

ComposeLens is pre-1.0, but its public API is not an unbounded experiment. Version 0.3.x is the
current supported release line for BoxFerry and independent Rust consumers.

## Patch-release contract

Within 0.3.x:

- supported module paths exercised by `tests/public_api.rs` remain source-compatible;
- public interfaces use ComposeLens-owned types and keep the YAML backend private;
- diagnostic code strings remain stable automation keys;
- canonical-v2 output remains deterministic for the same semantic input;
- parsing, processing, validation, and rendering keep their documented side-effect boundaries; and
- every supported API compiles on Rust 1.85.0 or newer.

A bug fix may change behavior that contradicts a documented contract or reviewed conformance
evidence. Such a change needs a regression test and a concise changelog entry. A patch release must
not silently normalize preserved syntax, expose a parser dependency, add implicit I/O, or rename a
diagnostic code.

## Supported surface

| Concern                                       | Public modules                               |
| --------------------------------------------- | -------------------------------------------- |
| Sources, diagnostics, and loss-aware YAML     | `source`, `diagnostic`, `syntax`             |
| Native Compose documents                      | `model`                                      |
| Caller-owned inputs and interpolation         | `loader`, `interpolation`                    |
| Merge and processed project views             | `merge`, `profiles`, `project`, `resolution` |
| Versioned compatibility findings              | `validation`                                 |
| Canonical, generated, and preservation output | `render`                                     |

The compile-and-behavior contract lives in `tests/public_api.rs`. Generated Rustdoc is the source of
truth for individual public types and methods; this policy deliberately does not copy a symbol or
field inventory.

Additive APIs may expand the supported surface in a patch release when they preserve existing
behavior. Existing exhaustive enums can become non-exhaustive only in a breaking release. Enums
already declared `#[non_exhaustive]` may add variants without a minor-version break.

## Processing and I/O

Module separation is part of the contract. ComposeLens does not add a convenience function that
silently discovers files, reads the process environment, interpolates, merges, selects profiles,
resolves paths, validates a provider, and renders in one call.

Caller-owned traits remain the authorization boundary for include content, environment values, path
policy, and defaults. Results preserve source evidence and redact sensitive `Debug` output.

## Before 1.0

Intentional source breaks require a new 0.x minor version, a release-worthy breaking change, an ADR
when architecture changes, and migration guidance in release notes. Input compatibility and Rust API
compatibility are separate: ComposeLens may continue parsing a deprecated Compose spelling without
keeping a redundant Rust accessor from an older release line.

The current contract does not promise complete provider/runtime semantics for every Compose value,
parity among Compose implementations, structural editing beyond the documented scalar boundary, or
long-term 1.x compatibility. A future 1.0 decision will define support lifetimes, deprecation periods,
and diagnostic-code policy.

[ADR 0019](decisions/0019-consolidated-0.2-public-api.md) records the pre-1.0 API consolidation, and
[ADR 0024](decisions/0024-safe-minimal-yaml-presentation.md) records the canonical-v2 byte contract.
