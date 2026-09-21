# ADR 0027: Release-gated observed provider-configuration evidence

- Status: accepted
- Date: 2026-09-21

## Context

Retained provider records are historical observations and do not show that the exact release
candidate still reproduces their bounded configuration results. Running planned provider or
runtime-effect rows would make a release an unbounded deployment test.

## Decision

`provider-conformance.yml` is the reusable native validation workflow. Scheduled and manual
diagnostics and Release invoke the same six-target worker matrix. Each worker runs exactly the
eight `status = "observed"` configuration rows for one pinned provider (48 rows total), writes a
new unreviewed capture, and retains an artifact named from the GitHub run ID and target. Retries
overwrite only that stable task artifact.

A successful worker requires fresh version and probe status, exit code, standard output, and
standard error to match the independently reviewed record. The comparator normalizes only
review-admitted machine paths (repository, acquisition root, Python standard library, and Python
site packages); it does not rewrite provider diagnostics or configuration output.

Workers check out and record the exact candidate SHA. The existing harness clears provider
environments and uses no daemon socket; `podman-compose` retains `--dry-run`. A failed, cancelled,
timed-out, absent, or skipped worker blocks the Release dependent job. Planned and runtime-effect
rows remain outside this workflow and retain no release-support claim.

## Consequences

- Each release candidate has current, traceable configuration evidence without claiming deployment
  or runtime-effect conformance.
- Provider versions and artifact checksums remain the reviewed matrix source of truth; workflow
  action pins remain Renovate-owned.
- Captures are unreviewed unless separately admitted through ADR 0012's record process.

## Alternatives

### Run planned or runtime-effect rows during release

Rejected because they are not evidence-backed and can require privilege and mutable host state.

### Treat retained records as release validation

Rejected because records do not bind a new candidate or workflow run.
