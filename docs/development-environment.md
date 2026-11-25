# Development environment

Use VS Code Dev Containers with the existing configured Podman engine and storage; rootless
operation is preferred. The candidate recipe now uses the same canonical tooling as local checks. Successful image
qualification remains separate from this source integration.

## Sources of truth

| Concern                                  | Source                                           |
| ---------------------------------------- | ------------------------------------------------ |
| Normal Rust toolchain                    | `rust-toolchain.toml`                            |
| Minimum Rust version and package version | `Cargo.toml`                                     |
| Rust dependency resolution               | `Cargo.lock`                                     |
| Container base and installed tools       | `.devcontainer/Dockerfile`                       |
| Dev Container features                   | `.devcontainer/devcontainer-lock.json`           |
| Node-based file tools                    | Selected shared `package-lock.json`              |
| Downloaded tool versions and checksums   | Selected shared `automation/tool-artifacts.json` |

CI and release workflows read versions from these files instead of maintaining another prose copy.
Ordinary development updates merge automatically after the required integrity and compatibility
checks. Qualified provider and workload targets remain fixed under the dependency policy.

The Bookworm recipe retains its exact signed `python3-venv` package and Feature integrity graph.
Downloaded tools come from canonical shared version/checksum pairs. Image and Feature proposals
remain disabled pending fresh standard GitHub CI qualification; ordinary dependency updates are
not held by that image rule.

Repository automation uses `scripts/shared-ci.py` and the exact revision in
`.github/automation.json`. The shared locked Python project owns uv, Ruff, ty, and their integrity
metadata; consumers do not duplicate these package versions. Its verified local bootstrap starts
from Python. The separate system venv/pip probe remains necessary for native provider-bootstrap
tests. A development implementation override is explicitly marked mutable and cannot establish
publication evidence.

Local, CI and image file tools, including Actionlint, use `scripts/install-host-tools.sh` and the
canonical shared inventory. The complete local gate provisions its own pinned Actionlint
before linting so an older binary on `PATH` cannot take precedence. The compatibility `scripts/install-file-tools.sh` forwards to this
same launcher. Python tooling is bootstrapped from the selected locked project. Package and
compiler declarations keep their existing ordinary owners even when an image consumes those files.

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
python3 scripts/shared-ci.py tools --destination target/workflow-lint-tools --tools actionlint
target/workflow-lint-tools/actionlint
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

Direct image and Feature proposals remain disabled until fresh standard
GitHub CI qualification is reviewed under issue #563. Retain the reviewed image pins and Feature lock,
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
The CI qualification recipe calls that same launcher and separately exports canonical Python
tools. The compatibility installer has no independent inventory or update owner. Image and
Feature proposals stay disabled until fresh GitHub qualification; this source integration does
not establish native qualification or change ordinary package/compiler ownership.

Node file checks dispatch through `scripts/node-tool.sh` and the selected shared
`node-tool` interface. The shared source owns the npm manifests, package versions and
lock integrity; `.github/node-tools.json` declares that ownership for image qualification.
Ordinary hosts acquire tools through the existing verified shared bootstrap. CI keys
its npm cache with the immutable checkout lock and uses the same dispatch.
ComposeLens retains its Prettier and Markdownlint configuration, ignore rules and
fixture exclusions; checks explicitly select these configs and disable EditorConfig.

The image installs once with `node-tool --installation-root /opt/compose-lens-file-tools
--install-only` and exposes its public `node_modules/.bin` for native qualification probes.
Fixed links in `/usr/local/bin` retain discovery under the finite developer and provider
runtime PATHs; format operations still use the selected-installation dispatcher.
It sets `COMPOSE_LENS_NODE_TOOLS_ROOT` so subsequent checks select the retained
`/opt/compose-lens-file-tools/boxferry-node-tools.py` image entry point for read-only
verification and execution. Unexpected image roots are rejected before loading code.
The retained entry point checks the checkout's selected revision without bootstrapping,
fetching or writing a checkout cache. Hosts leave this variable unset; installation
directory presence alone never selects an image. A stale image fails verification and
requires rebuilding against the checkout selected automation revision.
