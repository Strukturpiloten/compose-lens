//! Public syntax behavior and malformed-input recovery.

use compose_lens::source::SourceId;
use compose_lens::syntax::{SyntaxDocument, YAML_UNCLOSED_FLOW_SEQUENCE};
use compose_lens::{
    loader::{DocumentInput, DocumentOrigin, LoadedProject},
    merge::{MergedScalar, MergedValue, merge_project},
    model::ComposeDocument,
};

const LOSSLESS_COMPOSE: &str = include_str!("../fixtures/syntax/lossless-compose/compose.yaml");
const MALFORMED_FLOW: &str = include_str!("../fixtures/syntax/malformed-flow/compose.yaml");
const COMMA_PLAIN_SCALAR: &str = include_str!("../fixtures/syntax/comma-plain-scalar/compose.yaml");
const BLOCK_SCALAR_QUOTES: &str = include_str!("../fixtures/syntax/block-scalar-quotes/compose.yaml");

#[test]
fn block_body_quotes_do_not_consume_sibling_services() -> Result<(), Box<dyn std::error::Error>> {
    let source_id = SourceId::new(173);
    let syntax = SyntaxDocument::parse(source_id, BLOCK_SCALAR_QUOTES)?;
    assert!(syntax.is_valid(), "{:?}", syntax.diagnostics());
    assert_eq!(syntax.document().render_preserved(), BLOCK_SCALAR_QUOTES);
    let parsed = ComposeDocument::parse(syntax.document());
    assert!(parsed.is_valid(), "{:?}", parsed.diagnostics());
    let native = parsed.document().ok_or("native document expected")?;
    assert!(native.service("mongodb").is_some());
    assert!(native.service("postgresql").is_some());

    let loaded = LoadedProject::load([DocumentInput::new(
        source_id,
        DocumentOrigin::new("compose.yaml", "workspace/project"),
        BLOCK_SCALAR_QUOTES,
    )])?;
    assert!(loaded.is_valid(), "{:?}", loaded.diagnostics());
    let merged = merge_project(&loaded, None);
    assert!(merged.is_valid(), "{:?}", merged.diagnostics());
    let project = merged.project().ok_or("merged project expected")?;
    for (path, expected) in [
        (["services", "mongodb", "image"].as_slice(), "mongo:8"),
        (["services", "postgresql", "image"].as_slice(), "postgres:18"),
        (["services", "mongodb", "healthcheck", "test"].as_slice(), "\"\n"),
    ] {
        let value = project
            .value(path)
            .ok_or_else(|| format!("merged value expected: {path:?}"))?;
        assert_eq!(value.as_scalar().map(MergedScalar::value), Some(expected), "{path:?}");
    }
    for (name, expected) in [
        ("mongodb", "MONGO_INITDB_DATABASE=left"),
        ("postgresql", "POSTGRES_DB=right"),
    ] {
        let environment = project
            .value(&["services", name, "environment"])
            .and_then(MergedValue::as_sequence)
            .ok_or("environment expected")?;
        assert_eq!(environment.len(), 1);
        assert_eq!(environment[0].as_scalar().map(MergedScalar::value), Some(expected));
    }
    Ok(())
}

#[test]
fn block_quotes_retain_literal_and_folded_semantics() -> Result<(), Box<dyn std::error::Error>> {
    for (header, body, expected) in [
        ("|", "      \"\n", "\"\n"),
        ("|-", "      'quoted'\n\n", "'quoted'"),
        ("|+", "      \"quoted\"\n\n", "\"quoted\"\n\n"),
        (">", "      \"first\n      second'\n", "\"first second'\n"),
        (">-", "      \"first\n\n      second'\n", "\"first\nsecond'"),
        (
            ">+",
            "      \"first\n        indented'\n      last\n\n",
            "\"first\n  indented'\nlast\n\n",
        ),
        ("|2-", "      \"é雪\n", "\"é雪"),
        (">2-", "      \"é雪\n      next'\n", "\"é雪 next'"),
        ("|+2", "      \"first\n\n", "\"first\n\n"),
        ("|2-", "        \"first\n      next'\n", "  \"first\nnext'"),
        ("|2- # header comment", "      \"\n", "\""),
        ("|2-", "        \n      first\n", "  \nfirst"),
        ("|2", "        \n", "  \n"),
        ("|2-", "        \n", "  "),
        ("|2+", "        \n\n", "  \n\n"),
        ("|2", "", ""),
        ("|2-", "", ""),
        ("|2+", "", ""),
        ("| # note -", "      \"\n", "\"\n"),
        ("> # note 8", "      \"first\n      second'\n", "\"first second'\n"),
        ("|2+ # note -", "      \"\n\n", "\"\n\n"),
        ("|2-", "      \u{a0}\n", "\u{a0}"),
        (">+", "      first\n\n", "first\n\n"),
        (
            ">-",
            "      first\n\n        more\n\n      last\n",
            "first\n\n  more\n\nlast",
        ),
        ("|", "\n      \"first\n", "\n\"first\n"),
    ] {
        for crlf in [false, true] {
            let source =
                format!("---\nservices:\n  app:\n    command: {header}\n{body}  sibling:\n    image: \"later\"\n");
            let source = if crlf { source.replace('\n', "\r\n") } else { source };
            let parsed = SyntaxDocument::parse(SourceId::new(174), source.clone())?;
            assert!(parsed.is_valid(), "{header} CRLF={crlf}: {:?}", parsed.diagnostics());
            assert_eq!(parsed.document().render_preserved(), source);
            let loaded = LoadedProject::load([DocumentInput::new(
                SourceId::new(174),
                DocumentOrigin::new("compose.yaml", "project"),
                source,
            )])?;
            let merged = merge_project(&loaded, None);
            assert!(merged.is_valid(), "{header} CRLF={crlf}: {:?}", merged.diagnostics());
            let project = merged.project().ok_or("merged project expected")?;
            let value = project
                .value(&["services", "app", "command"])
                .ok_or("command expected")?;
            assert_eq!(
                value.as_scalar().map(MergedScalar::value),
                Some(expected),
                "{header} CRLF={crlf}"
            );
            assert_eq!(
                project
                    .value(&["services", "sibling", "image"])
                    .and_then(MergedValue::as_scalar)
                    .map(MergedScalar::value),
                Some("later")
            );
        }
    }
    Ok(())
}

#[test]
fn explicit_block_indentation_retains_sequence_and_compact_mapping_values() -> Result<(), Box<dyn std::error::Error>> {
    for (source, mapping) in [
        ("---\nx-values:\n  - |2-\n    \"first\n  - later\n", false),
        ("---\nx-values:\n  - test: |2-\n      \"first\n  - later\n", true),
        ("---\nx-values:\n  -    test: |2-\n         \"first\n  - later\n", true),
    ] {
        let parsed = SyntaxDocument::parse(SourceId::new(175), source)?;
        assert!(parsed.is_valid(), "{:?}", parsed.diagnostics());
        assert_eq!(parsed.document().render_preserved(), source);
        let loaded = LoadedProject::load([DocumentInput::new(
            SourceId::new(175),
            DocumentOrigin::new("compose.yaml", "project"),
            source,
        )])?;
        let merged = merge_project(&loaded, None);
        let values = merged
            .project()
            .and_then(|project| project.value(&["x-values"]))
            .and_then(MergedValue::as_sequence)
            .ok_or("values expected")?;
        assert_eq!(values.len(), 2);
        let first = if mapping {
            values[0].get("test")
        } else {
            Some(&values[0])
        };
        assert_eq!(
            first.and_then(MergedValue::as_scalar).map(MergedScalar::value),
            Some("\"first")
        );
        assert_eq!(values[1].as_scalar().map(MergedScalar::value), Some("later"));
    }
    Ok(())
}

#[test]
fn malformed_block_headers_and_indentation_fail_closed() -> Result<(), Box<dyn std::error::Error>> {
    for source in [
        "---\nx-value: |0\n  \"\nx-later: later\n",
        "---\nx-value: |++\n  \"\nx-later: later\n",
        "---\nx-value: |22\n  \"\nx-later: later\n",
        "---\nx-value: |#missing-separation\n  \"\nx-later: later\n",
        "---\nx-value: |2\n \"\nx-later: later\n",
        "---\nx-value: |\n   \n  \"\nx-later: later\n",
        "---\nx-value: |\n    \"first\n   less-indented\nx-later: later\n",
        "---\nx-value: |2-\n \u{a0}\nx-later: later\n",
    ] {
        let parsed = SyntaxDocument::parse(SourceId::new(176), source)?;
        assert!(!parsed.is_valid(), "malformed block accepted: {source:?}");
        assert_eq!(parsed.document().render_preserved(), source);
        assert!(
            parsed
                .diagnostics()
                .iter()
                .all(|diagnostic| !format!("{diagnostic:?}").contains("missing-separation"))
        );
    }
    Ok(())
}

#[test]
fn block_scalar_sequence_properties_and_nested_sequences_retain_boundaries() -> Result<(), Box<dyn std::error::Error>> {
    for source in [
        "---\nx-values:\n  - &block |2-\n    \"first\n  - *block\n",
        "---\nx-values:\n  - !!str |2-\n    \"first\n  - later\n",
        "---\nx-values:\n  - - |2-\n      \"first\n    - later\n",
    ] {
        let parsed = SyntaxDocument::parse(SourceId::new(177), source)?;
        assert!(parsed.is_valid(), "{source:?}: {:?}", parsed.diagnostics());
        assert_eq!(parsed.document().render_preserved(), source);
        let loaded = LoadedProject::load([DocumentInput::new(
            SourceId::new(177),
            DocumentOrigin::new("compose.yaml", "project"),
            source,
        )])?;
        let merged = merge_project(&loaded, None);
        assert!(merged.is_valid(), "{:?}", merged.diagnostics());
        let mut values = merged
            .project()
            .and_then(|project| project.value(&["x-values"]))
            .and_then(MergedValue::as_sequence)
            .ok_or("values expected")?;
        if source.contains("- - ") {
            values = values[0].as_sequence().ok_or("nested sequence expected")?;
        }
        assert_eq!(values.len(), 2);
        let first = match values[0].kind() {
            compose_lens::merge::MergedValueKind::Tagged { value, .. } => value.as_scalar(),
            _ => values[0].as_scalar(),
        };
        let first = match first {
            Some(value) => value.value(),
            None => return Err("block scalar expected".into()),
        };
        assert_eq!(first, "\"first");
        let later = if source.contains("*block") { "\"first" } else { "later" };
        assert_eq!(values[1].as_scalar().map(MergedScalar::value), Some(later));
    }
    Ok(())
}

#[test]
fn block_scalar_empty_bodies_and_eof_breaks_follow_authored_bytes() -> Result<(), Box<dyn std::error::Error>> {
    for header in ["|", "|-", "|+", ">", ">-", ">+", "|2", "|2-", "|2+"] {
        for (body, expected) in [
            ("", ""),
            ("\n", if header.contains('+') { "\n" } else { "" }),
            ("  \n", if header.contains('+') { "\n" } else { "" }),
            (
                "    \n  \n",
                if header.contains('2') {
                    if header.contains('+') {
                        "  \n\n"
                    } else if header.contains('-') {
                        "  "
                    } else {
                        "  \n"
                    }
                } else if header.contains('+') {
                    "\n\n"
                } else {
                    ""
                },
            ),
            ("  \"", "\""),
            ("  \"\n", if header.contains('-') { "\"" } else { "\"\n" }),
        ] {
            for crlf in [false, true] {
                let source = format!("---\nx-value: {header}\n{body}");
                let source = if crlf { source.replace('\n', "\r\n") } else { source };
                let parsed = SyntaxDocument::parse(SourceId::new(178), source.clone())?;
                assert!(parsed.is_valid(), "{source:?}: {:?}", parsed.diagnostics());
                assert_eq!(parsed.document().render_preserved(), source);
                let loaded = LoadedProject::load([DocumentInput::new(
                    SourceId::new(178),
                    DocumentOrigin::new("compose.yaml", "project"),
                    source,
                )])?;
                let merged = merge_project(&loaded, None);
                let value = merged
                    .project()
                    .and_then(|project| project.value(&["x-value"]))
                    .and_then(MergedValue::as_scalar)
                    .ok_or("scalar expected")?;
                assert_eq!(value.value(), expected, "{header} body={body:?} CRLF={crlf}");
            }
        }
    }
    Ok(())
}

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
fn preserves_top_level_anchored_blocks_with_hyphenated_names() -> Result<(), Box<dyn std::error::Error>> {
    let source = "x-user: &superset-user root\nx-volumes: &superset-volumes\n  # shared mounts\n  - ./data:/data\nx-build: &common-build\n  context: .\n  target: dev\n  args:\n    DEVELOPMENT: \"true\"\nservices:\n  app:\n    image: example\n    user: *superset-user\n    volumes: *superset-volumes\n    build: *common-build\n";
    let source_id = SourceId::new(8);
    let parsed = SyntaxDocument::parse(source_id, source)?;

    assert!(parsed.is_valid(), "{:#?}", parsed.diagnostics());
    assert_eq!(parsed.document().render_preserved(), source);

    let loaded = LoadedProject::load([DocumentInput::new(
        source_id,
        DocumentOrigin::new("compose.yaml", "workspace/project"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let project = merged.project().ok_or("merged project expected")?;
    let user = project
        .value(&["services", "app", "user"])
        .and_then(MergedValue::as_scalar)
        .map(MergedScalar::value);
    let volume = project
        .value(&["services", "app", "volumes"])
        .and_then(MergedValue::as_sequence)
        .and_then(|values| values.first())
        .and_then(MergedValue::as_scalar)
        .map(MergedScalar::value);
    let build_context = project
        .value(&["services", "app", "build", "context"])
        .and_then(MergedValue::as_scalar)
        .map(MergedScalar::value);

    assert!(loaded.is_valid(), "{:#?}", loaded.diagnostics());
    assert!(merged.is_valid(), "{:#?}", merged.diagnostics());
    assert_eq!(user, Some("root"));
    assert_eq!(volume, Some("./data:/data"));
    assert_eq!(build_context, Some("."));
    Ok(())
}

#[test]
fn preserves_unquoted_double_dash_sequence_items() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n  app:\n    image: example\n    command:\n      - --connect\n      - --constraints=Label(`stack`,`example`)\n";
    let source_id = SourceId::new(9);
    let parsed = SyntaxDocument::parse(source_id, source)?;
    let loaded = LoadedProject::load([DocumentInput::new(
        source_id,
        DocumentOrigin::new("compose.yaml", "workspace/project"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let command = merged
        .project()
        .and_then(|project| project.value(&["services", "app", "command"]))
        .and_then(MergedValue::as_sequence)
        .ok_or("command sequence expected")?
        .iter()
        .filter_map(MergedValue::as_scalar)
        .map(MergedScalar::value)
        .collect::<Vec<_>>();

    assert!(parsed.is_valid(), "{:#?}", parsed.diagnostics());
    assert_eq!(parsed.document().render_preserved(), source);
    assert!(loaded.is_valid(), "{:#?}", loaded.diagnostics());
    assert!(merged.is_valid(), "{:#?}", merged.diagnostics());
    assert_eq!(command, ["--connect", "--constraints=Label(`stack`,`example`)"]);
    Ok(())
}

#[test]
fn accepts_a_blank_line_before_an_indented_mapping_value() -> Result<(), Box<dyn std::error::Error>> {
    let source = "services:\n\n    app:\n      image: example\n";
    let source_id = SourceId::new(10);
    let parsed = SyntaxDocument::parse(source_id, source)?;
    let loaded = LoadedProject::load([DocumentInput::new(
        source_id,
        DocumentOrigin::new("compose.yaml", "workspace/project"),
        source,
    )])?;

    assert!(parsed.is_valid(), "{:#?}", parsed.diagnostics());
    assert_eq!(parsed.document().render_preserved(), source);
    assert!(loaded.is_valid(), "{:#?}", loaded.diagnostics());
    assert!(
        loaded
            .documents()
            .first()
            .and_then(|document| document.model().document())
            .and_then(|document| document.service("app"))
            .is_some()
    );
    Ok(())
}

#[test]
fn preserves_unquoted_required_interpolation_as_a_typed_scalar() -> Result<(), Box<dyn std::error::Error>> {
    let source = "---\nservices:\n  app:\n    image: ${IMAGE:?}\n";
    let source_id = SourceId::new(25);
    let parsed = SyntaxDocument::parse(source_id, source)?;
    let typed = ComposeDocument::parse(parsed.document());
    let image = typed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::image)
        .ok_or("image expected")?;
    let expected = "${IMAGE:?}";
    let start = source.find(expected).ok_or("image offset expected")?;

    assert!(parsed.is_valid(), "{:#?}", parsed.diagnostics());
    assert!(typed.is_valid(), "{:#?}", typed.diagnostics());
    assert_eq!(image.value().raw(), expected);
    assert_eq!(image.span().source_id(), source_id);
    assert_eq!(image.span().range(), start..start + expected.len());
    assert_eq!(parsed.document().text(image.span()), Some(expected));
    assert_eq!(parsed.document().render_preserved(), source);
    Ok(())
}

#[test]
fn preserves_embedded_nested_interpolation_and_trailing_scalar_text() -> Result<(), Box<dyn std::error::Error>> {
    let source = "---\nservices:\n  app:\n    image: registry.invalid/app:${TAG:-${FALLBACK:?}}-debug\n";
    let source_id = SourceId::new(26);
    let parsed = SyntaxDocument::parse(source_id, source)?;
    let typed = ComposeDocument::parse(parsed.document());
    let image = typed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::image)
        .ok_or("image expected")?;
    let expected = "registry.invalid/app:${TAG:-${FALLBACK:?}}-debug";
    let start = source.find(expected).ok_or("image offset expected")?;

    assert!(parsed.is_valid(), "{:#?}", parsed.diagnostics());
    assert!(typed.is_valid(), "{:#?}", typed.diagnostics());
    assert_eq!(image.value().raw(), expected);
    assert_eq!(image.span().source_id(), source_id);
    assert_eq!(image.span().range(), start..start + expected.len());
    assert_eq!(parsed.document().text(image.span()), Some(expected));
    assert_eq!(parsed.document().render_preserved(), source);
    Ok(())
}

#[test]
fn preserves_unbalanced_interpolation_closer_without_truncating_authored_bytes()
-> Result<(), Box<dyn std::error::Error>> {
    let source = "---\nservices:\n  app:\n    image: registry.invalid/app:${TAG:?}}\n    command: [still, present]\n  later:\n    image: registry.invalid/later:1\n";
    let source_id = SourceId::new(27);
    let parsed = SyntaxDocument::parse(source_id, source)?;
    let typed = ComposeDocument::parse(parsed.document());
    let image = typed
        .document()
        .and_then(|document| document.service("app"))
        .and_then(compose_lens::model::Service::image)
        .ok_or("image expected")?;
    let expected = "registry.invalid/app:${TAG:?}}";
    let start = source.find(expected).ok_or("image offset expected")?;

    assert!(parsed.is_valid(), "{:#?}", parsed.diagnostics());
    assert!(typed.is_valid(), "{:#?}", typed.diagnostics());
    assert_eq!(image.value().raw(), expected);
    assert_eq!(image.span().source_id(), source_id);
    assert_eq!(image.span().range(), start..start + expected.len());
    assert_eq!(parsed.document().text(image.span()), Some(expected));
    assert_eq!(
        typed
            .document()
            .and_then(|document| document.service("later"))
            .and_then(compose_lens::model::Service::image)
            .map(|image| image.value().raw()),
        Some("registry.invalid/later:1")
    );
    assert_eq!(parsed.document().source_span().range(), 0..source.len());
    assert_eq!(parsed.document().source_text(), source);
    assert_eq!(parsed.document().render_preserved(), source);
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
