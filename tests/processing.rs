//! Public Phase 3 processing behavior.

use compose_lens::diagnostic::{Diagnostic, Severity};
use compose_lens::interpolation::{
    EmptyEnvironment, EnvironmentProvider, EnvironmentValue, INVALID_EXPRESSION, InterpolationInput,
    InterpolationOptions, MapEnvironment, MissingVariablePolicy, NESTING_LIMIT, REQUIRED_VARIABLE, SubstitutionOutcome,
    UNSET_VARIABLE, interpolate, interpolate_document, interpolate_with_options,
};
use compose_lens::loader::{DocumentInput, DocumentOrigin, LoadedProject, ProjectLoadError};
use compose_lens::merge::{
    EntrySyntax, INTERPOLATION_PROJECT_MISMATCH, MergeOperation, MergedEntry, MergedProject, MergedScalar, MergedValue,
    merge_project,
};
use compose_lens::model::{DNS_EXPECTED_FORM, DNS_SEARCH_EXPECTED_FORM};
use compose_lens::source::{SourceId, SourceSpan};
use std::path::Path;

const OPERATOR_CASES: &str = include_str!("../fixtures/processing/interpolation-operators/cases.txt");
const DOCUMENT_INTERPOLATION: &str = include_str!("../fixtures/processing/document-interpolation/compose.yaml");
const MULTI_FILE_BASE: &str = include_str!("../fixtures/processing/multi-file-loading/compose.yaml");
const MULTI_FILE_OVERRIDE: &str = include_str!("../fixtures/processing/multi-file-loading/compose.override.yaml");
const MERGE_BASE: &str = include_str!("../fixtures/processing/multi-file-merge/compose.yaml");
const MERGE_OVERRIDE: &str = include_str!("../fixtures/processing/multi-file-merge/compose.override.yaml");
const TAG_BASE: &str = include_str!("../fixtures/processing/merge-tags/compose.yaml");
const TAG_OVERRIDE: &str = include_str!("../fixtures/processing/merge-tags/compose.override.yaml");

fn span(text: &str) -> Result<SourceSpan, Box<dyn std::error::Error>> {
    SourceSpan::new(SourceId::new(61), 0, text.len()).ok_or_else(|| "valid test span expected".into())
}

fn environment() -> MapEnvironment {
    let mut environment = MapEnvironment::new();
    let _ = environment.insert("SET", "value");
    let _ = environment.insert("EMPTY", "");
    let _ = environment.insert("FALLBACK", "fallback");
    environment
}

#[test]
fn merges_annotations_by_effective_key_after_per_file_value_interpolation() -> Result<(), Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(696),
            DocumentOrigin::new("compose.yaml", "workspace/base"),
            concat!(
                "services:\n",
                "  app:\n    annotations: [\"io.example.base=base\", \"io.example.same=base\"]\n",
                "  reset:\n    annotations: [\"io.example.old=old\"]\n",
            ),
        ),
        DocumentInput::new(
            SourceId::new(697),
            DocumentOrigin::new("compose.override.yaml", "workspace/override"),
            concat!(
                "services:\n",
                "  app:\n    annotations:\n",
                "      io.example.same: \"${ANNOTATION_SECRET}\"\n",
                "      io.example.number: 007\n",
                "  reset:\n    annotations: !reset {}\n",
            ),
        ),
    ])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("ANNOTATION_SECRET", "effective");
    let interpolation = loaded.interpolate(&environment);
    let result = merge_project(&loaded, Some(&interpolation));
    let project = result.project().ok_or("merged project expected")?;
    let annotations = project
        .value(&["services", "app", "annotations"])
        .and_then(MergedValue::as_mapping)
        .ok_or("keyed annotations mapping expected")?;
    assert_eq!(annotations.len(), 3);
    assert_eq!(annotations[0].key(), "io.example.base");
    assert_eq!(annotations[0].syntax(), EntrySyntax::ListKeyValue);
    assert!(annotations[0].raw_list_item().is_some());
    assert_eq!(annotations[1].key(), "io.example.same");
    assert_eq!(annotations[1].syntax(), EntrySyntax::Mapping);
    assert_eq!(annotations[1].key_sources().len(), 2);
    assert_eq!(
        annotations[1].value().as_scalar().map(MergedScalar::value),
        Some("effective")
    );
    assert!(annotations[1].value().is_sensitive());
    assert_eq!(annotations[2].value().as_scalar().map(MergedScalar::raw), Some("007"));

    let reset = project
        .value(&["services", "reset", "annotations"])
        .ok_or("reset annotations expected")?;
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    assert!(reset.as_mapping().is_some_and(<[MergedEntry]>::is_empty));
    Ok(())
}

#[test]
fn evaluates_the_authored_operator_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let environment = environment();
    for (line_index, line) in OPERATOR_CASES.lines().enumerate() {
        let (input, expected) = line
            .split_once(" => ")
            .ok_or_else(|| format!("fixture line {} has no separator", line_index + 1))?;
        let expected = if expected == "<empty>" { "" } else { expected };
        let result = interpolate(InterpolationInput::new(input, span(input)?), &environment);

        assert_eq!(result.resolved(), expected, "fixture line {}", line_index + 1);
        assert!(
            result.is_valid(),
            "fixture line {}: {:#?}",
            line_index + 1,
            result.diagnostics()
        );
        assert!(result.diagnostics().is_empty(), "fixture line {}", line_index + 1);
    }
    Ok(())
}

#[test]
fn applies_explicit_missing_and_required_variable_policies() -> Result<(), Box<dyn std::error::Error>> {
    let environment = EmptyEnvironment;
    let direct = "image:${MISSING}";
    let direct_result = interpolate(InterpolationInput::new(direct, span(direct)?), &environment);
    assert_eq!(direct_result.resolved(), "image:");
    assert!(direct_result.is_valid());
    assert_eq!(direct_result.diagnostics()[0].code(), UNSET_VARIABLE);
    assert_eq!(direct_result.diagnostics()[0].severity(), Severity::Warning);

    let required = "${REQUIRED:?caller supplied secret detail}";
    let required_result = interpolate(InterpolationInput::new(required, span(required)?), &environment);
    assert_eq!(required_result.resolved(), required);
    assert!(!required_result.is_valid());
    assert_eq!(required_result.diagnostics()[0].code(), REQUIRED_VARIABLE);
    assert!(!diagnostic_text(&required_result.diagnostics()[0]).contains("caller supplied secret detail"));

    let options = InterpolationOptions::new(MissingVariablePolicy::Error);
    let strict = interpolate_with_options(InterpolationInput::new(direct, span(direct)?), &environment, options);
    assert_eq!(strict.resolved(), direct);
    assert!(!strict.is_valid());
    assert_eq!(strict.diagnostics()[0].severity(), Severity::Error);

    let preserve_options = InterpolationOptions::new(MissingVariablePolicy::PreserveWithWarning);
    let preserved = interpolate_with_options(
        InterpolationInput::new(direct, span(direct)?),
        &environment,
        preserve_options,
    );
    assert_eq!(preserved.resolved(), direct);
    assert!(preserved.is_valid());
    assert_eq!(preserved.diagnostics()[0].severity(), Severity::Warning);
    Ok(())
}

#[test]
fn required_operators_distinguish_unset_from_empty() -> Result<(), Box<dyn std::error::Error>> {
    let environment = environment();
    let unset_only = "${EMPTY?redacted-message}";
    let unset_only_result = interpolate(InterpolationInput::new(unset_only, span(unset_only)?), &environment);
    assert_eq!(unset_only_result.resolved(), "");
    assert!(unset_only_result.is_valid());

    let unset_or_empty = "${EMPTY:?redacted-message}";
    let unset_or_empty_result = interpolate(
        InterpolationInput::new(unset_or_empty, span(unset_or_empty)?),
        &environment,
    );
    assert_eq!(unset_or_empty_result.resolved(), unset_or_empty);
    assert!(!unset_or_empty_result.is_valid());
    assert_eq!(unset_or_empty_result.diagnostics()[0].code(), REQUIRED_VARIABLE);
    assert!(!diagnostic_text(&unset_or_empty_result.diagnostics()[0]).contains("redacted-message"));
    Ok(())
}

#[test]
fn propagates_sensitivity_only_when_sensitive_content_is_inserted() -> Result<(), Box<dyn std::error::Error>> {
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("TOKEN", "top-secret");

    let direct = "token=${TOKEN}";
    let direct_result = interpolate(InterpolationInput::new(direct, span(direct)?), &environment);
    assert_eq!(direct_result.original(), direct);
    assert_eq!(direct_result.span(), span(direct)?);
    assert_eq!(direct_result.resolved(), "token=top-secret");
    assert!(direct_result.is_sensitive());
    assert!(direct_result.substitutions()[0].is_sensitive());
    assert_eq!(
        direct_result.substitutions()[0].outcome(),
        SubstitutionOutcome::Environment
    );

    let alternative = "${TOKEN:+configured}";
    let alternative_result = interpolate(InterpolationInput::new(alternative, span(alternative)?), &environment);
    assert_eq!(alternative_result.resolved(), "configured");
    assert!(!alternative_result.is_sensitive());

    let sensitive_literal = "authored-secret";
    let literal_result = interpolate(
        InterpolationInput::new(sensitive_literal, span(sensitive_literal)?).sensitive(),
        &EmptyEnvironment,
    );
    assert!(literal_result.is_sensitive());
    Ok(())
}

#[test]
fn recovers_unsupported_and_overly_nested_expressions() -> Result<(), Box<dyn std::error::Error>> {
    let environment = environment();
    let unsupported = "${SET/foo/bar}";
    let unsupported_result = interpolate(InterpolationInput::new(unsupported, span(unsupported)?), &environment);
    assert_eq!(unsupported_result.resolved(), unsupported);
    assert_eq!(unsupported_result.diagnostics()[0].code(), INVALID_EXPRESSION);

    let unclosed = "${SET";
    let unclosed_result = interpolate(InterpolationInput::new(unclosed, span(unclosed)?), &environment);
    assert_eq!(unclosed_result.resolved(), unclosed);
    assert_eq!(unclosed_result.diagnostics()[0].code(), INVALID_EXPRESSION);

    let nested = "${MISSING:-${SECOND:-${SET}}}";
    let options = InterpolationOptions::default().with_max_nesting(1);
    let nested_result = interpolate_with_options(InterpolationInput::new(nested, span(nested)?), &environment, options);
    assert!(!nested_result.is_valid());
    assert!(
        nested_result
            .diagnostics()
            .iter()
            .any(|value| value.code() == NESTING_LIMIT)
    );
    Ok(())
}

#[test]
fn providers_are_caller_owned_and_composable_by_contract() {
    struct FixedProvider;

    impl EnvironmentProvider for FixedProvider {
        fn get(&self, name: &str) -> Option<EnvironmentValue> {
            (name == "FIXED").then(|| EnvironmentValue::plain("provided"))
        }
    }

    let value = FixedProvider.get("FIXED");
    assert_eq!(value.as_ref().map(EnvironmentValue::value), Some("provided"));
    assert_eq!(EmptyEnvironment.get("FIXED"), None);

    let mut mapped = MapEnvironment::new();
    assert!(mapped.is_empty());
    let _ = mapped.insert_value("FIXED", EnvironmentValue::plain("mapped"));
    assert_eq!(mapped.len(), 1);
}

#[test]
fn applies_a_non_destructive_overlay_to_yaml_values_only() -> Result<(), Box<dyn std::error::Error>> {
    let syntax = compose_lens::syntax::SyntaxDocument::parse(SourceId::new(67), DOCUMENT_INTERPOLATION)?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert("TAG", "1.2");
    let _ = environment.insert_sensitive("LABEL_VALUE", "confidential");
    let overlay = interpolate_document(syntax.document(), &environment);
    let originals: Vec<_> = overlay
        .values()
        .iter()
        .map(compose_lens::interpolation::InterpolationResult::original)
        .collect();

    assert!(syntax.is_valid(), "{:#?}", syntax.diagnostics());
    assert_eq!(overlay.source_id(), SourceId::new(67));
    assert!(overlay.is_valid());
    assert!(originals.contains(&"example/app:${TAG}"));
    assert!(originals.contains(&"future-${FUTURE:-enabled}"));
    assert!(originals.contains(&"${LABEL_VALUE}"));
    assert!(!originals.contains(&"echo ${SINGLE_QUOTED}"));
    assert!(!originals.contains(&"${LABEL_KEY}"));
    assert_eq!(resolved(&overlay, "example/app:${TAG}"), Some("example/app:1.2"));
    assert_eq!(resolved(&overlay, "RUNTIME=$$RUNTIME"), Some("RUNTIME=$RUNTIME"));
    assert_eq!(resolved(&overlay, "BLOCKED=${MISSING}"), Some("BLOCKED="));
    let sensitive = overlay
        .values()
        .iter()
        .find(|value| value.original() == "${LABEL_VALUE}")
        .ok_or("sensitive label value is missing")?;
    assert!(sensitive.is_sensitive());
    assert_eq!(overlay.value(sensitive.span()), Some(sensitive));
    assert_eq!(syntax.document().render_preserved(), DOCUMENT_INTERPOLATION);
    Ok(())
}

#[test]
fn loads_documents_in_order_and_retains_all_origins() -> Result<(), Box<dyn std::error::Error>> {
    let project = loaded_project()?;

    assert_eq!(project.documents().len(), 2);
    assert_eq!(project.base_directory(), Path::new("workspace/project"));
    assert_eq!(project.documents()[0].source_id(), SourceId::new(71));
    assert_eq!(project.documents()[0].origin().label(), "compose.yaml");
    assert_eq!(project.documents()[1].source_id(), SourceId::new(72));
    assert_eq!(
        project.documents()[1].origin().directory(),
        Path::new("workspace/overrides")
    );
    assert_eq!(
        project
            .document(SourceId::new(72))
            .map(|document| document.syntax().source_text()),
        Some(MULTI_FILE_OVERRIDE)
    );
    assert!(project.is_valid(), "{:#?}", project.diagnostics());
    assert!(project.diagnostics().is_empty());
    Ok(())
}

#[test]
fn interpolates_each_loaded_file_before_any_merge() -> Result<(), Box<dyn std::error::Error>> {
    let project = loaded_project()?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert("TAG", "3.2");
    let overlays = project.interpolate(&environment);

    assert_eq!(overlays.documents().len(), 2);
    assert_eq!(
        overlays
            .document(SourceId::new(71))
            .and_then(|document| resolved(document, "example/base:${TAG}")),
        Some("example/base:3.2")
    );
    assert_eq!(
        overlays
            .document(SourceId::new(72))
            .and_then(|document| resolved(document, "example/override:${TAG:-latest}")),
        Some("example/override:3.2")
    );
    assert!(overlays.is_valid(), "{:#?}", overlays.diagnostics());
    assert_eq!(project.documents()[0].syntax().render_preserved(), MULTI_FILE_BASE);
    assert_eq!(project.documents()[1].syntax().render_preserved(), MULTI_FILE_OVERRIDE);
    Ok(())
}

#[test]
fn rejects_empty_projects_and_duplicate_source_ids() {
    assert_eq!(LoadedProject::load([]), Err(ProjectLoadError::EmptyProject));

    let duplicate = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(73),
            DocumentOrigin::new("first.yaml", "project"),
            "services: {}\n",
        ),
        DocumentInput::new(
            SourceId::new(73),
            DocumentOrigin::new("second.yaml", "override"),
            "services: {}\n",
        ),
    ]);
    assert!(matches!(
        duplicate,
        Err(ProjectLoadError::DuplicateSourceId {
            source_id,
            first_origin,
            duplicate_origin,
        }) if source_id == SourceId::new(73)
            && first_origin == "first.yaml"
            && duplicate_origin == "second.yaml"
    ));
}

#[test]
fn keeps_recoverable_parse_diagnostics_attached_to_the_loaded_document() -> Result<(), Box<dyn std::error::Error>> {
    let project = LoadedProject::load([DocumentInput::new(
        SourceId::new(74),
        DocumentOrigin::new("broken.yaml", "project"),
        "services: [app\n",
    )])?;

    assert_eq!(project.documents().len(), 1);
    assert!(!project.documents()[0].syntax_diagnostics().is_empty());
    assert!(!project.documents()[0].is_valid());
    assert!(!project.is_valid());
    assert!(!project.diagnostics().is_empty());
    Ok(())
}

#[test]
fn merges_compose_fields_with_order_fidelity_and_provenance() -> Result<(), Box<dyn std::error::Error>> {
    let project = merge_fixture_project(MERGE_BASE, MERGE_OVERRIDE, 81)?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert("TAG", "3.2");
    let interpolation = project.interpolate(&environment);
    let result = merge_project(&project, Some(&interpolation));
    let merged = result.project().ok_or("merged project expected")?;

    assert!(result.is_valid(), "{:#?}", result.diagnostics());
    assert_eq!(merged.base_directory(), Path::new("workspace/project"));
    assert_eq!(merged.source_ids(), &[SourceId::new(81), SourceId::new(82)]);
    assert_eq!(
        merged_scalar(merged.value(&["services", "app", "image"])),
        Some("example/override:3.2")
    );
    assert_eq!(
        merged
            .value(&["services", "app", "image"])
            .map(|value| value.provenance().operation()),
        Some(MergeOperation::Replaced)
    );

    let command = merged
        .value(&["services", "app", "command"])
        .and_then(MergedValue::as_sequence)
        .ok_or("merged command expected")?;
    assert_eq!(command.len(), 2);
    assert_eq!(command[1].as_scalar().map(MergedScalar::value), Some("override"));
    assert_eq!(
        merged
            .value(&["services", "app", "command"])
            .map(|value| value.provenance().operation()),
        Some(MergeOperation::Replaced)
    );
    assert_eq!(
        merged_scalar(merged.value(&["services", "app", "restart"])),
        Some("unless-stopped")
    );
    assert_shell_command_merge(merged)?;

    let capabilities = merged
        .value(&["services", "app", "cap_add"])
        .and_then(MergedValue::as_sequence)
        .ok_or("merged capabilities expected")?;
    assert_eq!(capabilities.len(), 2);
    assert_eq!(
        merged
            .value(&["services", "app", "cap_add"])
            .map(|value| value.provenance().operation()),
        Some(MergeOperation::Merged)
    );

    assert_keyed_and_unique_merge(merged)?;

    assert_eq!(
        merged_scalar(merged.value(&["services", "app", "x-unknown", "nested"])),
        Some("override")
    );
    assert_eq!(
        merged_scalar(merged.value(&["services", "app", "x-unknown", "added"])),
        Some("true")
    );
    let image_sources: Vec<_> = merged
        .value(&["services", "app", "image"])
        .ok_or("image expected")?
        .provenance()
        .sources()
        .iter()
        .map(|span| span.source_id())
        .collect();
    assert_eq!(image_sources, vec![SourceId::new(81), SourceId::new(82)]);
    Ok(())
}

#[test]
fn applies_reset_and_override_tags_without_normal_merge() -> Result<(), Box<dyn std::error::Error>> {
    let project = merge_fixture_project(TAG_BASE, TAG_OVERRIDE, 91)?;
    let result = merge_project(&project, None);
    let merged = result.project().ok_or("merged project expected")?;

    assert!(result.is_valid(), "{:#?}", result.diagnostics());
    let command = merged
        .value(&["services", "app", "command"])
        .and_then(MergedValue::as_sequence)
        .ok_or("overridden command expected")?;
    assert_eq!(command.len(), 2);
    assert_eq!(command[1].as_scalar().map(MergedScalar::value), Some("override"));
    assert_eq!(
        merged
            .value(&["services", "app", "command"])
            .map(|value| value.provenance().operation()),
        Some(MergeOperation::Override)
    );

    let environment = merged
        .value(&["services", "app", "environment"])
        .ok_or("reset environment expected")?;
    assert_eq!(environment.as_mapping().map(<[MergedEntry]>::len), Some(0));
    assert_eq!(environment.provenance().operation(), MergeOperation::Reset);

    let ports = merged
        .value(&["services", "app", "ports"])
        .ok_or("reset ports expected")?;
    assert_eq!(ports.as_sequence().map(<[MergedValue]>::len), Some(0));
    assert_eq!(ports.provenance().operation(), MergeOperation::Reset);

    let volumes = merged
        .value(&["services", "app", "volumes"])
        .ok_or("overridden volumes expected")?;
    assert_eq!(volumes.as_sequence().map(<[MergedValue]>::len), Some(1));
    assert_eq!(volumes.provenance().operation(), MergeOperation::Override);
    Ok(())
}

#[test]
fn merges_cap_drop_by_exact_scalar_while_reset_and_override_keep_their_contract()
-> Result<(), Box<dyn std::error::Error>> {
    let base = concat!(
        "services:\n",
        "  normal:\n",
        "    cap_drop: [NET_ADMIN, CHOWN, NET_ADMIN]\n",
        "  reset:\n",
        "    cap_drop: [NET_ADMIN]\n",
        "  override:\n",
        "    cap_drop: [NET_ADMIN]\n",
        "  case:\n",
        "    cap_drop: [NET_ADMIN]\n",
    );
    let override_source = concat!(
        "services:\n",
        "  normal:\n",
        "    cap_drop: [CHOWN, net_admin, SYS_TIME, SYS_TIME]\n",
        "  reset:\n",
        "    cap_drop: !reset []\n",
        "  override:\n",
        "    cap_drop: !override [CHOWN, CHOWN, chown]\n",
        "  case:\n",
        "    cap_drop: [net_admin]\n",
    );
    let loaded = merge_fixture_project(base, override_source, 121)?;
    let result = merge_project(&loaded, None);
    let merged = result.project().ok_or("merged project expected")?;

    let normal = merged
        .value(&["services", "normal", "cap_drop"])
        .ok_or("merged cap_drop expected")?;
    let normal_items = normal.as_sequence().ok_or("cap_drop sequence expected")?;
    assert_eq!(
        sequence_strings(normal_items),
        ["NET_ADMIN", "CHOWN", "net_admin", "SYS_TIME"]
    );
    assert_eq!(normal.provenance().operation(), MergeOperation::Merged);
    assert_eq!(normal.provenance().sources().len(), 2);
    assert_eq!(normal_items[0].provenance().sources().len(), 2);
    assert_eq!(normal_items[1].provenance().sources().len(), 2);
    assert_eq!(normal_items[3].provenance().sources().len(), 2);

    let case = merged
        .value(&["services", "case", "cap_drop"])
        .and_then(MergedValue::as_sequence)
        .ok_or("case-sensitive cap_drop expected")?;
    assert_eq!(sequence_strings(case), ["NET_ADMIN", "net_admin"]);

    let reset = merged
        .value(&["services", "reset", "cap_drop"])
        .ok_or("reset cap_drop expected")?;
    assert!(reset.as_sequence().is_some_and(<[MergedValue]>::is_empty));
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);

    let overridden = merged
        .value(&["services", "override", "cap_drop"])
        .ok_or("overridden cap_drop expected")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert_eq!(
        sequence_strings(overridden.as_sequence().ok_or("override sequence expected")?),
        ["CHOWN", "CHOWN", "chown"]
    );
    Ok(())
}

#[test]
fn merges_cap_add_by_exact_scalar_without_rewriting_cap_drop() -> Result<(), Box<dyn std::error::Error>> {
    let base = concat!(
        "services:\n",
        "  normal:\n",
        "    cap_add: [NET_ADMIN, CHOWN, NET_ADMIN]\n",
        "    cap_drop: [MKNOD]\n",
        "  reset:\n",
        "    cap_add: [NET_ADMIN]\n",
        "  override:\n",
        "    cap_add: [NET_ADMIN]\n",
        "  case:\n",
        "    cap_add: [NET_ADMIN]\n",
    );
    let override_source = concat!(
        "services:\n",
        "  normal:\n",
        "    cap_add: [CHOWN, net_admin, SYS_TIME, SYS_TIME]\n",
        "    cap_drop: [SYS_ADMIN]\n",
        "  reset:\n",
        "    cap_add: !reset []\n",
        "  override:\n",
        "    cap_add: !override [CHOWN, CHOWN, chown]\n",
        "  case:\n",
        "    cap_add: [net_admin]\n",
    );
    let loaded = merge_fixture_project(base, override_source, 123)?;
    let result = merge_project(&loaded, None);
    let merged = result.project().ok_or("merged project expected")?;

    let normal = merged
        .value(&["services", "normal", "cap_add"])
        .ok_or("merged cap_add expected")?;
    let normal_items = normal.as_sequence().ok_or("cap_add sequence expected")?;
    assert_eq!(
        sequence_strings(normal_items),
        ["NET_ADMIN", "CHOWN", "net_admin", "SYS_TIME"]
    );
    assert_eq!(normal.provenance().operation(), MergeOperation::Merged);
    assert_eq!(normal.provenance().sources().len(), 2);
    assert_eq!(normal_items[0].provenance().sources().len(), 2);
    assert_eq!(normal_items[1].provenance().sources().len(), 2);
    assert_eq!(normal_items[3].provenance().sources().len(), 2);
    assert_eq!(
        sequence_strings(
            merged
                .value(&["services", "normal", "cap_drop"])
                .and_then(MergedValue::as_sequence)
                .ok_or("independently merged cap_drop expected")?
        ),
        ["MKNOD", "SYS_ADMIN"]
    );

    let case = merged
        .value(&["services", "case", "cap_add"])
        .and_then(MergedValue::as_sequence)
        .ok_or("case-sensitive cap_add expected")?;
    assert_eq!(sequence_strings(case), ["NET_ADMIN", "net_admin"]);

    let reset = merged
        .value(&["services", "reset", "cap_add"])
        .ok_or("reset cap_add expected")?;
    assert!(reset.as_sequence().is_some_and(<[MergedValue]>::is_empty));
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);

    let overridden = merged
        .value(&["services", "override", "cap_add"])
        .ok_or("overridden cap_add expected")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert_eq!(
        sequence_strings(overridden.as_sequence().ok_or("override sequence expected")?),
        ["CHOWN", "CHOWN", "chown"]
    );
    Ok(())
}

#[test]
fn merges_service_tmpfs_as_an_ordinary_sequence_without_cross_file_deduplication()
-> Result<(), Box<dyn std::error::Error>> {
    let base = concat!(
        "services:\n",
        "  appended:\n    tmpfs: [/base, /same]\n",
        "  scalar-scalar:\n    tmpfs: /old\n",
        "  scalar-list:\n    tmpfs: /old\n",
        "  list-scalar:\n    tmpfs: [/old]\n",
        "  reset:\n    tmpfs: [/old]\n",
        "  override:\n    tmpfs: [/old]\n",
    );
    let override_source = concat!(
        "services:\n",
        "  appended:\n    tmpfs: [/same, /later]\n",
        "  scalar-scalar:\n    tmpfs: /new\n",
        "  scalar-list:\n    tmpfs: [/new]\n",
        "  list-scalar:\n    tmpfs: /new\n",
        "  reset:\n    tmpfs: !reset []\n",
        "  override:\n    tmpfs: !override [/same, /same, /case, /CASE]\n",
    );
    let loaded = merge_fixture_project(base, override_source, 124)?;
    let result = merge_project(&loaded, None);
    let merged = result.project().ok_or("merged project expected")?;

    let appended = merged
        .value(&["services", "appended", "tmpfs"])
        .ok_or("appended tmpfs expected")?;
    assert_eq!(appended.provenance().operation(), MergeOperation::Appended);
    assert_eq!(
        sequence_strings(appended.as_sequence().ok_or("tmpfs sequence expected")?),
        ["/base", "/same", "/same", "/later"]
    );

    let scalar_scalar = merged
        .value(&["services", "scalar-scalar", "tmpfs"])
        .and_then(MergedValue::as_scalar)
        .ok_or("replacement scalar expected")?;
    assert_eq!(scalar_scalar.value(), "/new");
    assert_eq!(
        merged
            .value(&["services", "scalar-scalar", "tmpfs"])
            .ok_or("scalar expected")?
            .provenance()
            .operation(),
        MergeOperation::Replaced
    );
    assert_eq!(
        sequence_strings(
            merged
                .value(&["services", "scalar-list", "tmpfs"])
                .and_then(MergedValue::as_sequence)
                .ok_or("replacement list expected")?
        ),
        ["/new"]
    );
    assert_eq!(
        merged
            .value(&["services", "list-scalar", "tmpfs"])
            .and_then(MergedValue::as_scalar)
            .map(MergedScalar::value),
        Some("/new")
    );

    let reset = merged
        .value(&["services", "reset", "tmpfs"])
        .ok_or("reset tmpfs expected")?;
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    assert!(reset.as_sequence().is_some_and(<[MergedValue]>::is_empty));

    let overridden = merged
        .value(&["services", "override", "tmpfs"])
        .ok_or("overridden tmpfs expected")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert_eq!(
        sequence_strings(overridden.as_sequence().ok_or("override list expected")?),
        ["/same", "/same", "/case", "/CASE"]
    );
    Ok(())
}

#[test]
fn merges_service_dns_with_ordinary_append_replacement_reset_and_override() -> Result<(), Box<dyn std::error::Error>> {
    let base = concat!(
        "services:\n",
        "  appended:\n    dns: [base.example, same.example]\n",
        "  scalar:\n    dns: old.example\n",
        "  cross-form:\n    dns: old.example\n",
        "  reset:\n    dns: [old.example]\n",
        "  reset-null:\n    dns: [old.example]\n",
        "  override:\n    dns: [old.example]\n",
    );
    let override_source = concat!(
        "services:\n",
        "  appended:\n    dns: [same.example, later.example]\n",
        "  scalar:\n    dns: new.example\n",
        "  cross-form:\n    dns: [new.example]\n",
        "  reset:\n    dns: !reset []\n",
        "  reset-null:\n    dns: !reset null\n",
        "  override:\n    dns: !override [same.example, same.example]\n",
    );
    let loaded = merge_fixture_project(base, override_source, 685)?;
    assert!(
        loaded
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == DNS_EXPECTED_FORM)
    );
    let result = merge_project(&loaded, None);
    let merged = result.project().ok_or("merged project expected")?;

    let appended = merged
        .value(&["services", "appended", "dns"])
        .ok_or("appended DNS expected")?;
    assert_eq!(appended.provenance().operation(), MergeOperation::Appended);
    assert_eq!(
        sequence_strings(appended.as_sequence().ok_or("DNS sequence expected")?),
        ["base.example", "same.example", "same.example", "later.example"]
    );
    for service in ["scalar", "cross-form"] {
        assert_eq!(
            merged
                .value(&["services", service, "dns"])
                .ok_or("replacement DNS expected")?
                .provenance()
                .operation(),
            MergeOperation::Replaced
        );
    }
    assert_eq!(
        merged
            .value(&["services", "scalar", "dns"])
            .and_then(MergedValue::as_scalar)
            .map(MergedScalar::value),
        Some("new.example")
    );
    for service in ["reset", "reset-null"] {
        let reset = merged
            .value(&["services", service, "dns"])
            .ok_or("reset DNS expected")?;
        assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
        assert!(reset.as_sequence().is_some_and(<[MergedValue]>::is_empty));
    }
    let overridden = merged
        .value(&["services", "override", "dns"])
        .ok_or("override DNS expected")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert_eq!(
        sequence_strings(overridden.as_sequence().ok_or("override DNS sequence expected")?),
        ["same.example", "same.example"]
    );
    Ok(())
}

#[test]
fn merges_service_dns_search_with_append_replacement_reset_and_override() -> Result<(), Box<dyn std::error::Error>> {
    let base = concat!(
        "services:\n",
        "  appended:\n    dns_search: [base.internal, same.internal]\n",
        "  scalar:\n    dns_search: old.internal\n",
        "  cross-form:\n    dns_search: old.internal\n",
        "  reset:\n    dns_search: [old.internal]\n",
        "  reset-null:\n    dns_search: [old.internal]\n",
        "  override:\n    dns_search: [old.internal]\n",
    );
    let override_source = concat!(
        "services:\n",
        "  appended:\n    dns_search: [same.internal, later.internal]\n",
        "  scalar:\n    dns_search: new.internal\n",
        "  cross-form:\n    dns_search: [new.internal]\n",
        "  reset:\n    dns_search: !reset []\n",
        "  reset-null:\n    dns_search: !reset null\n",
        "  override:\n    dns_search: !override [same.internal, same.internal, .]\n",
    );
    let loaded = merge_fixture_project(base, override_source, 690)?;
    assert!(
        loaded
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == DNS_SEARCH_EXPECTED_FORM)
    );
    let result = merge_project(&loaded, None);
    let merged = result.project().ok_or("merged project expected")?;

    let appended = merged
        .value(&["services", "appended", "dns_search"])
        .ok_or("appended dns_search expected")?;
    assert_eq!(appended.provenance().operation(), MergeOperation::Appended);
    assert_eq!(
        sequence_strings(appended.as_sequence().ok_or("dns_search sequence expected")?),
        ["base.internal", "same.internal", "same.internal", "later.internal"]
    );
    for service in ["scalar", "cross-form"] {
        assert_eq!(
            merged
                .value(&["services", service, "dns_search"])
                .ok_or("replacement dns_search expected")?
                .provenance()
                .operation(),
            MergeOperation::Replaced
        );
    }
    assert_eq!(
        merged
            .value(&["services", "scalar", "dns_search"])
            .and_then(MergedValue::as_scalar)
            .map(MergedScalar::value),
        Some("new.internal")
    );
    for service in ["reset", "reset-null"] {
        let reset = merged
            .value(&["services", service, "dns_search"])
            .ok_or("reset dns_search expected")?;
        assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
        assert!(reset.as_sequence().is_some_and(<[MergedValue]>::is_empty));
    }
    let overridden = merged
        .value(&["services", "override", "dns_search"])
        .ok_or("overridden dns_search expected")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert_eq!(
        sequence_strings(overridden.as_sequence().ok_or("override dns_search expected")?),
        ["same.internal", "same.internal", "."]
    );
    Ok(())
}

#[test]
fn replaces_service_dns_options_as_a_whole_and_retains_reset_and_override() -> Result<(), Box<dyn std::error::Error>> {
    let base = concat!(
        "services:\n",
        "  replaced:\n    dns_opt: [ndots:2, timeout:1]\n",
        "  reset:\n    dns_opt: [rotate]\n",
        "  reset-null:\n    dns_opt: [attempts:2]\n",
        "  override:\n    dns_opt: [old]\n",
    );
    let override_source = concat!(
        "services:\n",
        "  replaced:\n    dns_opt: [timeout:3, attempts:4]\n",
        "  reset:\n    dns_opt: !reset []\n",
        "  reset-null:\n    dns_opt: !reset null\n",
        "  override:\n    dns_opt: !override [ndots:5, ndots:5]\n",
    );
    let loaded = merge_fixture_project(base, override_source, 687)?;
    let result = merge_project(&loaded, None);
    let merged = result.project().ok_or("merged project expected")?;

    let replaced = merged
        .value(&["services", "replaced", "dns_opt"])
        .ok_or("replaced dns_opt expected")?;
    assert_eq!(replaced.provenance().operation(), MergeOperation::Replaced);
    assert_eq!(replaced.provenance().sources().len(), 2);
    assert_eq!(
        sequence_strings(replaced.as_sequence().ok_or("dns_opt sequence expected")?),
        ["timeout:3", "attempts:4"]
    );
    assert!(
        replaced
            .as_sequence()
            .is_some_and(|items| items.iter().all(|item| item.provenance().sources().len() == 1))
    );

    for service in ["reset", "reset-null"] {
        let reset = merged
            .value(&["services", service, "dns_opt"])
            .ok_or("reset dns_opt expected")?;
        assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
        assert!(reset.as_sequence().is_some_and(<[MergedValue]>::is_empty));
    }
    let overridden = merged
        .value(&["services", "override", "dns_opt"])
        .ok_or("overridden dns_opt expected")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert_eq!(
        sequence_strings(overridden.as_sequence().ok_or("override dns_opt expected")?),
        ["ndots:5", "ndots:5"]
    );
    Ok(())
}

#[test]
fn merges_expose_by_exact_text_and_yaml_scalar_kind_with_reset_and_override() -> Result<(), Box<dyn std::error::Error>>
{
    use compose_lens::merge::MergedScalarKind;

    let base = concat!(
        "services:\n",
        "  merged:\n    expose: [80, \"80\", \"80/tcp\", 81]\n",
        "  reset:\n    expose: [90]\n",
        "  override:\n    expose: [100]\n",
    );
    let override_source = concat!(
        "services:\n",
        "  merged:\n    expose: [80, \"80\", \"80/tcp\", \"81\", \"82/udp\"]\n",
        "  reset:\n    expose: !reset []\n",
        "  override:\n    expose: !override [100, 100, \"100\"]\n",
    );
    let loaded = merge_fixture_project(base, override_source, 692)?;
    let merged_result = merge_project(&loaded, None);
    let merged = merged_result.project().ok_or("merged project expected")?;
    let items = merged
        .value(&["services", "merged", "expose"])
        .and_then(MergedValue::as_sequence)
        .ok_or("merged expose expected")?;
    let identities = items
        .iter()
        .map(|item| {
            let scalar = item.as_scalar().ok_or("scalar expose item expected")?;
            Ok((scalar.kind(), scalar.value(), item.provenance().sources().len()))
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    assert_eq!(
        identities,
        [
            (MergedScalarKind::Number, "80", 2),
            (MergedScalarKind::String, "80", 2),
            (MergedScalarKind::String, "80/tcp", 2),
            (MergedScalarKind::Number, "81", 1),
            (MergedScalarKind::String, "81", 1),
            (MergedScalarKind::String, "82/udp", 1),
        ]
    );
    assert_eq!(
        merged
            .value(&["services", "merged", "expose"])
            .ok_or("field expected")?
            .provenance()
            .operation(),
        MergeOperation::Merged
    );
    let reset = merged.value(&["services", "reset", "expose"]).ok_or("reset expected")?;
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    assert!(reset.as_sequence().is_some_and(<[MergedValue]>::is_empty));
    let overridden = merged
        .value(&["services", "override", "expose"])
        .ok_or("override expected")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert_eq!(overridden.as_sequence().ok_or("override sequence expected")?.len(), 3);
    Ok(())
}

#[test]
fn appends_service_security_options_and_retains_duplicates_reset_and_override() -> Result<(), Box<dyn std::error::Error>>
{
    let base = concat!(
        "services:\n",
        "  appended:\n    security_opt: [\"label:disable\", \"label:filetype:container_file_t\", \"label:level:s0:c1,c2\", \"label:nested\", \"label:type:container_t\", \"mask=/proc/acpi:/proc/kcore\", \"apparmor=base\", \"no-new-privileges:false\", \"seccomp=base.json\"]\n",
        "  reset:\n    security_opt: [\"apparmor=old\"]\n",
        "  reset-null:\n    security_opt: [\"no-new-privileges:true\"]\n",
        "  override:\n    security_opt: [old]\n",
    );
    let override_source = concat!(
        "services:\n",
        "  appended:\n    security_opt: [\"apparmor=next\", \"label:disable\", \"label:filetype:container_file_t\", \"label:filetype:container_file_t\", \"label:level:s0:c1,c2\", \"label:level:s0:c1,c2\", \"label:nested\", \"label:nested\", \"label:type:container_t\", \"label:type:container_t\", \"mask=/proc/acpi:/proc/kcore\", \"mask=/proc/acpi:/proc/kcore\", \"label=disable\", \"no-new-privileges:true\", \"no-new-privileges:false\", \"seccomp=next.json\", \"seccomp=next.json\"]\n",
        "  reset:\n    security_opt: !reset []\n",
        "  reset-null:\n    security_opt: !reset null\n",
        "  override:\n    security_opt: !override [same, same, next]\n",
    );
    let loaded = merge_fixture_project(base, override_source, 697)?;
    let result = merge_project(&loaded, None);
    let merged = result.project().ok_or("merged project expected")?;

    let appended = merged
        .value(&["services", "appended", "security_opt"])
        .ok_or("appended security_opt expected")?;
    assert_eq!(appended.provenance().operation(), MergeOperation::Appended);
    assert_eq!(appended.provenance().sources().len(), 2);
    assert_eq!(
        sequence_strings(appended.as_sequence().ok_or("security_opt sequence expected")?),
        [
            "label:disable",
            "label:filetype:container_file_t",
            "label:level:s0:c1,c2",
            "label:nested",
            "label:type:container_t",
            "mask=/proc/acpi:/proc/kcore",
            "apparmor=base",
            "no-new-privileges:false",
            "seccomp=base.json",
            "apparmor=next",
            "label:disable",
            "label:filetype:container_file_t",
            "label:filetype:container_file_t",
            "label:level:s0:c1,c2",
            "label:level:s0:c1,c2",
            "label:nested",
            "label:nested",
            "label:type:container_t",
            "label:type:container_t",
            "mask=/proc/acpi:/proc/kcore",
            "mask=/proc/acpi:/proc/kcore",
            "label=disable",
            "no-new-privileges:true",
            "no-new-privileges:false",
            "seccomp=next.json",
            "seccomp=next.json",
        ]
    );

    for service in ["reset", "reset-null"] {
        let reset = merged
            .value(&["services", service, "security_opt"])
            .ok_or("reset security_opt expected")?;
        assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
        assert!(reset.as_sequence().is_some_and(<[MergedValue]>::is_empty));
    }
    let overridden = merged
        .value(&["services", "override", "security_opt"])
        .ok_or("overridden security_opt expected")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert_eq!(
        sequence_strings(overridden.as_sequence().ok_or("override security_opt expected")?),
        ["same", "same", "next"]
    );
    Ok(())
}

#[test]
fn appends_repeatable_unmask_options_without_deduplication_or_singleton_selection()
-> Result<(), Box<dyn std::error::Error>> {
    let base = concat!(
        "services:\n",
        "  appended:\n    security_opt: [\"unmask=ALL\", \"unmask=/proc/acpi\"]\n",
        "  reset:\n    security_opt: [\"unmask=/old\"]\n",
        "  override:\n    security_opt: [\"unmask=/old\"]\n",
    );
    let override_source = concat!(
        "services:\n",
        "  appended:\n    security_opt: [\"unmask=ALL\", \"unmask=/proc/acpi:/sys/firmware\"]\n",
        "  reset:\n    security_opt: !reset []\n",
        "  override:\n    security_opt: !override [\"unmask=ALL\", \"unmask=ALL\"]\n",
    );
    let loaded = merge_fixture_project(base, override_source, 734)?;
    let result = merge_project(&loaded, None);
    let merged = result.project().ok_or("merged project expected")?;

    let appended = merged
        .value(&["services", "appended", "security_opt"])
        .ok_or("appended security_opt expected")?;
    assert_eq!(appended.provenance().operation(), MergeOperation::Appended);
    assert_eq!(
        sequence_strings(appended.as_sequence().ok_or("security_opt sequence expected")?),
        [
            "unmask=ALL",
            "unmask=/proc/acpi",
            "unmask=ALL",
            "unmask=/proc/acpi:/sys/firmware",
        ]
    );

    let reset = merged
        .value(&["services", "reset", "security_opt"])
        .ok_or("reset security_opt expected")?;
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    assert!(reset.as_sequence().is_some_and(<[MergedValue]>::is_empty));

    let overridden = merged
        .value(&["services", "override", "security_opt"])
        .ok_or("overridden security_opt expected")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert_eq!(
        sequence_strings(overridden.as_sequence().ok_or("override sequence expected")?),
        ["unmask=ALL", "unmask=ALL"]
    );
    Ok(())
}

#[test]
fn merges_service_sysctls_by_authored_form_without_deduplicating_lists() -> Result<(), Box<dyn std::error::Error>> {
    let base = concat!(
        "services:\n",
        "  mapping:\n    sysctls: {base.only: base, shared: old}\n",
        "  list:\n    sysctls: [same=value, base=value]\n",
        "  map-to-list:\n    sysctls: {old: value}\n",
        "  list-to-map:\n    sysctls: [old=value]\n",
        "  reset-map:\n    sysctls: {old: value}\n",
        "  reset-list:\n    sysctls: [old=value]\n",
        "  override:\n    sysctls: [old=value]\n",
    );
    let override_source = concat!(
        "services:\n",
        "  mapping:\n    sysctls: {shared: new, added: true}\n",
        "  list:\n    sysctls: [same=value, later=value]\n",
        "  map-to-list:\n    sysctls: [new=value]\n",
        "  list-to-map:\n    sysctls: {new: value}\n",
        "  reset-map:\n    sysctls: !reset {}\n",
        "  reset-list:\n    sysctls: !reset []\n",
        "  override:\n    sysctls: !override [same=value, same=value]\n",
    );
    let loaded = merge_fixture_project(base, override_source, 125)?;
    let result = merge_project(&loaded, None);
    let merged = result.project().ok_or("merged project expected")?;

    let mapping = merged
        .value(&["services", "mapping", "sysctls"])
        .ok_or("merged sysctls mapping expected")?;
    assert_eq!(mapping.provenance().operation(), MergeOperation::Merged);
    assert_eq!(merged_scalar(mapping.get("base.only")), Some("base"));
    assert_eq!(merged_scalar(mapping.get("shared")), Some("new"));
    assert_eq!(merged_scalar(mapping.get("added")), Some("true"));
    assert_eq!(
        mapping.get("shared").map(|value| value.provenance().sources().len()),
        Some(2)
    );

    let list = merged
        .value(&["services", "list", "sysctls"])
        .ok_or("appended sysctls list expected")?;
    assert_eq!(list.provenance().operation(), MergeOperation::Appended);
    assert_eq!(
        sequence_strings(list.as_sequence().ok_or("sysctls list expected")?),
        ["same=value", "base=value", "same=value", "later=value"]
    );

    assert_eq!(
        sequence_strings(
            merged
                .value(&["services", "map-to-list", "sysctls"])
                .and_then(MergedValue::as_sequence)
                .ok_or("replacement list expected")?
        ),
        ["new=value"]
    );
    assert_eq!(
        merged_scalar(
            merged
                .value(&["services", "list-to-map", "sysctls"])
                .and_then(|value| value.get("new"))
        ),
        Some("value")
    );

    for service in ["reset-map", "reset-list"] {
        let reset = merged
            .value(&["services", service, "sysctls"])
            .ok_or("reset sysctls expected")?;
        assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    }
    assert!(
        merged
            .value(&["services", "reset-map", "sysctls"])
            .and_then(MergedValue::as_mapping)
            .is_some_and(<[MergedEntry]>::is_empty)
    );
    assert!(
        merged
            .value(&["services", "reset-list", "sysctls"])
            .and_then(MergedValue::as_sequence)
            .is_some_and(<[MergedValue]>::is_empty)
    );
    let overridden = merged
        .value(&["services", "override", "sysctls"])
        .ok_or("overridden sysctls expected")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert_eq!(
        sequence_strings(overridden.as_sequence().ok_or("override list expected")?),
        ["same=value", "same=value"]
    );
    Ok(())
}

#[test]
fn interpolates_sysctl_values_and_list_items_but_not_mapping_keys_before_merge()
-> Result<(), Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([DocumentInput::new(
        SourceId::new(127),
        DocumentOrigin::new("compose.yaml", "workspace/project"),
        concat!(
            "services:\n",
            "  app:\n",
            "    sysctls:\n",
            "      literal.${KEY}: \"${VALUE}\"\n",
            "  list:\n",
            "    sysctls: [\"${ASSIGNMENT}\"]\n",
        ),
    )])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert("KEY", "resolved-key");
    let _ = environment.insert_sensitive("VALUE", "sensitive-value");
    let _ = environment.insert("ASSIGNMENT", "net.core.somaxconn=1024");
    let interpolation = loaded.interpolate(&environment);
    let result = merge_project(&loaded, Some(&interpolation));
    let merged = result.project().ok_or("merged project expected")?;

    let mapping = merged
        .value(&["services", "app", "sysctls"])
        .ok_or("mapped sysctls expected")?;
    assert!(mapping.get("literal.${KEY}").is_some());
    assert!(mapping.get("literal.resolved-key").is_none());
    let mapped_value = mapping.get("literal.${KEY}").ok_or("literal key expected")?;
    assert_eq!(merged_scalar(Some(mapped_value)), Some("sensitive-value"));
    assert!(mapped_value.is_sensitive());
    let list_value = merged
        .value(&["services", "list", "sysctls"])
        .and_then(MergedValue::as_sequence)
        .and_then(|values| values.first())
        .ok_or("interpolated list item expected")?;
    assert_eq!(
        list_value.as_scalar().map(MergedScalar::value),
        Some("net.core.somaxconn=1024")
    );
    assert!(!format!("{result:?}").contains("sensitive-value"));
    Ok(())
}

#[test]
fn merge_results_redact_interpolated_sensitive_values_from_debug_output() -> Result<(), Box<dyn std::error::Error>> {
    let project = LoadedProject::load([DocumentInput::new(
        SourceId::new(101),
        DocumentOrigin::new("compose.yaml", "workspace/project"),
        "services:\n  app:\n    image: example/app:${TOKEN}\n",
    )])?;
    let mut environment = MapEnvironment::new();
    let _ = environment.insert_sensitive("TOKEN", "debug-secret-value");
    let interpolation = project.interpolate(&environment);
    let result = merge_project(&project, Some(&interpolation));
    let image = result
        .project()
        .and_then(|project| project.value(&["services", "app", "image"]))
        .ok_or("merged sensitive image expected")?;

    assert!(image.is_sensitive());
    assert_eq!(
        image.as_scalar().map(MergedScalar::value),
        Some("example/app:debug-secret-value")
    );
    assert!(!format!("{result:?}").contains("debug-secret-value"));
    Ok(())
}

#[test]
fn rejects_interpolation_overlays_from_another_project() -> Result<(), Box<dyn std::error::Error>> {
    let project = LoadedProject::load([DocumentInput::new(
        SourceId::new(111),
        DocumentOrigin::new("compose.yaml", "workspace/project"),
        "services: {}\n",
    )])?;
    let other = LoadedProject::load([DocumentInput::new(
        SourceId::new(112),
        DocumentOrigin::new("other.yaml", "workspace/other"),
        "services: {}\n",
    )])?;
    let interpolation = other.interpolate(&EmptyEnvironment);
    let result = merge_project(&project, Some(&interpolation));

    assert!(!result.is_valid());
    assert_eq!(
        result
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == INTERPOLATION_PROJECT_MISMATCH)
            .count(),
        2
    );
    Ok(())
}

fn loaded_project() -> Result<LoadedProject, ProjectLoadError> {
    LoadedProject::load([
        DocumentInput::new(
            SourceId::new(71),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            MULTI_FILE_BASE,
        ),
        DocumentInput::new(
            SourceId::new(72),
            DocumentOrigin::new("compose.override.yaml", "workspace/overrides"),
            MULTI_FILE_OVERRIDE,
        ),
    ])
}

fn merge_fixture_project(
    base: &'static str,
    override_source: &'static str,
    first_id: u32,
) -> Result<LoadedProject, ProjectLoadError> {
    LoadedProject::load([
        DocumentInput::new(
            SourceId::new(first_id),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            base,
        ),
        DocumentInput::new(
            SourceId::new(first_id + 1),
            DocumentOrigin::new("compose.override.yaml", "workspace/overrides"),
            override_source,
        ),
    ])
}

fn assert_keyed_and_unique_merge(merged: &MergedProject) -> Result<(), Box<dyn std::error::Error>> {
    let environment = merged
        .value(&["services", "app", "environment"])
        .ok_or("merged environment expected")?;
    assert_eq!(merged_scalar(environment.get("MAP_ONLY")), Some("base"));
    assert_eq!(merged_scalar(environment.get("SHARED")), Some("override"));
    assert_eq!(merged_scalar(environment.get("LIST_ONLY")), Some("added"));
    assert_eq!(entry_syntax(environment, "MAP_ONLY"), Some(EntrySyntax::Mapping));
    assert_eq!(entry_syntax(environment, "SHARED"), Some(EntrySyntax::ListKeyValue));

    let labels = merged
        .value(&["services", "app", "labels"])
        .ok_or("merged labels expected")?;
    assert_eq!(merged_scalar(labels.get("list.only")), Some("base"));
    assert_eq!(merged_scalar(labels.get("shared")), Some("override"));
    assert_eq!(merged_scalar(labels.get("map.only")), Some("added"));
    assert_eq!(entry_syntax(labels, "list.only"), Some(EntrySyntax::ListKeyValue));
    assert_eq!(entry_syntax(labels, "shared"), Some(EntrySyntax::Mapping));

    let ports = merged
        .value(&["services", "app", "ports"])
        .and_then(MergedValue::as_sequence)
        .ok_or("merged ports expected")?;
    assert_eq!(ports.len(), 3);
    assert_eq!(merged_scalar(ports[0].get("mode")), Some("host"));
    assert_eq!(ports[0].provenance().operation(), MergeOperation::Replaced);

    let volumes = merged
        .value(&["services", "app", "volumes"])
        .and_then(MergedValue::as_sequence)
        .ok_or("merged volumes expected")?;
    assert_eq!(volumes.len(), 3);
    assert_eq!(merged_scalar(volumes[0].get("target")), Some("/data"));
    assert_eq!(
        merged_scalar(volumes[0].get("bind").and_then(|bind| bind.get("selinux"))),
        Some("Z")
    );
    assert_eq!(volumes[0].provenance().operation(), MergeOperation::Replaced);

    let devices = merged
        .value(&["services", "app", "devices"])
        .and_then(MergedValue::as_sequence)
        .ok_or("merged devices expected")?;
    assert_eq!(devices.len(), 2);
    assert_eq!(
        devices[0].as_scalar().map(MergedScalar::value),
        Some("/dev/override:/dev/device:rw")
    );
    assert_eq!(devices[0].provenance().operation(), MergeOperation::Replaced);

    let configs = merged
        .value(&["services", "app", "configs"])
        .and_then(MergedValue::as_sequence)
        .ok_or("merged configs expected")?;
    assert_eq!(configs.len(), 1);
    assert_eq!(merged_scalar(configs[0].get("source")), Some("override-config"));
    assert_eq!(merged_scalar(configs[0].get("mode")), Some("0444"));
    assert_eq!(merged_scalar(configs[0].get("uid")), Some("1000"));
    assert_eq!(configs[0].provenance().operation(), MergeOperation::Merged);

    let secrets = merged
        .value(&["services", "app", "secrets"])
        .and_then(MergedValue::as_sequence)
        .ok_or("merged secrets expected")?;
    assert_eq!(secrets.len(), 1);
    assert_eq!(merged_scalar(secrets[0].get("source")), Some("replacement-secret"));
    assert_eq!(secrets[0].provenance().operation(), MergeOperation::Replaced);
    Ok(())
}

fn assert_shell_command_merge(merged: &MergedProject) -> Result<(), Box<dyn std::error::Error>> {
    let entrypoint = merged
        .value(&["services", "app", "entrypoint"])
        .and_then(MergedValue::as_sequence)
        .ok_or("merged entrypoint expected")?;
    assert_eq!(entrypoint.len(), 1);
    assert_eq!(
        entrypoint[0].as_scalar().map(MergedScalar::value),
        Some("/override-entrypoint")
    );
    assert_eq!(
        merged
            .value(&["services", "app", "entrypoint"])
            .map(|value| value.provenance().operation()),
        Some(MergeOperation::Replaced)
    );

    let healthcheck = merged
        .value(&["services", "app", "healthcheck"])
        .ok_or("merged healthcheck expected")?;
    let test = healthcheck
        .get("test")
        .and_then(MergedValue::as_sequence)
        .ok_or("merged healthcheck test expected")?;
    assert_eq!(test.len(), 2);
    assert_eq!(test[1].as_scalar().map(MergedScalar::value), Some("override-health"));
    assert_eq!(merged_scalar(healthcheck.get("interval")), Some("30s"));
    assert_eq!(merged_scalar(healthcheck.get("timeout")), Some("5s"));
    assert_eq!(
        healthcheck.get("test").map(|value| value.provenance().operation()),
        Some(MergeOperation::Replaced)
    );
    Ok(())
}

#[test]
fn recursively_merges_ulimit_ranges_and_preserves_reset_override_and_shape_replacement()
-> Result<(), Box<dyn std::error::Error>> {
    let loaded = LoadedProject::load([
        DocumentInput::new(
            SourceId::new(683),
            DocumentOrigin::new("compose.yaml", "workspace/project"),
            concat!(
                "services:\n",
                "  merged:\n",
                "    ulimits: {nofile: {soft: 100, hard: 200}, core: {soft: 1, hard: 2}}\n",
                "  reset:\n",
                "    ulimits: {nofile: 1}\n",
                "  overridden:\n",
                "    ulimits: {nofile: 1}\n",
            ),
        ),
        DocumentInput::new(
            SourceId::new(684),
            DocumentOrigin::new("compose.override.yaml", "workspace/override"),
            concat!(
                "services:\n",
                "  merged:\n",
                "    ulimits: {nofile: {hard: 300}, core: -1}\n",
                "  reset:\n",
                "    ulimits: !reset {}\n",
                "  overridden:\n",
                "    ulimits: !override {nproc: 8}\n",
            ),
        ),
    ])?;
    let merged = merge_project(&loaded, None);
    let project = merged.project().ok_or("merged project expected")?;
    let nofile = project
        .value(&["services", "merged", "ulimits", "nofile"])
        .ok_or("merged nofile expected")?;
    assert_eq!(nofile.provenance().operation(), MergeOperation::Merged);
    assert_eq!(merged_scalar(nofile.get("soft")), Some("100"));
    assert_eq!(merged_scalar(nofile.get("hard")), Some("300"));
    assert_eq!(
        nofile.get("hard").map(|value| value.provenance().operation()),
        Some(MergeOperation::Replaced)
    );
    assert_eq!(
        project
            .value(&["services", "merged", "ulimits", "core"])
            .map(|value| value.provenance().operation()),
        Some(MergeOperation::Replaced)
    );
    let reset = project
        .value(&["services", "reset", "ulimits"])
        .ok_or("reset ulimits expected")?;
    assert!(reset.as_mapping().is_some_and(<[_]>::is_empty));
    assert_eq!(reset.provenance().operation(), MergeOperation::Reset);
    let overridden = project
        .value(&["services", "overridden", "ulimits"])
        .ok_or("overridden ulimits expected")?;
    assert_eq!(overridden.provenance().operation(), MergeOperation::Override);
    assert_eq!(merged_scalar(overridden.get("nproc")), Some("8"));
    assert!(overridden.get("nofile").is_none());
    Ok(())
}

fn merged_scalar(value: Option<&MergedValue>) -> Option<&str> {
    value.and_then(MergedValue::as_scalar).map(MergedScalar::value)
}

fn sequence_strings(values: &[MergedValue]) -> Vec<&str> {
    values
        .iter()
        .filter_map(MergedValue::as_scalar)
        .map(MergedScalar::value)
        .collect()
}

fn entry_syntax(value: &MergedValue, key: &str) -> Option<EntrySyntax> {
    value
        .as_mapping()?
        .iter()
        .find(|entry| entry.key() == key)
        .map(MergedEntry::syntax)
}

fn resolved<'a>(overlay: &'a compose_lens::interpolation::DocumentInterpolation, original: &str) -> Option<&'a str> {
    overlay
        .values()
        .iter()
        .find(|value| value.original() == original)
        .map(compose_lens::interpolation::InterpolationResult::resolved)
}

fn diagnostic_text(diagnostic: &Diagnostic) -> String {
    let mut text = diagnostic.message().to_owned();
    for label in diagnostic.labels() {
        text.push_str(label.message());
    }
    for note in diagnostic.notes() {
        text.push_str(note);
    }
    text
}
