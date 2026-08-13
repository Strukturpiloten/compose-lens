# Project structure

ComposeLens is one published library crate with strong internal module boundaries. Repository-only
conformance tooling remains an ignored integration test and is not part of the library API.
Separate published crates are not justified until an independent consumer and release need are
demonstrated.

```text
compose-lens/
├── .devcontainer/         # digest-pinned VS Code environment and feature lock
├── .vscode/               # shared editor settings, recommendations, and local tasks
├── scripts/               # complete local checks and non-Rust file-quality tooling
├── package.json           # pinned repository-only Node tooling
├── package-lock.json      # complete repository-tool dependency graph
├── lychee.toml            # offline/local and rate-limited external link policy
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── rustfmt.toml
├── clippy.toml
├── deny.toml
├── .cargo/
│   └── config.toml        # canonical Cargo aliases
├── AGENTS.md
├── README.md
├── CHANGELOG.md
├── LICENSE
├── src/
│   ├── lib.rs
│   ├── source/             # source identifiers, spans, and line/column lookup
│   ├── syntax/             # loss-aware YAML document and parser diagnostics
│   ├── model/              # native document, form-specific fields, resources, paths, and values
│   ├── loader/             # ordered caller inputs, origins, diagnostics, and project overlays
│   ├── merge/              # field-aware semantic composition and source provenance
│   ├── interpolation/      # explicit providers, operators, provenance, and redacted diagnostics
│   ├── profiles/           # explicit post-merge service profile selection
│   ├── project.rs          # native selected-project values with multi-file provenance
│   ├── resolution/         # host paths, cross-references, and default decisions
│   ├── validation/         # exact versions, compatibility profiles, evidence, and findings
│   ├── render/             # canonical/generated rendering and atomic exact-span scalar editing
│   └── diagnostic/         # stable codes, severities, labels, and notes
├── tests/
│   ├── README.md           # suite ownership and introduction rules
│   ├── repository_policy.rs # fixture and workflow-pin enforcement
│   ├── syntax.rs           # preservation and malformed-input behavior
│   ├── roundtrip.rs        # parse/render/parse stability
│   ├── typed_model.rs      # typed structure, short/long forms, and diagnostics
│   ├── processing.rs       # interpolation, loading, merge, and later project operations
│   ├── project_view.rs     # native merged/profile-selected values and provenance
│   ├── rendering.rs        # canonical/custom output, stability, recovery, and redaction
│   ├── generated_rendering.rs # typed new-document construction, exact bytes, and parse-back validation
│   ├── preservation_editing.rs # atomic scalar edits and byte-preservation failures
│   ├── compatibility.rs    # exact provider/runtime compatibility behavior
│   ├── conformance.rs      # provider matrix policy and ignored capture runner
│   ├── runtime_conformance.rs # fail-closed planned runtime matrix policy
│   ├── real_world.rs       # licensed project regressions
│   ├── public_api.rs       # supported 0.1.x consumer contract
│   └── support/            # private repository-test helpers
├── fixtures/
│   ├── README.md           # fixture location and safety rules
│   ├── syntax/             # authored syntax and recovery cases
│   ├── roundtrip/          # authored stability cases
│   ├── typed-model/        # authored typed extraction and form-fidelity cases
│   ├── processing/         # authored processing operators and recovery cases
│   ├── conformance/        # authored questions for exact external providers
│   └── real-world/         # licensed external project corpus
├── conformance/
│   ├── provider-config-matrix.toml # exact provider/probe selections and evidence state
│   ├── runtime-effect-matrix.toml # exact planned provider/runtime/privilege contexts
│   └── records/            # reviewed retained external observations
├── docs/
│   ├── fixture-format.md   # versioned fixture manifest contract
│   ├── typed-model.md      # completed Phase 2 boundary and parse contract
│   ├── releases/           # release notes and credentialed publication checklist
│   └── research/           # versioned technical evaluations
└── .github/
    ├── renovate.json
    └── workflows/
        ├── ci.yml
        ├── documentation-links.yml
        └── release.yml
```

## Module placement rules

| Concern                                | Module          |
| -------------------------------------- | --------------- |
| Text positions and source identity     | `source`        |
| YAML structure and spelling            | `syntax`        |
| Compose-native types                   | `model`         |
| Caller-supplied documents and origins  | `loader`        |
| Overlay semantics                      | `merge`         |
| Variable expressions and substitution  | `interpolation` |
| Compose service profiles               | `profiles`      |
| Native merged-project consumer values  | `project`       |
| Paths, references, and defaults        | `resolution`    |
| Implementation support and correctness | `validation`    |
| YAML output and new-document builders  | `render`        |
| Stable diagnostic codes                | `diagnostic`    |

Do not place file access in syntax parsing, automatic interpolation in deserialization, or BoxFerry conversion types in the native model. Application adapters perform file access and pass explicit text and origins to the loader.
