# Repository guidance for coding agents

This file applies to the entire ComposeLens repository.

## Read before changing the repository

Always read:

1. `README.md`
2. `docs/architecture.md`
3. `docs/decisions/README.md`

Then read only the material relevant to the change:

| Work                                                | Read                                                             |
| --------------------------------------------------- | ---------------------------------------------------------------- |
| Loading, interpolation, merge, profiles, resolution | `docs/processing-model.md` and linked ADRs                       |
| Canonical output, generated documents, source edits | `docs/rendering.md` and linked ADRs                              |
| Native coverage or public API                       | `docs/coverage.md`, `docs/api-stability.md`                      |
| Tests, fixtures, provider evidence                  | `docs/testing.md`, `fixtures/README.md`, `conformance/README.md` |
| Dependencies or releases                            | `docs/dependency-policy.md`, `docs/releasing.md`                 |
| Development environment                             | `docs/development-environment.md`                                |

Read an accepted ADR when a change touches its decision. Architectural changes require an ADR or an
explicit amendment or superseding decision in the same change. Do not reread every historical ADR
for an unrelated documentation or maintenance edit.

## Scope

ComposeLens owns native Compose syntax, models, project loading, merging, profile selection,
interpolation, validation profiles, rendering, source locations, and diagnostics. It does not own
cross-format conversion, runtime inspection, Quadlet, Kubernetes deployment policy, or BoxFerry's
neutral application model. ComposeLens must not depend on BoxFerry.

## Origin policy

ComposeLens is implemented from scratch. Do not copy or mechanically translate source from
`compose_spec_rs`, Docker Compose, Podman Compose, Podlet, or another parser. Specifications,
public documentation, and versioned observable behavior may inform an independent implementation.

Differential evidence must record the implementation, version, command, inputs, environment, and
expected result. Third-party parsing dependencies require deliberate review and an ADR when they
constrain round-trip behavior or the public model.

## Non-negotiable behavior

- Parsing never reads process environment variables unless a caller explicitly supplies a provider.
- Unknown fields and `x-*` extensions are not silently discarded.
- Preserve scalar spelling and field-specific short or long forms when normalization could lose
  meaning.
- Preserve enough source and provenance for actionable diagnostics and safe conversion decisions.
- Keep provider/runtime compatibility claims versioned and evidence-backed.
- Treat user input as fallible; malformed input must not panic the process.
- Do not invoke providers, runtimes, networks, or generated commands from the library.
- Redact sensitive values from diagnostics, snapshots, logs, and `Debug` output by default.
- Start repository-owned complete YAML documents with `---`; marker-free YAML is allowed only as
  explicit parser test data.
- Pin every GitHub Action to a full commit SHA and append its exact release tag comment.
- Keep release notes concise and link to canonical technical documentation instead of duplicating
  field and test inventories.

## Canonical development commands

The crate uses Rust 2024, supports Rust 1.85.0 and newer, and pins the normal development toolchain
in `rust-toolchain.toml`.

```console
./scripts/check-all.sh
./scripts/check-files.sh --check
cargo fmt --all -- --check
cargo ci-check
cargo ci-policy
cargo ci-clippy
cargo ci-test
cargo ci-doctest
RUSTDOCFLAGS="-D warnings" cargo ci-doc
cargo +1.85.0 ci-check
cargo +1.85.0 ci-policy
cargo deny check
cargo test --locked --test conformance
cargo test --locked --test runtime_conformance
cargo test --locked --test real_world
cargo test --locked --test public_api
cargo package --locked
```

The `ci-*` aliases use locked resolution and all workspace features and targets where applicable.
Do not weaken checks or lints to accommodate a change. Provider capture remains explicit and ignored
by ordinary tests; its isolation and inputs are documented in `conformance/README.md`.

## GitHub issue-to-PR workflow

When the user authorizes the full Git workflow:

1. Inspect `git status` and the complete diff; preserve unrelated work.
2. Search for a duplicate issue, then create one focused issue if needed.
3. Fetch `origin/main`, synchronize local `main`, and branch as
   `TheRealBecks/issue<NUMBER>`.
4. Complete and review the change without staging unrelated files.
5. Run `./scripts/check-all.sh`. Every step must pass. Any source, test, configuration, or
   documentation edit after a successful run invalidates the gate and requires another full run.
6. Stage only explicit in-scope paths, run `git diff --cached --check`, and review the staged diff.
7. Create one intentional commit, push, and open a ready pull request containing
   `Closes #<NUMBER>`.
8. Read the pull request back and report the issue, branch, commit, validation, URL, and checks.

Opening and reading back the ready pull request is the default stopping point. Authorization to run
the Git workflow or perform GitHub writes does not authorize a merge.

Merge only when the user explicitly authorizes merging the specific pull request or the scoped set
of pull requests in the current request. Immediately before merging, read back the exact head
commit and verify that the pull request is ready, mergeable, and has every required check
successful. Never bypass branch protection, use an administrator override, or infer authority for
an out-of-scope release, publication, or deployment pull request.

Use the repository's normal merge method with an exact-head safeguard, then read back and report
the merged state and merge commit.

Use `feat`, `fix`, `perf`, `refactor`, or `revert` only for release-worthy code changes. Use
`docs`, `test`, `ci`, `build`, `style`, or `chore` for documentation and maintenance so
release-plz ignores them. A failed or incomplete full gate blocks commits, pushes, and pull-request
creation.

The primary Sol agent owns Git and GitHub writes, final integration, full validation, staging, and
readback. Subagents never commit, push, publish, tag, release, or create pull requests.

## Multi-agent coordination

- Delegate only concrete, bounded work with an independently verifiable result.
- Never run two source-writing agents in the same checkout concurrently.
- Agents may write concurrently only in separate checkouts with an agreed public contract.
- Research and review agents stay read-only.
- Run a repository verifier only after writing is complete; verifiers report failures but do not
  modify files.
- The primary agent owns architectural and cross-repository API decisions.
