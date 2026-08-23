# Research evidence

This directory retains versioned technical evaluations used to make ComposeLens decisions. A note
records what was reviewed at a particular time; it is not a current blanket support claim.

| Topic                      | Evidence                                                                                               |
| -------------------------- | ------------------------------------------------------------------------------------------------------ |
| YAML representation        | [`yaml-representation.md`](yaml-representation.md)                                                     |
| Syntax-form fidelity       | [`compose-syntax-fidelity.md`](compose-syntax-fidelity.md)                                             |
| Interpolation              | [`compose-interpolation.md`](compose-interpolation.md)                                                 |
| Multi-file merge           | [`compose-merge.md`](compose-merge.md)                                                                 |
| Post-merge processing      | [`compose-post-merge-processing.md`](compose-post-merge-processing.md)                                 |
| Compatibility profiles     | [`compose-compatibility-profiles.md`](compose-compatibility-profiles.md)                               |
| Canonical rendering        | [`compose-canonical-rendering.md`](compose-canonical-rendering.md)                                     |
| Provider observations      | [`provider-config-conformance-2026-07-31.md`](provider-config-conformance-2026-07-31.md)               |
| Upstream regression review | [`podlet-compose-spec-rs-regressions-2026-08-01.md`](podlet-compose-spec-rs-regressions-2026-08-01.md) |

Current contracts live in architecture documentation, tests, conformance matrices, and accepted
ADRs. Update or add research evidence when a new decision depends on different specifications,
versions, or observable behavior; do not silently rewrite an old observation into a new one.
