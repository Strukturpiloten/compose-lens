# Development environment

The supported VS Code setup is the repository's Dev Container. Open the repository in VS Code
and run **Dev Containers: Reopen in Container**. The default container is unprivileged and is
suitable for parser, model, rendering, documentation, and repository-policy work.

## Sources of truth

- `rust-toolchain.toml` selects the current Rust toolchain and its components.
- `rust-version` in the workspace `Cargo.toml` declares the MSRV. CI reads it from Cargo
  metadata; the workflow does not duplicate it.
- `Cargo.toml` declares package versions. Release workflows read package versions from Cargo
  metadata rather than accepting a manually typed version.
- `.devcontainer/Dockerfile` records the exact base-image release and pins its digest.
- `.devcontainer/devcontainer-lock.json` pins resolved Dev Container feature versions and
  integrity hashes.
- `.devcontainer/Dockerfile` pins repository tooling; Renovate owns version proposals.

## Shared editor setup

Repository settings in `.vscode/settings.json` enable format-on-save, exclude Cargo output from
search and file watching, use all Cargo features in rust-analyzer, and select the matching
formatter for Rust, Markdown, JSON, YAML, TOML, and shell files. `.vscode/extensions.json`
contains recommendations for contributors who open the checkout outside the container.

Use **Tasks: Run Test Task** or **ComposeLens: Format, lint, and test all** for the complete local
workflow. **ComposeLens: Required Rust checks** and the focused format, check, policy, Clippy,
test, documentation, conformance, public-API, and package tasks support shorter edit cycles.

## Included tools

The image provides Rust, Cargo, rustfmt, Clippy, rust-analyzer, CodeLLDB, Git, GitHub CLI, Node.js,
`cargo-deny`, `cargo-llvm-cov`, `cargo-semver-checks`, `lychee`, `zizmor`, `actionlint`, Prettier,
markdownlint-cli2, Tombi, shfmt, ShellCheck, and Hadolint. Node tools use the committed lockfile;
downloaded native tools use reviewed SHA-256 checksums.

Run the complete deterministic local workflow with:

```console
./scripts/check-all.sh
```

It formats Rust and owned non-Rust files, then runs all deterministic checks. It intentionally
does not execute the ignored provider capture lane. Run only the non-Rust layer with
`./scripts/check-files.sh --fix`; CI and release validation use its non-mutating `--check` mode.

## Issue-to-PR contribution workflow

For an issue-backed change, inspect the complete working-tree diff first and preserve unrelated
work. Search for a duplicate issue, create the issue, synchronize local `main` with `origin/main`,
and create `TheRealBecks/issue<NUMBER>`. After the implementation and final diff review, run:

```console
./scripts/check-all.sh
```

All steps must pass before the change is committed, pushed, or submitted as a pull request. If any
source, test, configuration, or documentation file changes after that successful run, run the
complete task again. Then stage only explicit in-scope paths, check the staged diff, commit, push,
and open a ready-for-review pull request containing `Closes #<NUMBER>`. Read the created pull
request back from GitHub to verify its base branch, head branch, issue linkage, draft state, and
check status.

The pull-request title becomes the squash commit title and controls automated release eligibility.
Use `feat`, `fix`, `perf`, `refactor`, or `revert` only for release-worthy code changes, with an
optional scope and breaking `!`. Use `docs`, `test`, `ci`, `build`, `style`, or `chore` for
non-code work; release-plz ignores those commits and excludes them from generated release notes.

The primary Sol agent uses high reasoning effort and owns the issue, branch, final integration,
complete local gate, staging, commit, push, pull request, and GitHub readback. Terra agents may
perform bounded implementation, research, read-only review, or non-mutating verification, but they
never perform Git or GitHub writes. The formatting `./scripts/check-all.sh` task therefore remains
Sol's final responsibility.

Coverage starts by removing its complete repository-specific artifact tree, so the persistent
target volume cannot retain a fingerprint for a missing test executable and another repository's
concurrent cleanup cannot remove a ComposeLens executable.
Authored YAML fixtures are excluded from generic formatting because malformed and byte-exact
source is part of their test contract. TOML fixture metadata and retained conformance records are
syntax-checked but not rewritten.
Offline Tombi runs select the repository's structural Cargo-manifest schema instead of Tombi
1.4.0's embedded Cargo schema, whose lint subsections reference two remote-only schemas. Cargo
metadata, checks, Clippy, tests, and packaging remain the authoritative semantic manifest
validation. This keeps an empty CI cache equivalent to an established local cache without
disabling schema discovery for other TOML files.
Routine documentation-link checks are offline. A scheduled workflow performs the slower,
rate-limited external-link check.

The script keeps coverage and API-compatibility artifacts under
`$CARGO_TARGET_DIR/check-all/compose-lens`. It never reuses the container image's
`/usr/local/cargo`, another repository's validation tree, or an older `target/semver-checks` tree,
because those locations can contain read-only package locks or incompatible build artifacts.
Registry downloads, the SemVer check's exclusive lock, and its generated rustdoc projects
therefore stay in writable, repository-specific build storage.

Useful focused checks include:

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
cargo semver-checks check-release --package compose-lens
```

## Updating the container

Renovate proposes updates for the base image, feature references, Rust toolchain, and installed
tools. When a feature changes, regenerate the committed lock file with the pinned CLI version:

```console
npx --yes @devcontainers/cli@0.88.0 upgrade --workspace-folder .
```

Review the resolved version and integrity changes, rebuild the container, and run the repository
checks before merging.

## Runtime conformance

The default container deliberately does not mount a Docker or Podman socket, run systemd, or
request privileged mode. Runtime and Quadlet-generator conformance requires explicit, isolated
test environments with documented engine versions, privileges, and host requirements. Those
environments will be added with their test harnesses instead of expanding the trust boundary of
every editor session.
