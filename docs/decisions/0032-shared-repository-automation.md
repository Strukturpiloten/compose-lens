# ADR 0032: Shared repository automation with trusted prerequisite rollout

- Status: accepted
- Date: 2026-11-03

## Decision

BoxFerry owns reusable repository automation, Python tooling, and Renovate presets. ComposeLens
retains its independent Cargo graph, native conformance, API checks, coverage thresholds, and
maintainer-approved publication. Automation reuse introduces no library dependency on BoxFerry.

Roll out in two reviewed stages. First install the default-branch dependency eligibility service,
trusted task policy and `.github/automation.json` manifest, and conservative complete PR and merge-queue validation while retaining every existing
check. Then adopt the shared planner, read-only eligibility waiter, workflows, and presets at the
same verified immutable BoxFerry revision. The waiter cannot be enabled before the trusted service, manifest, and task policy reach main.
The service uses default-branch `workflow_run` and scheduled triggers; privileged PR-target execution
is excluded. The shared read-only waiter resolves admission implementation A from the exact trusted
base manifest Git object, even when the candidate proposes automation B. A receipt never claims
B-policy admission before B reaches the trusted base. PR and queue policies come from their trusted base; unknown or mixed
changes fail closed to full validation. Main, manual, local final, and release validation stay full.

The initial infrastructure has no released software tag. Its exact SHA carries the narrow
`automation-v1 interface v1` comment for the explicitly enumerated shared workflows; moving refs,
short hashes, other workflows, and unversioned comments remain rejected. Renovate owns the SHA.

Ordinary dependency updates need no routine human approval and retain a 72-hour third-party age
hold. A verified isolated first-party release update may waive age for its necessary resolution;
integrity, audit, compatibility, and exact-candidate gates remain mandatory. Historical provider
versions, hashes, records, and fixtures remain immutable evidence. Software publication still
requires maintainer approval of the protected release environment.

Rollback restores the last verified immutable automation revision and conservative complete CI;
never run a candidate-controlled privileged service or bypass the Required CI aggregate.

## 2026-11-06 reconstruction record (historical)

This section records the reconstruction stage and its source-admission hold. The later ordinary
adoption decision below defines the current scope; these historical prerequisites are not
additional local development requirements.

The implementation now delegates Rust orchestration, dependency eligibility, release preparation,
crate publication mechanics, Renovate policy and locked typed Python tooling to one immutable
BoxFerry interface. ComposeLens retains native provider/application commands, its Cargo aliases,
coverage 88/87/89 and its protected release hooks. The Python planner is a thin dispatch entry point.
Narrow trusted profiles combine by union; unknown or mixed configuration inputs fail closed.

Development API checks use the trusted default-branch baseline and an explicit minor development
purpose. Release requests published purpose and propagates the actual shared API job's distinct
published receipt; a generic candidate receipt is not published API proof. The shared release
gate's distinct-purpose enforcement correction must be admitted before qualification.

## 2026-11-06 ordinary adoption and deferred images

Ordinary host/CI automation uses one admitted immutable source. The ordinary Required CI gate
retains every product/provider job, coverage threshold, exact-candidate outcome and published API
proof. Draft-added image jobs, seed gating and the local image leaf are outside this adoption.

VS Code Dev Containers use the existing Podman setup and storage, preferably rootless. No custom
private store, mandatory local sudo or root-control ceremony is required for ordinary development.
The direct image/lifecycle definitions and Feature lock return to their current-main state. Their
legacy image installer has one extraction owner and a late disabled proposal rule; ordinary host
and CI callers use canonical shared tools without a legacy fallback. Package/compiler declarations
keep their ordinary owners. The unchanged image closure is explicitly unqualified by this change.

Fresh standard GitHub CI image qualification and later updater activation remain separate under
issue #563. No image build, runtime mutation, App activation, publication or deployment is performed
by ordinary adoption. Native provider evidence and maintainer-approved release boundaries remain.
