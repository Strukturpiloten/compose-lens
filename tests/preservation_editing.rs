//! Atomic preservation-oriented editing over typed value-scalar spans.

use compose_lens::model::{Command, ComposeDocument, Environment};
use compose_lens::render::{
    EDIT_INVALID_NUMBER, EDIT_OVERLAP, EDIT_SOURCE_MISMATCH, EDIT_TARGET_NOT_SCALAR, EDIT_UNSUPPORTED_SCALAR_STYLE,
    ReplacementScalar, ScalarEdit, apply_preservation_edits,
};
use compose_lens::source::{SourceId, SourceSpan};
use compose_lens::syntax::SyntaxDocument;

const INPUT: &str = include_str!("../fixtures/roundtrip/preservation-edits/compose.yaml");
const EXPECTED: &str = include_str!("../fixtures/roundtrip/preservation-edits/expected.yaml");

#[test]
fn edits_exact_scalars_and_preserves_every_unrelated_byte() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(SourceId::new(301), INPUT)?;
    let typed = ComposeDocument::parse(syntax.document());
    let document = typed.document().ok_or("typed document expected")?;
    let app = document.service("app").ok_or("app service expected")?;
    let image = app.image().ok_or("image expected")?;
    let Command::List { values: command, .. } = app.command().ok_or("command expected")? else {
        return Err("command list expected".into());
    };
    let Some(Environment::Map {
        entries: environment, ..
    }) = app.environment()
    else {
        return Err("environment mapping expected".into());
    };

    assert!(syntax.is_valid(), "{:#?}", syntax.diagnostics());
    assert!(typed.is_valid(), "{:#?}", typed.diagnostics());
    let edits = [
        ScalarEdit::new(
            image.span(),
            ReplacementScalar::string("registry.example/app:2.0@sha256:abc"),
        ),
        ScalarEdit::new(command[1].span(), ReplacementScalar::string("--port=8080")),
        ScalarEdit::new(
            environment_span(environment, "PLAIN")?,
            ReplacementScalar::string("contains # marker"),
        ),
        ScalarEdit::new(
            environment_span(environment, "SINGLE")?,
            ReplacementScalar::string("Martin's value"),
        ),
        ScalarEdit::new(
            environment_span(environment, "BOOLEAN")?,
            ReplacementScalar::boolean(false),
        ),
        ScalarEdit::new(
            environment_span(environment, "NUMBER")?,
            ReplacementScalar::number("2.5"),
        ),
        ScalarEdit::new(environment_span(environment, "NULLABLE")?, ReplacementScalar::null()),
    ];
    let edited = apply_preservation_edits(syntax.document(), &edits);

    assert!(edited.is_valid(), "{:#?}", edited.diagnostics());
    assert!(!edited.is_sensitive());
    assert_eq!(edited.output(), EXPECTED);
    let reparsed = SyntaxDocument::parse(SourceId::new(302), edited.output())?;
    let retyped = ComposeDocument::parse(reparsed.document());
    assert!(reparsed.is_valid(), "{:#?}", reparsed.diagnostics());
    assert!(retyped.is_valid(), "{:#?}", retyped.diagnostics());
    Ok(())
}

#[test]
fn rejects_invalid_targets_and_overlaps_atomically() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(SourceId::new(311), INPUT)?;
    let typed = ComposeDocument::parse(syntax.document());
    let image_span = typed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::image)
        .ok_or("image expected")?
        .span();
    let foreign_span =
        SourceSpan::new(SourceId::new(999), image_span.start(), image_span.end()).ok_or("foreign span expected")?;
    let key_start = INPUT.find("name:").ok_or("name key expected")?;
    let key_span =
        SourceSpan::new(SourceId::new(311), key_start, key_start + "name".len()).ok_or("key span expected")?;
    let edits = [
        ScalarEdit::new(foreign_span, ReplacementScalar::string("foreign")),
        ScalarEdit::new(key_span, ReplacementScalar::string("renamed")),
        ScalarEdit::new(image_span, ReplacementScalar::string("first")),
        ScalarEdit::new(image_span, ReplacementScalar::string("second")),
    ];
    let edited = apply_preservation_edits(syntax.document(), &edits);

    assert!(!edited.is_valid());
    assert!(!edited.is_sensitive());
    assert_eq!(edited.output(), INPUT);
    for code in [EDIT_SOURCE_MISMATCH, EDIT_TARGET_NOT_SCALAR, EDIT_OVERLAP] {
        assert!(
            edited.diagnostics().iter().any(|diagnostic| diagnostic.code() == code),
            "missing diagnostic {code}"
        );
    }
    Ok(())
}

#[test]
fn rejects_block_scalars_and_invalid_numbers_atomically() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    environment:\n      NUMBER: 1\nconfigs:\n  generated:\n    content: |\n      line one\n      line two\n";
    let syntax = SyntaxDocument::parse(SourceId::new(321), source)?;
    let typed = ComposeDocument::parse(syntax.document());
    let document = typed.document().ok_or("typed document expected")?;
    let app = document.service("app").ok_or("app service expected")?;
    let Some(Environment::Map {
        entries: environment, ..
    }) = app.environment()
    else {
        return Err("environment mapping expected".into());
    };
    let content_span = document
        .configs()
        .iter()
        .find(|config| config.name().value() == "generated")
        .and_then(compose_lens::model::ConfigDefinition::content)
        .ok_or("config content expected")?
        .span();
    let edits = [
        ScalarEdit::new(
            environment_span(environment, "NUMBER")?,
            ReplacementScalar::number("not a number"),
        ),
        ScalarEdit::new(content_span, ReplacementScalar::string("replacement")),
    ];
    let edited = apply_preservation_edits(syntax.document(), &edits);

    assert!(!edited.is_valid());
    assert_eq!(edited.output(), source);
    for code in [EDIT_INVALID_NUMBER, EDIT_UNSUPPORTED_SCALAR_STYLE] {
        assert!(
            edited.diagnostics().iter().any(|diagnostic| diagnostic.code() == code),
            "missing diagnostic {code}"
        );
    }
    Ok(())
}

#[test]
fn redacts_sensitive_replacements_from_debug_output() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = SyntaxDocument::parse(SourceId::new(331), INPUT)?;
    let typed = ComposeDocument::parse(syntax.document());
    let image_span = typed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::image)
        .ok_or("image expected")?
        .span();
    let edit = ScalarEdit::new(image_span, ReplacementScalar::sensitive_string("private-edit-value"));
    let edit_debug = format!("{edit:?}");
    let edited = apply_preservation_edits(syntax.document(), std::slice::from_ref(&edit));

    assert!(edited.is_valid(), "{:#?}", edited.diagnostics());
    assert!(edited.is_sensitive());
    assert!(edited.output().contains("private-edit-value"));
    assert!(!edit_debug.contains("private-edit-value"));
    assert!(!format!("{edited:?}").contains("private-edit-value"));
    assert!(edited.diagnostics().iter().all(|diagnostic| {
        !diagnostic.message().contains("private-edit-value")
            && diagnostic
                .labels()
                .iter()
                .all(|label| !label.message().contains("private-edit-value"))
            && diagnostic
                .notes()
                .iter()
                .all(|note| !note.contains("private-edit-value"))
    }));
    Ok(())
}

fn environment_span(
    entries: &[compose_lens::model::EnvironmentMapEntry],
    name: &str,
) -> Result<SourceSpan, Box<dyn std::error::Error>> {
    entries
        .iter()
        .find(|entry| entry.name().value() == name)
        .map(|entry| entry.value().span())
        .ok_or_else(|| format!("environment entry {name} expected").into())
}
