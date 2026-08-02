# API stability policy

ComposeLens is pre-1.0, but its API is not an unbounded experiment. Version 0.1 establishes one
documented public release line for early BoxFerry integration and independent consumers. This
policy is recorded by [ADR 0013](decisions/0013-versioned-public-api-and-release-contract.md).

## The 0.1.x contract

Within the 0.1.x line:

- patch releases preserve source compatibility for supported public entry points;
- public APIs use ComposeLens-owned types, while `yaml-edit` remains private;
- the module paths used by `tests/public_api.rs` remain available;
- diagnostic code strings remain machine-readable contracts;
- canonical-v1 default rendering remains deterministic for the same semantic input;
- parsing, processing, validation, and rendering keep their documented side-effect boundaries; and
- all supported public APIs compile on Rust 1.85.0 or newer.

Bug fixes may change a result that contradicted a documented contract or retained conformance
evidence. Such a change needs a regression test and changelog entry. A patch release must not
silently normalize a preserved syntax form, expose a parser-dependency type, perform new I/O, or
rename a diagnostic code.

## Supported entry points

The 0.1 consumer contract covers these explicit stages:

| Stage | Public modules |
| --- | --- |
| Source and diagnostics | `source`, `diagnostic`, `syntax` |
| Native Compose types | `model` |
| Caller-owned environment and project inputs | `interpolation`, `loader` |
| Merge and post-merge views | `merge`, `profiles`, `resolution` |
| Versioned compatibility reports | `validation` |
| Canonical output and scalar preservation edits | `render` |

The compile-and-behavior contract in `tests/public_api.rs` exercises this path as an external crate
consumer would. The modules remain separate deliberately; 0.1 does not add a convenience function
that hides file access, interpolation, merging, profile selection, validation, or rendering.

## Changes before 1.0

Rust's semantic-versioning convention permits breaking changes in the next pre-1.0 minor release.
ComposeLens still requires an ADR when the processing architecture changes, release notes with a
migration section, and a new 0.x minor version for an intentional public break. Consumers that
cannot absorb that cadence should use an exact dependency requirement or commit their lockfile.

Adding a variant to one of the public compatibility-context enums marked `#[non_exhaustive]` is not
a breaking change. Other public enums may become non-exhaustive only in a breaking release because
adding that attribute itself affects downstream exhaustive matches.

## Not promised by 0.1

The 0.1 contract does not claim:

- complete coverage of every Compose field;
- structural source editing beyond the documented scalar boundary;
- behavior parity among the Compose Specification, Docker Compose, and `podman-compose`;
- runtime effects from provider-only `config` observations; or
- long-term 1.x compatibility.

Before 1.0, the project will define supported release lifetimes, deprecation periods, and the
1.x diagnostic-code policy through a superseding ADR.
