# Development environment

The supported setup is the repository's unprivileged VS Code Dev Container. Reopen the checkout in
the container before running the complete gate; it contains the exact Rust and repository tools used
by CI.

## Sources of truth

| Concern                                  | Source                                 |
| ---------------------------------------- | -------------------------------------- |
| Normal Rust toolchain                    | `rust-toolchain.toml`                  |
| Minimum Rust version and package version | `Cargo.toml`                           |
| Rust dependency resolution               | `Cargo.lock`                           |
| Container base and installed tools       | `.devcontainer/Dockerfile`             |
| Dev Container features                   | `.devcontainer/devcontainer-lock.json` |
| Node-based file tools                    | `package-lock.json`                    |
| Native file-tool checksums               | `scripts/install-file-tools.sh`        |

CI and release workflows read versions from these files instead of maintaining another prose copy.
Renovate proposes updates; every proposal receives ordinary review and validation.

## Complete validation

Run:

```console
./scripts/check-all.sh
```

The script formats repository-owned files before validating them. A successful run covers Rust and
non-Rust formatting, workflow security, repository policy, all targets, Clippy, tests, Rustdoc,
package contents, coverage ratchets, MSRV, dependency policy, offline links, and SemVer checks. Any
later edit invalidates the result.

The issue-to-PR sequence and ownership rules are canonical in [`AGENTS.md`](../AGENTS.md). Human and
agent contributors use the same complete gate.

## Focused checks

Use focused commands while iterating:

```console
cargo fmt --all -- --check
./scripts/check-files.sh --check
cargo ci-check
cargo ci-policy
cargo ci-clippy
cargo ci-test
cargo ci-doctest
RUSTDOCFLAGS="-D warnings" cargo ci-doc
cargo deny check
actionlint
zizmor .github/workflows
lychee --config lychee.toml --root-dir . --offline './**/*.md'
```

The VS Code task **ComposeLens: Format, lint, and test all** runs the complete workflow. Smaller tasks
mirror the focused commands.

## Tooling behavior

Authored YAML parser fixtures are not generically formatted because malformed and byte-exact input is
part of their test contract. TOML metadata and complete repository YAML are still validated.

Offline Tombi uses the repository's Cargo-manifest schema so an empty cache behaves like an existing
cache. Cargo commands remain the semantic manifest authority. Routine link checks stay offline;
scheduled automation performs rate-limited external checks.

Coverage and SemVer artifacts use repository-specific writable storage below
`$CARGO_TARGET_DIR/check-all/compose-lens`. This avoids another repository's cleanup, read-only global
package locks, or stale generated Rustdoc projects affecting a result.

## Updating the container

When a Dev Container feature changes, regenerate its committed lock with the pinned CLI version from
the repository configuration, review resolved versions and integrity hashes, rebuild, then run the
complete gate. Do not hand-edit a resolved feature reference merely to satisfy a check.

## External conformance

The default container does not mount Docker or Podman sockets, run systemd, or request privileged
mode. Provider and runtime observations require explicit environments and commands defined in
[`conformance/README.md`](../conformance/README.md). Do not widen every editor session's trust boundary
to run an optional evidence capture.
