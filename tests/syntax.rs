//! Public syntax behavior and malformed-input recovery.

use compose_lens::source::SourceId;
use compose_lens::syntax::{SyntaxDocument, YAML_UNCLOSED_FLOW_SEQUENCE};
use compose_lens::{
    loader::{DocumentInput, DocumentOrigin, LoadedProject},
    merge::{MergedScalar, MergedValue, merge_project},
};

const LOSSLESS_COMPOSE: &str = include_str!("../fixtures/syntax/lossless-compose/compose.yaml");
const MALFORMED_FLOW: &str = include_str!("../fixtures/syntax/malformed-flow/compose.yaml");
const COMMA_PLAIN_SCALAR: &str = include_str!("../fixtures/syntax/comma-plain-scalar/compose.yaml");

#[test]
fn preserves_compose_shaped_yaml_without_normalization() -> Result<(), Box<dyn std::error::Error>> {
    let source_id = SourceId::new(7);
    let parsed = SyntaxDocument::parse(source_id, LOSSLESS_COMPOSE)?;
    let document = parsed.document();

    assert!(parsed.is_valid(), "{:#?}", parsed.diagnostics());
    assert_eq!(document.source_id(), source_id);
    assert_eq!(document.source_span().range(), 0..LOSSLESS_COMPOSE.len());
    assert_eq!(document.text(document.source_span()), Some(LOSSLESS_COMPOSE));
    assert_eq!(document.document_count(), 1);
    assert_eq!(document.comment_count(), 2);
    assert_eq!(document.render_preserved(), LOSSLESS_COMPOSE);
    assert_eq!(LOSSLESS_COMPOSE.match_indices("duplicate:").count(), 2);
    assert!(LOSSLESS_COMPOSE.contains("&defaults"));
    assert!(LOSSLESS_COMPOSE.contains("*defaults"));
    assert!(LOSSLESS_COMPOSE.contains("x-podman"));
    assert!(LOSSLESS_COMPOSE.contains("\"false\""));
    assert!(LOSSLESS_COMPOSE.contains("${VALUE:-unchanged}"));
    Ok(())
}

#[test]
fn malformed_yaml_returns_a_spanned_diagnostic_and_a_document() -> Result<(), Box<dyn std::error::Error>> {
    let source_id = SourceId::new(11);
    let parsed = SyntaxDocument::parse(source_id, MALFORMED_FLOW)?;

    assert!(!parsed.is_valid());
    assert_eq!(parsed.document().render_preserved(), MALFORMED_FLOW);
    assert!(parsed.diagnostics().iter().any(|diagnostic| {
        diagnostic.code() == YAML_UNCLOSED_FLOW_SEQUENCE
            && diagnostic
                .labels()
                .iter()
                .any(|label| label.span().source_id() == source_id && label.span().end() <= MALFORMED_FLOW.len())
    }));
    Ok(())
}

#[test]
fn preserves_unicode_and_crlf_while_reporting_byte_locations() -> Result<(), Box<dyn std::error::Error>> {
    let source = "# Käfer\r\nservices:\r\n  app:\r\n    image: demo\r\n";
    let parsed = SyntaxDocument::parse(SourceId::new(19), source)?;
    let document = parsed.document();
    let services_offset = source.find("services:");

    assert!(parsed.is_valid(), "{:#?}", parsed.diagnostics());
    assert_eq!(document.render_preserved(), source);
    assert_eq!(
        services_offset
            .and_then(|offset| document.line_column(offset))
            .map(|position| (position.line(), position.column())),
        Some((2, 1))
    );
    Ok(())
}

#[test]
fn accepts_and_preserves_a_comma_in_a_block_plain_scalar() -> Result<(), Box<dyn std::error::Error>> {
    let parsed = SyntaxDocument::parse(SourceId::new(23), COMMA_PLAIN_SCALAR)?;

    assert!(parsed.is_valid(), "{:#?}", parsed.diagnostics());
    assert_eq!(parsed.document().source_text(), COMMA_PLAIN_SCALAR);
    assert_eq!(parsed.document().render_preserved(), COMMA_PLAIN_SCALAR);
    assert_eq!(parsed.document().document_count(), 1);
    Ok(())
}

#[test]
fn restores_authored_commas_before_semantic_merge_processing() -> Result<(), Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(24),
        DocumentOrigin::new("compose.yaml", "workspace/project"),
        COMMA_PLAIN_SCALAR,
    )])?;
    let merged = merge_project(&loaded, None);
    let value = merged
        .project()
        .and_then(|project| project.value(&["x-parser-guards", "plain-mapping"]))
        .and_then(MergedValue::as_scalar)
        .map(MergedScalar::value);

    assert!(loaded.is_valid(), "{:#?}", loaded.diagnostics());
    assert!(merged.is_valid(), "{:#?}", merged.diagnostics());
    assert_eq!(value, Some("alpha,beta"));
    Ok(())
}
