# Project structure

ComposeLens is one library crate with strong internal module boundaries. The crate foundation exists; entries marked `planned` are created with their first behavior and tests. Separate published crates are not justified until an independent consumer and release need are demonstrated.

```text
compose-lens/
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
├── LICENSE
├── src/
│   ├── lib.rs
│   ├── source/             # source identifiers, spans, and line/column lookup
│   ├── syntax/             # loss-aware YAML document and parser diagnostics
│   ├── model/              # Phase 2 typed document, field variants, resources, and values
│   ├── loader/             # ordered caller inputs, origins, diagnostics, and project overlays
│   ├── merge/              # field-aware semantic composition and source provenance
│   ├── interpolation/      # explicit providers, operators, provenance, and redacted diagnostics
│   ├── profiles/           # explicit post-merge service profile selection
│   ├── resolution/         # host paths, cross-references, and default decisions
│   ├── validation/         # exact versions, compatibility profiles, evidence, and findings
│   ├── render/             # canonical rendering and atomic exact-span scalar editing
│   └── diagnostic/         # stable codes, severities, labels, and notes
├── tests/
│   ├── README.md           # suite ownership and introduction rules
│   ├── repository_policy.rs # fixture and workflow-pin enforcement
│   ├── syntax.rs           # preservation and malformed-input behavior
│   ├── roundtrip.rs        # parse/render/parse stability
│   ├── typed_model.rs      # typed structure, short/long forms, and diagnostics
│   ├── processing.rs       # interpolation, loading, merge, and later project operations
│   ├── rendering.rs        # canonical/custom output, stability, recovery, and redaction
│   ├── preservation_editing.rs # atomic scalar edits and byte-preservation failures
│   └── support/            # private repository-test helpers
├── fixtures/
│   ├── README.md           # fixture location and safety rules
│   ├── syntax/             # authored syntax and recovery cases
│   ├── roundtrip/          # authored stability cases
│   ├── typed-model/        # authored typed extraction and form-fidelity cases
│   └── processing/         # authored processing operators and recovery cases
├── docs/
│   ├── fixture-format.md   # versioned fixture manifest contract
│   ├── typed-model.md      # completed Phase 2 boundary and parse contract
│   └── research/           # versioned technical evaluations
└── .github/
    ├── renovate.json
    └── workflows/
        └── ci.yml
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
| Paths, references, and defaults         | `resolution`    |
| Implementation support and correctness | `validation`    |
| YAML output                            | `render`        |
| Stable diagnostic codes                | `diagnostic`    |

Do not place file access in syntax parsing, automatic interpolation in deserialization, or BoxFerry conversion types in the native model. Application adapters perform file access and pass explicit text and origins to the loader.
