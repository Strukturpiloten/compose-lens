# Development environment

Use VS Code Dev Containers with the existing configured Podman engine and storage; rootless
operation is preferred. The current image remains unchanged. Ordinary tooling follows the
manifest-selected shared implementation; rebuilding or qualifying an image is separate work.

## Sources of truth

| Concern                                  | Source                                 |
| ---------------------------------------- | -------------------------------------- |
| Normal Rust toolchain                    | `rust-toolchain.toml`                  |
| Minimum Rust version and package version | `Cargo.toml`                           |
| Rust dependency resolution               | `Cargo.lock`                           |
| Container base and installed tools       | `.devcontainer/Dockerfile`             |
| Dev Container features                   | `.devcontainer/devcontainer-lock.json` |
| Node-based file tools                    | `package-lock.json`                    |
| Legacy image file-tool checksums         | `scripts/install-file-tools.sh`        |

CI and release workflows read versions from these files instead of maintaining another prose copy.
Ordinary development updates merge automatically after the required integrity and compatibility
checks. Qualified provider and workload targets remain fixed under the dependency policy.

The existing Bookworm image retains its exact signed `python3-venv` package, legacy file tools
and Feature integrity graph. Image and Feature proposals remain disabled pending fresh standard
GitHub CI qualification; ordinary dependency updates are not held by that image rule.

Repository automation uses `scripts/shared-ci.py` and the exact revision in
`.github/automation.json`. The shared locked Python project owns uv, Ruff, ty, and their integrity
metadata; consumers do not duplicate these package versions. Its verified local bootstrap starts
from Python. The separate system venv/pip probe remains necessary for native provider-bootstrap
tests. A development implementation override is explicitly marked mutable and cannot establish
publication evidence.

Ordinary host file tools use `scripts/install-host-tools.sh` and the canonical shared inventory.
Python tooling is bootstrapped from the selected locked project. The retained image-only installer
is not a fallback for host or CI tooling. Package and compiler declarations keep their existing
ordinary owners even when an image consumes those files.

## Complete validation

Run:

```console
./scripts/check-all.sh
```

The gate first creates a temporary isolated Python venv and checks its own pip. This fails quickly
with a Dev Container rebuild hint if `ensurepip` or `python3-venv` is missing. Dev Container
post-create verification and CI's Rust quality job check the same contract before expensive steps.
The probes remove their temporary directories on normal success, failure, and handled interruption.

The script formats repository-owned files before validating them. A successful run covers Rust and
non-Rust formatting, workflow security, repository policy, all targets, Clippy, tests, Rustdoc,
package contents, the offline provider-bootstrap regression, coverage ratchets, MSRV, dependency
policy, offline links, and SemVer checks. Any later edit invalidates the result.

The complete ordinary gate retains all Rust, provider, policy, coverage and API checks. It does
not build or run the Dev Container. Use VS Code Dev Containers with the existing configured
Podman engine and storage; rootless operation is preferred. No private graph root, root bridge
or mandatory local image controls are introduced. Fresh image qualification runs separately on
standard GitHub CI before image-update activation under issue #563.

The issue-to-PR sequence and ownership rules are canonical in [`AGENTS.md`](../AGENTS.md). Human and
agent contributors use the same complete gate. The local change-aware planner and VS Code tasks
can shorten iteration on public prose, but do not replace the complete pre-publication run.

Ordinary adoption uses the existing local checks and configured Podman setup in VS Code Dev
Containers. It adds no local image, private-store or privilege prerequisite. Fresh standard
GitHub CI image qualification under issue #563 remains separate from ordinary adoption.

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
package locks, or stale generated Rustdoc projects affecting a result. Before SemVer analysis,
the shared loader verifies the declared Cargo tool and Rust channel using that same isolated
Cargo home.

## Updating the container

Direct image, Feature and legacy image-tool proposals remain disabled until fresh standard
GitHub CI qualification is reviewed under issue #563. Retain the current image, Feature lock,
checksums and VS Code lifecycle definitions meanwhile. Rebuilding the existing setup does not
activate the deferred updater or establish new qualification evidence.

## External conformance

The default container does not mount Docker or Podman sockets, run systemd, or request privileged
mode. Provider and runtime observations require explicit environments and commands defined in
[`conformance/README.md`](../conformance/README.md). Do not widen every editor session's trust boundary
to run an optional evidence capture.

## Agent-assisted verification

Repository model defaults and role overrides live in [`.codex/`](../.codex/); permissions and
workflow ownership remain defined in [`AGENTS.md`](../AGENTS.md). Reload or start a new trusted
project session after updating configuration; an explicit session override can take precedence.
Keep any explicit primary-session override aligned with Sol/xhigh.

Use `./scripts/check-all.sh --check` to run the complete gate without formatting repository-owned
files. The default command (or `--fix`) still formats first. Both modes run the same validation;
ignored caches and build artifacts may change. Verifiers report failures without fixing files,
and the primary agent owns the final complete gate and merges covered by the standing
authorization in [`AGENTS.md`](../AGENTS.md).

The shell-runner regression tests target the Linux Dev Container gate. Agent-configuration checks
remain platform-independent. macOS client compatibility is intended but not verified by CI; the
CI and release validation lanes are Linux-only.

Ordinary downloaded tools use `scripts/install-host-tools.sh`, the selected immutable shared
implementation and its architecture-specific integrity inventory. CI uses this same launcher.
Run it with a caller-owned destination and put that directory on PATH; it does not require sudo.
The existing Dev Container alone retains `scripts/install-file-tools.sh` and its pinned legacy
image tools. Image and Feature proposals stay disabled until fresh GitHub qualification. The
image closure is unchanged and remains distinct from ordinary package/compiler/tool ownership.
