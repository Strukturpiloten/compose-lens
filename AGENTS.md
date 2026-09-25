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

For task-related Git and GitHub work authorized below:

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

Use `feat`, `fix`, `perf`, `refactor`, or `revert` only for release-worthy code changes. Use
`docs`, `test`, `ci`, `build`, `style`, or `chore` for documentation and maintenance so
release-plz ignores them. A failed or incomplete full gate blocks commits, pushes, and pull-request
creation.

The primary agent owns Git and GitHub writes, final integration, full validation, staging, and
readback. Subagents never commit, push, publish, tag, release, or create pull requests.

## Workspace scope and standing GitHub authorization

The maintainer grants standing authorization for task-related Git and GitHub work only in these
workspace repositories:

- `Strukturpiloten/boxferry`
- `Strukturpiloten/compose-lens`
- `Strukturpiloten/podman-lens`
- `Strukturpiloten/quadlet-lens`
- `Strukturpiloten/boxferry-website`
- `Strukturpiloten/docker-lens`

Do not work on or modify any repository outside this explicit allowlist, including its issues,
pull requests, branches, settings, or workflows. An upstream documentation reference is not
permission to operate on that upstream repository. A newly discovered checkout is not implicitly
in scope.

For user-requested work within this scope, the primary agent may create issues, branches, commits,
pushes, and pull requests and merge verified task-related pull requests without asking for renewed
approval. This permission does not authorize unrelated backlog work, implementation of
discussion-only proposals, or expansion of the requested product scope. A later user instruction
may narrow or revoke this permission.

Immediately before merging, read back the exact head commit and verify that the pull request is
ready, mergeable, independently reviewed, and has every required check successful. Use the normal
merge method with an exact-head safeguard; never bypass branch protection or use an administrator
override. Read back the merged state and merge commit, synchronize local `main` with `origin/main`,
and remove the task's recorded worktrees and verified merged local branches while preserving
unrelated work.

This standing permission does not authorize releases, publication, deployment operations, or
merging release/publication/deployment pull requests; those require a separate explicit request.
The primary agent owns all Git and GitHub writes. Subagents remain within their assigned task and
checkout and must not perform those writes.

## Multi-agent coordination

- Delegate only concrete, bounded work with an independently verifiable result.
- Never run two source-writing agents in the same checkout concurrently.
- Agents may write concurrently only in separate checkouts with an agreed public contract.
- Research and review agents stay read-only.
- Run a repository verifier only after writing is complete; verifiers report failures but do not
  modify files.
- The primary agent owns architectural and cross-repository API decisions.

## Agent roles and verification

Model defaults belong in [`.codex/config.toml`](.codex/config.toml); task-specific models and
reasoning belong in [`.codex/agents/`](.codex/agents/). The primary manager always uses
`gpt-6-astra` with `xhigh` reasoning. Implementation, specification research, and independent review
use `gpt-6-sol` with `high` reasoning; check-only verification uses `gpt-6-luna` with `high`
reasoning. Use Luna for bounded read-only exploration and Sol for difficult failure diagnosis.
These model settings do not expand the workspace scope or grant additional permissions.

- Delegate bounded tasks when independent work can usefully proceed in parallel. Define the shared
  contract and explicit repository, checkout, and file ownership before delegation.
- Use up to nine concurrent subagents plus the primary manager, subject to the session's actual
  runtime limit. Nine is a ceiling, not a target or nine distinct roles: several subagents may use
  the same role for independent tasks. Do not create nested agents to evade the limit.
- Never run two writers in one checkout. Use separate assigned repositories or worktrees for
  concurrent implementation. Research and review remain read-only.
- The reviewer checks the original requirements and independent expected results, not just agreement
  between the implementation and its tests.
- After writing finishes, the verifier runs `./scripts/check-all.sh --check`. It reports failures
  without formatting or editing tracked files; ignored build artifacts and caches are allowed.
- Run at most one complete gate or heavy runtime suite at a time across this workspace. Agent
  concurrency is not permission for competing builds. The primary owns integration, the final
  complete gate, and every authorized Git or GitHub write.

The default `./scripts/check-all.sh` still formats before checking. `--check` runs the same
complete gate without source formatting; it is not a reduced test tier. A later edit invalidates
either result. Neither mode grants release, publication, or deployment authority.

## Code discovery

For code discovery, use an available codebase-memory graph first; otherwise use CodeGraph only if
the repository already has a usable index. Do not create an index without user authorization.
If neither graph is available or a query cannot answer the question, use `rg` and targeted reads.
For string literals, configuration, scripts, and documentation, start with `rg` directly.

## After an authorized merge

Read back the merged state and exact merge commit, then synchronize the primary checkout with
`origin/main`. Preserve unrelated files. Remove only the recorded task worktree with
`git worktree remove <recorded-path>`, delete the verified merged local issue branch with
`git branch --delete --force TheRealBecks/issue<NUMBER>`, and run
`git worktree prune --verbose`. Read back `git worktree list --porcelain` and
`git status --short --branch`; do not leave stale task worktree registrations.
