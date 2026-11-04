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
