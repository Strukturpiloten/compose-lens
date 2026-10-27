//! Unevaluated native interpolation grammar, ranges, eligibility, limits, and protection.
use compose_lens::{
    interpolation::{
        INSPECTION_LIMIT, INTERPOLATION_INSPECTION_MAX_BYTES, INTERPOLATION_INSPECTION_MAX_DEPTH,
        INTERPOLATION_INSPECTION_MAX_SEGMENTS, INVALID_EXPRESSION, InterpolationInput, InterpolationInspection,
        InterpolationInspectionErrorKind, InterpolationIssueKind, InterpolationOperator, InterpolationReferenceForm,
        InterpolationSegment, MapEnvironment, ScalarInterpolationInspection, inspect_interpolation,
        inspect_scalar_interpolation,
    },
    loader::{DocumentInput, DocumentOrigin, LoadedProject},
    merge::merge_project,
    source::{SourceId, SourceSpan},
    syntax::ScalarPresentation,
};
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn span() -> Result<SourceSpan, Box<dyn std::error::Error>> {
    SourceSpan::new(SourceId::new(189), 400, 500).ok_or_else(|| "containing source span".into())
}
fn inspect(value: &str) -> Result<InterpolationInspection, Box<dyn std::error::Error>> {
    Ok(inspect_interpolation(InterpolationInput::new(value, span()?))?)
}
fn literals(result: &InterpolationInspection) -> Result<String, Box<dyn std::error::Error>> {
    let mut output = String::new();
    for segment in result.segments() {
        let InterpolationSegment::Literal(literal) = segment else {
            return Err("expected literal-only result".into());
        };
        output.push_str(literal.value());
    }
    Ok(output)
}
#[test]
fn direct_references_are_unevaluated_and_ranges_are_decoded_utf8_offsets() -> TestResult {
    let input = "ä-$HOME/${USER}-雪";
    let result = inspect(input)?;
    assert!(result.is_valid());
    assert!(result.diagnostics().is_empty());
    assert_eq!(result.original(), input);
    assert_eq!(result.range(), 0..20);
    assert_eq!(result.span(), span()?);
    assert!(!result.is_sensitive());
    assert_eq!(
        result
            .segments()
            .iter()
            .map(InterpolationSegment::range)
            .collect::<Vec<_>>(),
        [0..3, 3..8, 8..9, 9..16, 16..20]
    );
    for segment in result.segments() {
        assert_eq!(segment.original(), &input[segment.range()]);
        assert_eq!(segment.span(), span()?);
        assert!(!segment.is_sensitive());
    }
    for (index, name, form, name_range) in [
        (1, "HOME", InterpolationReferenceForm::Unbraced, 4..8),
        (3, "USER", InterpolationReferenceForm::Braced, 11..15),
    ] {
        let InterpolationSegment::Reference(reference) = &result.segments()[index] else {
            return Err("reference".into());
        };
        assert_eq!(reference.name(), name);
        assert_eq!(reference.form(), form);
        assert_eq!(reference.name_range(), name_range);
        assert_eq!(reference.operator(), InterpolationOperator::Direct);
        assert_eq!(reference.operator_range(), None);
        assert_eq!(reference.operand(), None);
        assert_eq!(reference.operand_range(), None);
        assert_eq!(reference.operand_segments(), None);
        assert_eq!(reference.span(), span()?);
    }
    Ok(())
}
#[test]
fn all_native_operators_and_nested_operands_keep_syntax_without_branch_selection() -> TestResult {
    for (spelling, operator) in [
        (":-", InterpolationOperator::DefaultIfUnsetOrEmpty),
        ("-", InterpolationOperator::DefaultIfUnset),
        (":?", InterpolationOperator::RequiredIfUnsetOrEmpty),
        ("?", InterpolationOperator::RequiredIfUnset),
        (":+", InterpolationOperator::AlternativeIfSetAndNonEmpty),
        ("+", InterpolationOperator::AlternativeIfSet),
    ] {
        let text = format!("${{MISSING{spelling}operand-$OTHER}}");
        let result = inspect(&text)?;
        assert!(result.is_valid());
        assert!(result.diagnostics().is_empty());
        let InterpolationSegment::Reference(reference) = &result.segments()[0] else {
            return Err("operator reference".into());
        };
        assert_eq!(reference.operator(), operator);
        assert_eq!(reference.operand(), Some("operand-$OTHER"));
        assert_eq!(&text[reference.operator_range().ok_or("operator range")?], spelling);
        assert_eq!(reference.operand_segments().ok_or("operand syntax")?.len(), 2);
    }
    let text = "λ${A:-pre${B}post}";
    let result = inspect(text)?;
    let InterpolationSegment::Reference(reference) = &result.segments()[1] else {
        return Err("outer reference".into());
    };
    assert_eq!(reference.range(), 2..19);
    assert_eq!(reference.operator_range(), Some(5..7));
    assert_eq!(reference.operand_range(), Some(7..18));
    let operand = reference.operand_segments().ok_or("operand")?;
    assert_eq!(
        operand.iter().map(InterpolationSegment::range).collect::<Vec<_>>(),
        [7..10, 10..14, 14..18]
    );
    let InterpolationSegment::Reference(nested) = &operand[1] else {
        return Err("nested reference".into());
    };
    assert_eq!(nested.name(), "B");
    assert_eq!(nested.name_range(), 12..13);
    for segment in operand {
        assert_eq!(segment.original(), &text[segment.range()]);
    }
    let empty_operand = inspect("${A:-}")?;
    let InterpolationSegment::Reference(reference) = &empty_operand.segments()[0] else {
        return Err("empty operand reference".into());
    };
    assert_eq!(reference.operand(), Some(""));
    assert_eq!(reference.operand_segments(), Some([].as_slice()));
    Ok(())
}
#[test]
fn dollar_escapes_collapse_once_and_never_create_new_references() -> TestResult {
    for (input, expected) in [
        ("", ""),
        ("plain", "plain"),
        ("$", "$"),
        ("$9", "$9"),
        ("$- $雪", "$- $雪"),
        ("$$", "$"),
        ("$$$$", "$$"),
        ("$$HOME", "$HOME"),
        ("$${HOME}", "${HOME}"),
        ("a$$b$$", "a$b$"),
        ("$${NOT:SUPPORTED}", "${NOT:SUPPORTED}"),
    ] {
        let result = inspect(input)?;
        assert!(result.is_valid(), "{input:?}");
        assert_eq!(literals(&result)?, expected);
        assert_eq!(result.original(), input);
    }
    let result = inspect("$$$HOME")?;
    assert_eq!(result.segments().len(), 2);
    let InterpolationSegment::Literal(literal) = &result.segments()[0] else {
        return Err("escaped literal".into());
    };
    assert_eq!(literal.original(), "$$");
    assert_eq!(literal.value(), "$");
    let InterpolationSegment::Reference(reference) = &result.segments()[1] else {
        return Err("remaining real reference".into());
    };
    assert_eq!(reference.original(), "$HOME");
    let nested_escape = inspect("${A:-$${B}}")?;
    let InterpolationSegment::Reference(reference) = &nested_escape.segments()[0] else {
        return Err("outer".into());
    };
    let operand = reference.operand_segments().ok_or("escaped operand")?;
    assert_eq!(operand.len(), 1);
    let InterpolationSegment::Literal(literal) = &operand[0] else {
        return Err("escaped operand must stay literal".into());
    };
    assert_eq!(literal.value(), "${B}");
    Ok(())
}
#[test]
fn malformed_and_unsupported_braced_syntax_remains_typed_and_source_addressable() -> TestResult {
    for (input, issue) in [
        ("${A", InterpolationIssueKind::UnclosedBracedExpression),
        ("${}", InterpolationIssueKind::InvalidVariableName),
        ("${1A}", InterpolationIssueKind::InvalidVariableName),
        ("${A/foo}", InterpolationIssueKind::UnsupportedOperator),
        ("${A:}", InterpolationIssueKind::UnsupportedOperator),
    ] {
        let result = inspect(input)?;
        assert!(!result.is_valid());
        assert_eq!(result.diagnostics().len(), 1);
        assert_eq!(result.diagnostics()[0].code(), INVALID_EXPRESSION);
        let InterpolationSegment::Invalid(invalid) = &result.segments()[0] else {
            return Err("invalid segment".into());
        };
        assert_eq!(invalid.kind(), issue);
        assert_eq!(invalid.original(), input);
        assert_eq!(invalid.range(), 0..input.len());
        assert_eq!(invalid.span(), span()?);
        assert!(!invalid.is_sensitive());
    }
    let result = inspect("前${A/b}後$END")?;
    assert_eq!(
        result
            .segments()
            .iter()
            .map(InterpolationSegment::range)
            .collect::<Vec<_>>(),
        [0..3, 3..9, 9..12, 12..16]
    );
    assert_eq!(result.diagnostics()[0].labels()[0].span(), span()?);
    let nested_invalid = inspect("${A:-before${B/foo}after}")?;
    assert!(!nested_invalid.is_valid());
    let InterpolationSegment::Reference(reference) = &nested_invalid.segments()[0] else {
        return Err("outer remains reference".into());
    };
    assert!(matches!(
        reference.operand_segments().ok_or("operand")?[1],
        InterpolationSegment::Invalid(_)
    ));
    Ok(())
}
#[test]
fn native_style_exclusions_are_explicit_and_leave_all_decoded_dollars_unchanged() -> TestResult {
    for presentation in [
        ScalarPresentation::SingleQuoted,
        ScalarPresentation::Literal,
        ScalarPresentation::Folded,
    ] {
        for text in ["$NAME $$ $${OTHER}", "ordinary", ""] {
            let result = inspect_scalar_interpolation(InterpolationInput::new(text, span()?), presentation)?;
            let ScalarInterpolationInspection::Excluded(excluded) = result else {
                return Err("excluded presentation".into());
            };
            assert_eq!(excluded.original(), text);
            assert_eq!(excluded.presentation(), presentation);
            assert_eq!(excluded.contains_dollar(), text.contains('$'));
            assert_eq!(excluded.span(), span()?);
            assert_eq!(excluded.range(), 0..text.len());
            assert!(!excluded.is_sensitive());
        }
    }
    for presentation in [ScalarPresentation::Plain, ScalarPresentation::DoubleQuoted] {
        let ScalarInterpolationInspection::Inspected(result) =
            inspect_scalar_interpolation(InterpolationInput::new("$$HOME $NAME", span()?), presentation)?
        else {
            return Err("eligible presentation".into());
        };
        assert_eq!(result.segments().len(), 2);
    }
    Ok(())
}
#[test]
fn protected_names_literals_operands_invalid_syntax_and_excluded_results_redact_individually() -> TestResult {
    let input = InterpolationInput::new("literal-secret$$${SECRET_NAME:-operand-secret$NESTED}", span()?).sensitive();
    let result = inspect_interpolation(input)?;
    assert!(result.is_sensitive());
    let mut debug = vec![
        format!("{input:?}"),
        format!("{result:?}"),
        format!("{:?}", result.clone()),
    ];
    for segment in result.segments() {
        assert!(segment.is_sensitive());
        debug.push(format!("{segment:?}"));
        match segment {
            InterpolationSegment::Literal(literal) => {
                assert!(literal.is_sensitive());
                debug.push(format!("{literal:?}"));
            }
            InterpolationSegment::Reference(reference) => {
                assert!(reference.is_sensitive());
                debug.push(format!("{reference:?}"));
                for segment in reference.operand_segments().ok_or("operand")? {
                    assert!(segment.is_sensitive());
                    debug.push(format!("{segment:?}"));
                }
            }
            _ => return Err("unexpected protected segment".into()),
        }
    }
    let invalid = inspect_interpolation(InterpolationInput::new("${SECRET_NAME:operand-secret}", span()?).sensitive())?;
    let InterpolationSegment::Invalid(invalid_segment) = &invalid.segments()[0] else {
        return Err("protected invalid".into());
    };
    assert!(invalid_segment.is_sensitive());
    debug.extend([
        format!("{invalid:?}"),
        format!("{invalid_segment:?}"),
        format!("{:?}", invalid.diagnostics()),
    ]);
    let excluded = inspect_scalar_interpolation(input, ScalarPresentation::SingleQuoted)?;
    let ScalarInterpolationInspection::Excluded(excluded_value) = &excluded else {
        return Err("protected exclusion".into());
    };
    assert!(excluded_value.is_sensitive());
    debug.extend([format!("{excluded:?}"), format!("{excluded_value:?}")]);
    for value in debug {
        for secret in ["literal-secret", "SECRET_NAME", "operand-secret", "NESTED"] {
            assert!(!value.contains(secret), "protected data leaked");
        }
    }
    Ok(())
}
#[test]
fn inspection_enforces_global_input_segment_and_operand_depth_bounds() -> TestResult {
    let maximum = "x".repeat(INTERPOLATION_INSPECTION_MAX_BYTES);
    assert!(inspect(&maximum)?.is_valid());
    let oversized = format!("{maximum}SECRET");
    let error = inspect_interpolation(InterpolationInput::new(&oversized, span()?).sensitive())
        .err()
        .ok_or("input bound")?;
    assert_eq!(error.kind(), InterpolationInspectionErrorKind::InputLimit);
    assert_eq!(error.range(), None);
    assert_eq!(error.span(), span()?);
    assert_eq!(error.diagnostic().code(), INSPECTION_LIMIT);
    assert!(!format!("{error:?} {error} {:?}", error.diagnostic()).contains("SECRET"));
    assert!(
        inspect_scalar_interpolation(
            InterpolationInput::new(&oversized, span()?),
            ScalarPresentation::Literal
        )
        .is_err()
    );
    let nodes = "$A".repeat(INTERPOLATION_INSPECTION_MAX_SEGMENTS);
    assert_eq!(inspect(&nodes)?.segments().len(), INTERPOLATION_INSPECTION_MAX_SEGMENTS);
    let error = inspect_interpolation(InterpolationInput::new(&format!("{nodes}$A"), span()?))
        .err()
        .ok_or("segment bound")?;
    assert_eq!(error.kind(), InterpolationInspectionErrorKind::SegmentLimit);
    assert_eq!(error.range(), Some(nodes.len()..nodes.len() + 2));
    let globally_excessive = format!("${{A:-{nodes}}}");
    assert_eq!(
        inspect_interpolation(InterpolationInput::new(&globally_excessive, span()?))
            .err()
            .ok_or("global operand segment count")?
            .kind(),
        InterpolationInspectionErrorKind::SegmentLimit
    );
    let nested = format!(
        "{}value{}",
        "${A:-".repeat(INTERPOLATION_INSPECTION_MAX_DEPTH),
        "}".repeat(INTERPOLATION_INSPECTION_MAX_DEPTH)
    );
    assert!(inspect(&nested)?.is_valid());
    let too_deep = format!("${{A:-{nested}}}");
    let error = inspect_interpolation(InterpolationInput::new(&too_deep, span()?))
        .err()
        .ok_or("depth bound")?;
    assert_eq!(error.kind(), InterpolationInspectionErrorKind::DepthLimit);
    assert!(error.range().is_some());
    Ok(())
}
#[test]
fn keyed_list_input_retains_original_style_latest_source_and_explicit_overlay_stage() -> TestResult {
    let base = "---\nservices:\n  app:\n    labels: [DEFER=baseline, ACTIVE=baseline]\n";
    let overlay = "---\nservices:\n  app:\n    labels: ['DEFER=$NAME', \"ACTIVE=$NAME\"]\n";
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(190),
            DocumentOrigin::new("base.yaml", "list-source"),
            base,
        ),
        DocumentInput::new(
            SourceId::new(191),
            DocumentOrigin::new("override.yaml", "list-source"),
            overlay,
        ),
    ])?;
    let merged = merge_project(&loaded, None);
    assert!(
        merged
            .project()
            .ok_or("project")?
            .root()
            .as_mapping()
            .ok_or("root mapping")?[0]
            .raw_list_item_input()
            .is_none()
    );
    let labels = merged
        .project()
        .ok_or("project")?
        .root()
        .get("services")
        .and_then(|value| value.get("app"))
        .and_then(|value| value.get("labels"))
        .and_then(|value| value.as_mapping())
        .ok_or("keyed list entries")?;
    for (key, original, presentation) in [
        ("DEFER", "DEFER=$NAME", ScalarPresentation::SingleQuoted),
        ("ACTIVE", "ACTIVE=$NAME", ScalarPresentation::DoubleQuoted),
    ] {
        let entry = labels.iter().find(|entry| entry.key() == key).ok_or("entry")?;
        let input = entry.raw_list_item_input().ok_or("complete item input")?;
        assert_eq!(input.value(), original);
        assert_eq!(input.span().source_id(), SourceId::new(191));
        assert!(overlay[input.span().range()].contains(original));
        assert_eq!(entry.raw_list_item().ok_or("raw item")?.presentation(), presentation);
        assert_eq!(
            entry.value().as_scalar().ok_or("synthetic suffix")?.presentation(),
            ScalarPresentation::Plain
        );
    }
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("NAME", "resolved-secret");
    let interpolation = loaded.interpolate(&environment);
    let resolved = merge_project(&loaded, Some(&interpolation));
    let entries = resolved
        .project()
        .ok_or("resolved project")?
        .root()
        .get("services")
        .and_then(|value| value.get("app"))
        .and_then(|value| value.get("labels"))
        .and_then(|value| value.as_mapping())
        .ok_or("resolved entries")?;
    let active = entries
        .iter()
        .find(|entry| entry.key() == "ACTIVE")
        .and_then(compose_lens::merge::MergedEntry::raw_list_item_input)
        .ok_or("resolved input")?;
    assert_eq!(active.value(), "ACTIVE=resolved-secret");
    assert!(active.is_sensitive());
    assert!(!format!("{active:?}").contains("resolved-secret"));
    assert_eq!(
        entries
            .iter()
            .find(|entry| entry.key() == "DEFER")
            .and_then(compose_lens::merge::MergedEntry::raw_list_item_input)
            .ok_or("excluded original input")?
            .value(),
        "DEFER=$NAME"
    );
    Ok(())
}

#[test]
fn decoded_ranges_remain_separate_from_raw_quoted_yaml_source_offsets() -> TestResult {
    let source = "---\nservices:\n  app:\n    environment:\n      VALUE: \"a\\n$NAME\"\n";
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(192),
        DocumentOrigin::new("compose.yaml", "decoded-ranges"),
        source,
    )])?;
    let merged = merge_project(&loaded, None);
    let value = merged
        .project()
        .ok_or("project")?
        .root()
        .get("services")
        .and_then(|value| value.get("app"))
        .and_then(|value| value.get("environment"))
        .and_then(|value| value.get("VALUE"))
        .ok_or("native scalar")?;
    let scalar = value.as_scalar().ok_or("scalar")?;
    let source_span = value.provenance().effective_source().ok_or("source span")?;
    assert_eq!(scalar.value(), "a\n$NAME");
    assert_eq!(&source[source_span.range()], "\"a\\n$NAME\"");
    let ScalarInterpolationInspection::Inspected(result) = inspect_scalar_interpolation(
        InterpolationInput::new(scalar.value(), source_span),
        scalar.presentation(),
    )?
    else {
        return Err("native double-quoted eligibility".into());
    };
    assert_eq!(result.range(), 0..7);
    assert_eq!(result.segments()[1].range(), 2..7);
    assert_eq!(result.segments()[1].span(), source_span);
    assert_eq!(result.segments()[1].original(), "$NAME");
    Ok(())
}
