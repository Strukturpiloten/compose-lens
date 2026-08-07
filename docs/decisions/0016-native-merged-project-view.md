# ADR 0016: Native merged-project consumer view

- Status: accepted
- Date: 2026-08-02
- Additive amendments: 2026-08-03, 2026-08-05, 2026-08-06, 2026-08-07

## Context

ComposeLens already retained correct multi-file semantics and provenance in `MergedProject`, but
the public post-merge representation was a generic `MergedValue` tree. A native adapter such as
BoxFerry therefore had two poor choices: duplicate Compose field interpretation over
`MergedValue`, or render canonical YAML and parse it again as a `ComposeDocument`. The latter
replaces original source locations with generated-output spans and cannot explain which input file
contributed an effective value.

The first conversion boundary needs native images, commands, environment, ports, volumes, service
networks, profiles, and top-level resources after interpolation, merge, and profile selection.
It also needs an explicit path for fields outside that boundary without pretending that they were
converted.

## Decision

ComposeLens adds the public `project` module and `build_project_view` operation:

1. The builder consumes a `MergedProject` and an optional matching `ProfileSelection` directly.
   It never renders, reparses, reads files, reads environment variables, or invokes a provider.
2. A matching selection includes only active services. Omitting the selection includes every
   service. A selection from another project returns the existing stable mismatch diagnostic and
   no view.
3. The initial view exposes native effective values for the BoxFerry conversion boundary:
   project name; services; images; commands; environment; extra hosts; health checks; service
   dependencies; ports; volume mounts; network attachments; profiles; and top-level network,
   volume, config, and secret definitions.
4. `ProjectValue<T>` wraps every exposed effective value with the complete `MergeProvenance`, the
   effective source, and sensitivity state. Collections also retain per-item provenance.
5. `ProjectKey` retains every authored key span. Environment entries are normalized by semantic
   key after Compose field-specific merging while retaining each effective `EntrySyntax`.
6. Fields outside the initial project-view boundary remain available as
   `ProjectFieldReference` values with semantic paths, all key sources, value provenance,
   extension classification, and sensitivity state.
7. Sensitive project values redact their contents from `Debug` output.
8. Invalid effective forms return stable source-spanned diagnostics and as much partial native data
   as can be represented.
9. Effective `extra_hosts` entries retain hostname key sources, address provenance, sequence or
   mapping syntax, and raw-preserving address classification. `host-gateway` remains an explicit
   implementation token instead of being forced into an IP-address grammar.
10. Effective `depends_on` retains short versus long syntax, ordered service-name provenance, and
    nested `condition`, `restart`, and `required` provenance. Unknown and extension options remain
    source-addressable. The view exposes authored intent; it does not claim that another lifecycle
    manager has equivalent startup, health, completion, restart-propagation, or optionality
    semantics.
11. Effective execution identity exposes raw-preserving `user` and `userns_mode`, ordered
    `group_add`, `working_dir`, and `read_only` values. The view classifies only lexical forms and
    known namespace modes; it does not resolve host accounts, groups, paths, or runtime state.
12. Effective service config and secret grants retain short versus long syntax, item provenance,
    and field-level provenance for `source`, `target`, `uid`, `gid`, and `mode`. Unknown options
    remain source-addressable, and unique-by-target merging is not flattened into replacement.
13. Effective service labels are normalized by semantic key after Compose field-specific merging.
    Each entry retains mapping, `KEY=VALUE`, or key-only list syntax plus complete key and value
    provenance. Key-only labels expose an empty string rather than inheriting environment-variable
    host-resolution semantics.
14. Effective service devices retain an optional ordered mixed short/long sequence. Raw path,
    CDI-like, deferred, and opaque short strings remain authoritative. Long `source`, `target`, and
    `permissions` values plus extensions and unknown fields retain nested provenance. The existing
    target-keyed replacement rule remains observable alongside reset, override, duplicates, and
    interpolation sensitivity without device, permissions, CDI, GPU, or runtime validation.
15. Effective `dns` and `dns_search` retain scalar/list form, append/replacement rules,
    provenance, duplicates, sensitivity, reset, and override; `dns_opt` retains the same evidence
    with whole-sequence replacement.
16. Effective `expose` retains ordered YAML scalar identity and documented decimal classification
    without inferring a protocol or runtime publication.
17. Effective `annotations` retain mapping/list syntax and keyed contributors. Mapping keys do
    not interpolate, and ambiguous key-only entries receive no invented value.
18. Effective `security_opt` retains the raw appended sequence. Exact AppArmor,
    no-new-privileges, seccomp, SELinux-label, Mask, and Unmask shapes are independent diagnostic
    candidates; conflicts and near misses remain unselected.
19. These views perform no resolver, profile, path, filesystem, provider, runtime, or cross-format
    interpretation.
20. Effective service `logging` retains an uninterpreted optional string driver and optional
    ordered options mapping. String/number/null option kinds, authored and interpolated spelling,
    nested recursive-merge and replacement provenance, empty/reset/override state, sensitivity,
    extensions, unknowns, and malformed entries remain source-aware without provider semantics.

The generic `MergedProject` remains public for advanced analysis, validation, rendering, and future
native-boundary expansion. The new view is additive within the 0.1.x compatibility line.

## Consequences

- BoxFerry can migrate from one-document import to a real multi-file, profile-selected input
  without owning Compose merge-tree interpretation.
- Conversion outcomes can cite every source file that contributed to an effective value.
- Mixed list/mapping environment merges do not have to be forced back into one invented document
  syntax.
- Native project coverage can grow field by field while unmodeled references prevent silent loss.
- `ProjectValue<T>` is intentionally separate from single-document `Located<T>`: one effective
  value can have several contributing spans.

## Alternatives considered

- Canonical render and `ComposeDocument` reparse was rejected because generated spans erase
  original multi-file provenance.
- Exposing only convenience lookup methods on `MergedValue` was rejected because adapters would
  still duplicate field forms, validation, and native parsing.
- Replacing `MergedProject` with one normalized object graph was rejected because validation,
  exact rendering, unknown fields, operations, and syntax evidence still need the generic tree.
- Moving the interpretation into BoxFerry was rejected because Compose semantics belong in
  ComposeLens and should be reusable by independent consumers.
