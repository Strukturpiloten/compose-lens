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
- Keep release validation unprivileged and reusable: validation-only dispatches must be unable to
  reach publication, and native provider evidence must bind the exact candidate, run, and task.
- Keep release notes concise and link to canonical technical documentation instead of duplicating
  field and test inventories.

## Canonical development commands

Rust 2024 supports Rust 1.85.0+; `rust-toolchain.toml` pins the normal toolchain.

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

`ci-*` aliases use locked resolution and applicable workspace features/targets. Do not weaken
checks or lints. Provider capture is explicit, ignored by ordinary tests, and documented in
`conformance/README.md`.

## GitHub issue-to-PR workflow

For authorized task-related Git/GitHub work: inspect status and full diff; preserve unrelated work;
search duplicates and create one focused issue when needed; fetch `origin/main`, synchronize
`main`, and branch `TheRealBecks/issue<NUMBER>`; then complete and review the scoped change.

Run `./scripts/check-all.sh`; every step must pass. Any later source, test, configuration, or
documentation edit invalidates the gate. Stage explicit in-scope paths, run `git diff --cached --check`,
review the staged diff, make one intentional commit, push, and open a ready pull request with
`Closes #<NUMBER>`. Read back and report issue, branch, commit, validation, URL, and checks.

Use `feat`, `fix`, `perf`, `refactor`, or `revert` only for release-worthy code changes. Use
`docs`, `test`, `ci`, `build`, `style`, or `chore` for documentation and maintenance so
release-plz ignores them.
A failed or incomplete full gate blocks commits, pushes, and pull-request creation.

The primary agent owns Git and GitHub writes.
Subagents never commit, push, publish, tag, release, or create pull requests.

## Workspace scope and standing GitHub authorization

The maintainer grants standing authorization for task-related Git and GitHub work only in these
workspace repositories:

- `Strukturpiloten/boxferry`
- `Strukturpiloten/compose-lens`
- `Strukturpiloten/podman-lens`
- `Strukturpiloten/quadlet-lens`
- `Strukturpiloten/boxferry-website`
- `Strukturpiloten/docker-lens`

Do not work on or modify any repository outside this explicit allowlist, including issues, pull
requests, branches, settings, or workflows. An upstream documentation reference is not permission
to operate there. A newly discovered checkout is not implicitly in scope.

For user-requested work within this scope, the primary agent may create issues, branches, commits,
pushes, and pull requests and merge verified task-related pull requests without asking for renewed
approval. This permission does not authorize unrelated backlog work or implementation of
discussion-only proposals. It does not expand the requested product scope. A later user instruction
may narrow or revoke this permission.

Immediately before merging, read back the exact head commit and verify the pull request is
ready, mergeable, independently reviewed, and has every required check successful. Use the normal merge
method with an exact-head safeguard; never bypass branch protection or use an administrator
override. Read back the merged state and merge commit, synchronize local `main` with `origin/main`,
and remove the task's recorded worktrees and verified merged local branches while preserving
unrelated work. Use `git worktree remove <recorded-path>`,
`git branch --delete --force TheRealBecks/issue<NUMBER>`, and `git worktree prune --verbose`; read
back `git worktree list --porcelain` and `git status --short --branch` with no stale registrations.

This standing permission does not authorize releases, publication, deployment operations, or
merging release/publication/deployment pull requests; those require a separate explicit request.
The primary agent owns all Git and GitHub writes. Subagents remain within their assigned task and
checkout and must not perform those writes.

## Multi-agent coordination

- Delegate only concrete, bounded, independently verifiable work. Writers need separate checkouts
  and an agreed public contract; never use two in one checkout. Research/review is read-only.
- Run a verifier only after writing is complete; it reports failures without edits. The primary owns
  architectural and cross-repository API decisions.

## Agent roles and verification

Model defaults: [`.codex/config.toml`](.codex/config.toml); task settings:
[`.codex/agents/`](.codex/agents/). The primary manager always uses `gpt-6-astra` with `xhigh` reasoning;
implementation/research/review use `gpt-6-sol` with `high` reasoning; check-only verification uses
`gpt-6-luna` with `high` reasoning. Use Luna for bounded read-only exploration and Sol for difficult
failure diagnosis. Models do not expand scope or permissions.

- Before delegation, define contract, repository, checkout, and file ownership.
  Use up to nine concurrent subagents plus the primary manager, subject to the runtime limit.
  Nine is a ceiling, not a target or nine distinct roles. Do not create nested agents to evade the limit.
- The reviewer checks the original requirements and independent expected results, not just agreement
  between the implementation and its tests.
- After writing finishes, the verifier runs `./scripts/check-all.sh --check` without tracked-file edits;
  ignored build artifacts/caches are allowed.
  Run at most one complete gate or heavy runtime suite at a time across this workspace:
  concurrency does not permit competing builds. The primary owns integration, the final gate, and
  authorized Git/GitHub writes.

Default `./scripts/check-all.sh` formats; `--check` is the same gate without source formatting, not
a reduced tier. A later edit invalidates either; neither grants release, publication, or deployment.

## Cross-repository workflow and version policy

- Align equivalent local/PR/main/release definitions across BoxFerry, ComposeLens, PodmanLens,
  QuadletLens, DockerLens, and the website. Identify canonical definitions and all consumers;
  coordinate updates, validate each consumer, and link justified differences/follow-ups.
- Reuse shared scripts/actions/workflows; preserve native conformance and thresholds. BoxFerry
  owns application suites; Lens products must not depend on it.
- Added/changed software needs explicit versions and supported integrity records: image
  version tags plus digests; Action/workflow full SHAs plus exact release-tag comments;
  downloaded-tool versions plus verified checksums; package declarations plus lockfile integrity.
  Document unavailable-integrity exceptions; never invent checksums or weaken reviewed pins.
- Added/changed/moved/removed pins or definitions require same-change Renovate ownership,
  paths/extraction, grouping, approvals, and regression review. Update configuration/consumers
  together or document verified no-change evidence. Avoid duplicate managers; preserve historical
  evidence and intentional fixtures. Follow `docs/dependency-policy.md`.
- Preserve least privilege, exact-candidate evidence, failure propagation, budgets, privacy, and
  cleanup. Follow `docs/releasing.md`; never claim planned automation is delivered. One passing
  repository does not prove rollout completion. These rules grant no additional authority.

## Code discovery

For code discovery, use an available codebase-memory graph first; otherwise use CodeGraph only if
the repository already has a usable index. Do not create an index without user authorization.
If neither graph is available or a query cannot answer the question, use `rg` and targeted reads.
For string literals, configuration, scripts, and documentation, start with `rg` directly.
