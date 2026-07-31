# ADR 0003: preserve Compose syntax forms

- Status: accepted
- Date: 2026-07-31

## Context

Many Compose fields accept short and long syntax. Those forms are not merely two
serializations of one value:

- one form can expose options that the other cannot express;
- defaults can differ, including bind-source creation behavior;
- implementations can route the forms through different runtime APIs;
- an implementation may accept a field but fail to apply it at runtime;
- converting forms can therefore change observable behavior.

Service volume mounts demonstrate the problem. The Compose Specification documents
SELinux `z` and `Z` in short syntax and `bind.selinux` in long syntax. Docker's bind-mount
documentation also states that its Mount API cannot modify an SELinux label. A reported
Docker Compose 2.40.3 case showed short syntax relabeling while equivalent-looking long
syntax did not. The evidence and its limits are recorded in the
[syntax-fidelity research note](../research/compose-syntax-fidelity.md).

Treating both forms as an immediately normalized mount would lose information needed for
correct validation, conversion, and preservation-oriented rendering.

## Decision

ComposeLens retains the authored syntax variant in every typed field that has multiple
forms.

1. Each Compose field owns a field-specific enum such as `VolumeMount::Short` and
   `VolumeMount::Long`. There is no universal public `ShortOrLong<T>` that implies the
   variants are interchangeable.
2. Typed values retain source spans and the original syntax document remains the source of
   truth for preservation-oriented rendering.
3. Parsing does not automatically rewrite one form into another.
4. Semantic views may expose common facts, such as the requested SELinux relabel mode, but
   those views do not erase syntax provenance.
5. Canonical rendering preserves the variant by default. A form-changing rewrite must be
   an explicit transformation with compatibility diagnostics.
6. Specification acceptance and runtime support remain separate. Versioned Docker Compose,
   Podman Compose, and target-runtime profiles will decide whether a valid construct is
   reliable for a particular environment.
7. Unknown options and extension fields remain attached to their authored node rather than
   being discarded during typed extraction.

## Consequences

- BoxFerry can distinguish an exact mapping from a behavior-changing approximation.
- ComposeLens uses more sum types and cannot expose a single normalized structure for every
  multi-form field.
- Callers that only need shared semantics use explicit helper methods while retaining access
  to the original variant.
- Compatibility knowledge can evolve without changing the parser's representation of valid
  source.
- Tests must cover both syntax variants independently, including cases that look equivalent.

## Rejected alternatives

### Normalize every field while parsing

Rejected because it loses authored defaults, implementation routing, unknown options, and
the evidence needed to issue safe conversion diagnostics.

### Preserve only the raw YAML node

Rejected because callers still need useful typed access to common Compose concepts. The
typed and syntax layers are complementary.

### Add SELinux as a special-case flag outside the mount type

Rejected because the same asymmetry occurs in other Compose fields. Syntax provenance is a
general modeling rule, not a volume-only workaround.
